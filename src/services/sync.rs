use crate::config::Config;
use crate::config::storage::sign;
use crate::db::changelog::{LOG, Record};
use crate::db::database::CONN;
use crate::db::database::tables::apply;
use crate::db::database::tables::insert;
use crate::db::database::traits::{Authorize, Database, Find, Load};
use crate::proto::services::sync::{
    FileUrl, InsertData, Mutation, MutationBatch, MutationResult, PushAck, Sync, SyncDelta,
    SyncServer, UserInsert, WatchRequest, insert_data, update_data,
};
use crate::types::error::{Conflict, Error, ForeignKeyError, Result};
use crate::types::id::Id;
use crate::types::phone::Phone;
use crate::types::role::{Action, Actions, Organisation, Permissions, Resource, Role};
use crate::types::token::Token;
use crate::types::user::{Level, Status, User};
use chrono::Utc;
use diesel::Connection;
use diesel::SqliteConnection;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use tokio::sync::mpsc;
use tokio_stream::{Stream, StreamExt};
use tonic::Streaming;
use tracing::{debug, info, warn};

/// LogTable maps proto table integers to their logical table identity.
/// These must match the InsertData/UpdateData oneof field numbers in sync.proto.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(i32)]
pub enum LogTable {
    Users = 1,
    Schools = 2,
    Owners = 3,
    Students = 4,
    Guardians = 5,
    Departments = 6,
    Teachers = 7,
    Staff = 8,
    Terms = 9,
    ClassTeachers = 10,
    Enrollments = 11,
    Subjects = 12,
    Attendance = 13,
    Timetable = 14,
    Lessons = 15,
    Exams = 16,
    Papers = 17,
    Grades = 18,
    Fees = 19,
    Invoices = 20,
    Payments = 21,
    Announcements = 22,
    Mastery = 23,
    AiUsage = 24,
    Settings = 25,
    Roles = 26,
    Scopes = 27,
    Plans = 28,
    Subscriptions = 29,
    Discounts = 30,
}

impl LogTable {
    pub fn from_i32(v: i32) -> Option<Self> {
        match v {
            1 => Some(Self::Users),
            2 => Some(Self::Schools),
            3 => Some(Self::Owners),
            4 => Some(Self::Students),
            5 => Some(Self::Guardians),
            6 => Some(Self::Departments),
            7 => Some(Self::Teachers),
            8 => Some(Self::Staff),
            9 => Some(Self::Terms),
            10 => Some(Self::ClassTeachers),
            11 => Some(Self::Enrollments),
            12 => Some(Self::Subjects),
            13 => Some(Self::Attendance),
            14 => Some(Self::Timetable),
            15 => Some(Self::Lessons),
            16 => Some(Self::Exams),
            17 => Some(Self::Papers),
            18 => Some(Self::Grades),
            19 => Some(Self::Fees),
            20 => Some(Self::Invoices),
            21 => Some(Self::Payments),
            22 => Some(Self::Announcements),
            23 => Some(Self::Mastery),
            24 => Some(Self::AiUsage),
            25 => Some(Self::Settings),
            26 => Some(Self::Roles),
            27 => Some(Self::Scopes),
            28 => Some(Self::Plans),
            29 => Some(Self::Subscriptions),
            30 => Some(Self::Discounts),
            _ => None,
        }
    }

    /// Whether this table is a member table that can trigger invitation flow.
    pub fn is_member_table(self) -> bool {
        matches!(
            self,
            Self::Owners | Self::Teachers | Self::Staff | Self::Students | Self::Guardians
        )
    }

    /// Map a table to its governing Resource for permission checks.
    pub fn resource(self) -> Option<Resource> {
        match self {
            Self::Users => Some(Resource::Users),
            Self::Schools | Self::Settings => Some(Resource::Schools),
            Self::Owners => Some(Resource::Owners),
            Self::Teachers => Some(Resource::Teachers),
            Self::Staff => Some(Resource::Staff),
            Self::Students | Self::Guardians | Self::Enrollments => Some(Resource::Students),
            Self::Departments => Some(Resource::Departments),
            Self::ClassTeachers | Self::Subjects | Self::Timetable => Some(Resource::Classes),
            Self::Attendance => Some(Resource::Attendance),
            Self::Lessons => Some(Resource::Lessons),
            Self::Exams | Self::Papers => Some(Resource::Exams),
            Self::Grades | Self::Mastery => Some(Resource::Grades),
            Self::Fees | Self::Invoices => Some(Resource::Fees),
            Self::Payments => Some(Resource::Payments),
            Self::Announcements => Some(Resource::Announcements),
            Self::Roles | Self::Scopes => Some(Resource::Roles),
            Self::Plans | Self::Subscriptions | Self::Discounts => Some(Resource::Plans),
            Self::AiUsage => Some(Resource::AI),
            Self::Terms => Some(Resource::Schools),
        }
    }

    /// Map the proto operation (0=Insert,1=Update,2=Delete) to a required Action.
    pub fn action_for_op(self, op: i32) -> Option<Action> {
        match op {
            OP_INSERT => match self {
                Self::Enrollments | Self::Scopes => Some(Action::Assign),
                Self::Attendance | Self::Grades | Self::Mastery => Some(Action::Mark),
                _ => Some(Action::Create),
            },
            OP_UPDATE => match self {
                Self::Attendance => Some(Action::Mark),
                _ => Some(Action::Update),
            },
            OP_DELETE => match self {
                Self::Enrollments | Self::Scopes => Some(Action::Unassign),
                _ => Some(Action::Delete),
            },
            _ => None,
        }
    }

    /// Extract the school_id from a row_key, if applicable.
    /// Most school-scoped tables have the school as the first "|"-delimited segment.
    /// System tables (users, plans) return None.
    pub fn school_from_key(self, row_key: &str) -> Option<Id> {
        match self {
            // System-level tables — no school scope
            Self::Users | Self::Plans => None,
            // id-PK school-scoped tables — school is in the row data, not the key
            Self::Exams | Self::Fees | Self::Invoices | Self::Announcements | Self::Payments => {
                None
            }
            // Roles and scopes can be system-scoped (school IS NULL)
            Self::Roles => None,
            Self::Scopes => {
                let parts: Vec<&str> = row_key.split('|').collect();
                if parts.len() >= 3 {
                    // parts[0] is school (could be empty string for NULL)
                    if parts[0].is_empty() {
                        None
                    } else {
                        parts[0].parse().ok()
                    }
                } else {
                    None
                }
            }
            // All other tables: school is the first segment of the composite key
            _ => {
                let first = row_key.split('|').next()?;
                first.parse().ok()
            }
        }
    }

    /// Primary key field names for this table (used for row_key validation on deletes).
    pub fn pk_field_names(self) -> &'static [&'static str] {
        match self {
            Self::Users => &["id"],
            Self::Schools => &["id"],
            Self::Owners => &["school", "user"],
            Self::Students => &["school", "adm"],
            Self::Guardians => &["school", "user", "student"],
            Self::Departments => &["school", "name"],
            Self::Teachers => &["school", "user"],
            Self::Staff => &["school", "user"],
            Self::Terms => &["school", "year", "term"],
            Self::ClassTeachers => &["school", "year", "term", "grade", "stream", "teacher"],
            Self::Enrollments => &["school", "year", "term", "grade", "stream", "student"],
            Self::Subjects => &["school", "year", "term", "grade", "stream", "subject"],
            Self::Attendance => &[
                "school", "year", "term", "grade", "stream", "student", "date",
            ],
            Self::Timetable => &[
                "school", "year", "term", "grade", "stream", "subject", "day", "start",
            ],
            Self::Lessons => &[
                "school", "year", "term", "grade", "stream", "date", "subject", "teacher",
            ],
            Self::Exams => &["id"],
            Self::Papers => &["school", "exam", "subject", "paper"],
            Self::Grades => &["school", "exam", "student", "subject", "paper"],
            Self::Fees => &["id"],
            Self::Invoices => &["id"],
            Self::Payments => &["id"],
            Self::Announcements => &["id"],
            Self::Mastery => &["school", "student", "grade", "subject", "topic"],
            Self::AiUsage => &["school", "student", "year", "term"],
            Self::Settings => &["school"],
            Self::Roles => &["id"],
            Self::Scopes => &["school", "user", "role"],
            Self::Plans => &["id"],
            Self::Subscriptions => &["school", "plan", "year", "term", "student"],
            Self::Discounts => &["school", "plan", "year", "term", "grade"],
        }
    }

    /// Returns a dependency-order index for sorting mutations.
    ///
    /// Parent tables get lower indices so they are processed before
    /// children.  This mirrors `SNAPSHOT_TABLE_ORDER` but as a
    /// per-table weight suitable for `sort_by_key`.
    ///
    /// For **deletes** the caller should negate or reverse the order
    /// so children are removed before parents.
    pub fn dependency_order(self) -> u8 {
        match self {
            Self::Users => 0,
            Self::Schools => 1,
            Self::Plans => 2,
            Self::Roles => 3,
            Self::Owners => 4,
            Self::Teachers => 5,
            Self::Staff => 6,
            Self::Students => 7,
            Self::Guardians => 8,
            Self::Departments => 9,
            Self::Scopes => 10,
            Self::Settings => 11,
            Self::Terms => 12,
            Self::Enrollments => 13,
            Self::ClassTeachers => 14,
            Self::Subjects => 15,
            Self::Attendance => 16,
            Self::Timetable => 17,
            Self::Lessons => 18,
            Self::Exams => 19,
            Self::Papers => 20,
            Self::Grades => 21,
            Self::Fees => 22,
            Self::Invoices => 23,
            Self::Payments => 24,
            Self::Announcements => 25,
            Self::Mastery => 26,
            Self::AiUsage => 27,
            Self::Subscriptions => 28,
            Self::Discounts => 29,
        }
    }
}

/// Operation constants matching the proto convention.
pub const OP_INSERT: i32 = 0;
pub const OP_UPDATE: i32 = 1;
pub const OP_DELETE: i32 = 2;

/// Result codes for MutationResult.
pub const CODE_OK: i32 = 0;
pub const CODE_PERMISSION_DENIED: i32 = 1;
pub const CODE_CONFLICT: i32 = 2;
pub const CODE_VALIDATION: i32 = 3;
pub const CODE_NOT_FOUND: i32 = 4;
pub const CODE_FK_VIOLATION: i32 = 5;
pub const CODE_DATABASE_LOCKED: i32 = 6;

/// Safety limits for PushChanges.
const MAX_MUTATIONS_PER_BATCH: usize = 500;
const MAX_BATCHES_PER_STREAM: usize = 10_000;
const MAX_ROW_DATA_SIZE: usize = 65_536; // 64KB

/// Pre-computed context for a push batch.
/// Avoids repeated DB queries for the same user's membership data.
struct PushContext {
    user: User,
    /// School IDs where the user is a member (pre-loaded for Normal users).
    schools: HashSet<Id>,
}

impl PushContext {
    fn build(conn: &mut SqliteConnection, user: User) -> PushContext {
        let schools = if user.level == Level::Normal {
            let ids: Vec<Id> = Load::<&User, Id>::load(conn, &user).unwrap_or_default();
            ids.into_iter().collect()
        } else {
            HashSet::new()
        };
        PushContext { user, schools }
    }
}

pub struct SyncService<C> {
    config: Arc<C>,
    notify: Arc<tokio::sync::Notify>,
}

impl<C: Config + Send + ::std::marker::Sync + 'static> Sync for SyncService<C> {
    type Config = Arc<C>;
    type WatchStream = std::pin::Pin<Box<dyn Stream<Item = Result<SyncDelta>> + Send>>;

    fn new(config: Self::Config) -> SyncServer<Self> {
        SyncServer::new(Self {
            config,
            notify: Arc::new(tokio::sync::Notify::new()),
        })
    }

    async fn push_changes(
        &self,
        token: Token,
        mut stream: Streaming<MutationBatch>,
    ) -> Result<mpsc::Receiver<PushAck>> {
        // Validate token is an access token
        if token.purpose != crate::types::token::Purpose::Access {
            return Err(Error::Unauthorized);
        }

        // Load the pushing user
        let user: User = CONN
            .find::<Id, User>(token.user)?
            .ok_or(Error::UserNotFound)?;

        if user.status != Status::Active && user.status != Status::Invited {
            return Err(Error::Forbidden);
        }

        info!(user_id = %user.id, level = ?user.level, "[SYNC-DEBUG] PushChanges stream opened");

        let (tx, rx) = mpsc::channel::<PushAck>(64);
        let notify = self.notify.clone();

        tokio::spawn(async move {
            let mut batch_count: usize = 0;
            let mut processed_batches: HashSet<String> = HashSet::new();

            while let Some(Ok(batch)) = stream.next().await {
                batch_count += 1;
                info!(
                    user_id = %user.id,
                    batch_id = %batch.batch_id,
                    batch_count = batch_count,
                    mutations = batch.mutations.len(),
                    "[SYNC-DEBUG] PUSH ← received batch"
                );
                for (i, m) in batch.mutations.iter().enumerate() {
                    let tbl = LogTable::from_i32(m.table)
                        .map(|t| format!("{t:?}"))
                        .unwrap_or_else(|| format!("Unknown({})", m.table));
                    let op = match m.operation {
                        0 => "Insert",
                        1 => "Update",
                        2 => "Delete",
                        o => &format!("Op({o})").leak(),
                    };
                    info!(
                        user_id = %user.id,
                        batch_id = %batch.batch_id,
                        idx = i,
                        table = %tbl,
                        op = op,
                        row_key = %m.row_key,
                        "[SYNC-DEBUG] PUSH ←   mutation[{i}]"
                    );
                }
                if batch_count > MAX_BATCHES_PER_STREAM {
                    let _ = tx
                        .send(PushAck {
                            batch_id: batch.batch_id.clone(),
                            success: false,
                            error: Some(format!(
                                "stream limit reached: {} batches (max {})",
                                batch_count, MAX_BATCHES_PER_STREAM
                            )),
                            server_seq: 0,
                            results: vec![],
                        })
                        .await;
                    break;
                }

                // Idempotency check — skip already-processed batches
                if processed_batches.contains(&batch.batch_id) {
                    debug!(batch_id = %batch.batch_id, "duplicate batch_id — already processed");
                    let _ = tx
                        .send(PushAck {
                            batch_id: batch.batch_id.clone(),
                            success: true,
                            error: Some("duplicate batch_id — already processed".into()),
                            server_seq: 0,
                            results: vec![],
                        })
                        .await;
                    continue;
                }

                let ack = CONN.with(|cell| process_batch(&mut *cell.borrow_mut(), &user, &batch));
                info!(
                    user_id = %user.id,
                    batch_id = %ack.batch_id,
                    success = ack.success,
                    mutations = ack.results.len(),
                    "[SYNC-DEBUG] PUSH → batch result"
                );
                for r in &ack.results {
                    info!(
                        user_id = %user.id,
                        batch_id = %ack.batch_id,
                        idx = r.index,
                        success = r.success,
                        code = r.code,
                        error = r.error.as_deref().unwrap_or(""),
                        "[SYNC-DEBUG] PUSH →   result[{}] code={}", r.index, r.code
                    );
                }

                processed_batches.insert(batch.batch_id.clone());

                // Cap the set size to prevent unbounded memory growth
                if processed_batches.len() > MAX_BATCHES_PER_STREAM {
                    processed_batches.clear();
                }

                // Notify waiting watch loops that new data is available
                if ack.success {
                    notify.notify_waiters();
                }

                if tx.send(ack).await.is_err() {
                    break;
                }
            }
            info!(user_id = %user.id, batches = batch_count, "[SYNC-DEBUG] PushChanges stream closed");
        });

        Ok(rx)
    }

    async fn watch_changes(
        &self,
        token: Token,
        request: WatchRequest,
    ) -> Result<Self::WatchStream> {
        if token.purpose != crate::types::token::Purpose::Access {
            return Err(Error::Unauthorized);
        }

        let user: User = CONN
            .find::<Id, User>(token.user)?
            .ok_or(Error::UserNotFound)?;

        if user.status != Status::Active && user.status != Status::Invited {
            return Err(Error::Forbidden);
        }

        info!(user_id = %user.id, level = ?user.level, last_cursor = request.last_seq, "[SYNC-DEBUG] WatchChanges stream opened");

        let (tx, rx) = mpsc::channel::<Result<SyncDelta>>(256);
        let last_cursor = request.last_seq;
        let notify = self.notify.clone();

        tokio::spawn(async move {
            let result = watch_loop(&tx, &user, last_cursor, &notify).await;
            info!(user_id = %user.id, success = result.is_ok(), "[SYNC-DEBUG] WatchChanges stream closed");
            if result.is_err() {
                // Stream ended or send failed — task exits naturally
            }
        });

        Ok(Box::pin(tokio_stream::wrappers::ReceiverStream::new(rx)))
    }
}

/// Polling interval for checking new changelog entries.
const POLL_INTERVAL: std::time::Duration = std::time::Duration::from_secs(1);

/// Pre-computed filter state for a single watch stream.
///
/// Built once when the stream opens, then consulted for every
/// changelog `Record`.  This avoids repeated database queries on each
/// polling iteration.
enum SyncFilter {
    /// Super users — no filtering, receive everything.
    Super,

    /// System users — globally scoped but role-gated.
    ///
    /// `readable` is the set of `Resource`s the user has `Read` on via
    /// their system-scoped roles.  `schools` is the set of school IDs
    /// where the user also has a membership (school-scoped data for
    /// resources *not* in `readable` are sent for these schools).
    System {
        readable: HashSet<Resource>,
        schools: HashSet<Id>,
        co_members: HashSet<Id>,
        user_id: Id,
    },

    /// Normal users — membership-based.
    ///
    /// Only see data for schools where they are a member, their own
    /// user row, co-member user rows, and all plans.
    Normal {
        schools: HashSet<Id>,
        co_members: HashSet<Id>,
        user_id: Id,
    },
}

impl SyncFilter {
    /// Build a `SyncFilter` for the given user.
    ///
    /// Loads system-scoped roles (for System users) and school
    /// memberships (for System and Normal users) from the database.
    fn build(user: &User) -> Result<Self> {
        match user.level {
            Level::Super => Ok(Self::Super),

            Level::System => {
                // Aggregate Read permissions from system-scoped roles
                let roles: Vec<Role> = CONN.load::<&User, Role>(user)?;
                let mut granted = Permissions::new();
                for role in &roles {
                    granted += role.permissions;
                }

                let mut readable = HashSet::new();
                for resource in Resource::VARIANTS {
                    if granted[resource].contains(Action::Read) {
                        readable.insert(resource);
                    }
                }

                // Also load school memberships — system users who are
                // members of specific schools get school-scoped data
                // for resources they don't have system-level Read on.
                let school_ids: Vec<Id> = CONN.load::<&User, Id>(user)?;
                let schools: HashSet<Id> = school_ids.into_iter().collect();
                let co_member_ids: Vec<Id> = CONN.load::<&HashSet<Id>, Id>(&schools)?;
                let co_members: HashSet<Id> = co_member_ids.into_iter().collect();

                Ok(Self::System {
                    readable,
                    schools,
                    co_members,
                    user_id: user.id,
                })
            }

            Level::Normal => {
                let school_ids: Vec<Id> = CONN.load::<&User, Id>(user)?;
                let schools: HashSet<Id> = school_ids.into_iter().collect();
                let co_member_ids: Vec<Id> = CONN.load::<&HashSet<Id>, Id>(&schools)?;
                let co_members: HashSet<Id> = co_member_ids.into_iter().collect();
                Ok(Self::Normal {
                    schools,
                    co_members,
                    user_id: user.id,
                })
            }
        }
    }

    /// Quick check: does this user have any access to this table type?
    ///
    /// Super: always true.
    /// System: true if the user has system-level Read on the resource,
    ///         OR if the user has any school memberships (they might see
    ///         school-scoped data).
    /// Normal: always true for Users/Plans; true for school-scoped tables
    ///         if the user has any school memberships.
    fn table_visible(&self, table: LogTable) -> bool {
        match self {
            Self::Super => true,

            Self::System {
                readable, schools, ..
            } => {
                let resource = match table.resource() {
                    Some(r) => r,
                    None => return false,
                };
                if readable.contains(&resource) {
                    return true;
                }
                // Users and Plans are always potentially visible
                if table == LogTable::Users || resource == Resource::Plans {
                    return true;
                }
                // School-scoped data visible if user has memberships
                !schools.is_empty()
            }

            Self::Normal { schools, .. } => {
                let resource = match table.resource() {
                    Some(r) => r,
                    None => return false,
                };
                if table == LogTable::Users || resource == Resource::Plans {
                    return true;
                }
                !schools.is_empty()
            }
        }
    }

    /// Row-level visibility check.
    ///
    /// Given a table, the row's primary key, and optional school_id
    /// (extracted from the row data or key), returns `true` if this
    /// row should be sent to the user.
    fn row_visible(&self, table: LogTable, row_key: &str, school_id: Option<&Id>) -> bool {
        match self {
            Self::Super => true,

            Self::System {
                readable,
                schools,
                co_members,
                user_id,
            } => {
                let resource = match table.resource() {
                    Some(r) => r,
                    None => return false,
                };

                // System-level Read on the resource → global visibility
                if readable.contains(&resource) {
                    return true;
                }

                // School-scoped: visible if the row belongs to a member school
                if let Some(sid) = school_id {
                    if schools.contains(sid) {
                        return true;
                    }
                }

                // Users table: own row + co-members
                if table == LogTable::Users {
                    if let Ok(id) = row_key.parse::<Id>() {
                        return id == *user_id || co_members.contains(&id);
                    }
                    return false;
                }

                // Plans are always visible
                if resource == Resource::Plans {
                    return true;
                }

                false
            }

            Self::Normal {
                schools,
                co_members,
                user_id,
            } => {
                let resource = match table.resource() {
                    Some(r) => r,
                    None => return false,
                };

                // Users table: own row + co-members
                if table == LogTable::Users {
                    if let Ok(id) = row_key.parse::<Id>() {
                        return id == *user_id || co_members.contains(&id);
                    }
                    return false;
                }

                // Plans always visible
                if resource == Resource::Plans {
                    return true;
                }

                // School-scoped: visible if user is a member
                if let Some(sid) = school_id {
                    return schools.contains(sid);
                }

                // System-scoped entries (school_id IS NULL) that aren't
                // the user's own row or plans — not visible to normal
                // users (e.g. system roles, other users).
                false
            }
        }
    }
}

/// Dependency-ordered table list for initial sync snapshots.
///
/// Tables are ordered so that parent entities are sent before children,
/// allowing the client to insert rows without FK violations.
const SNAPSHOT_TABLE_ORDER: &[i32] = &[
    1, 2, 28, 26, // users, schools, plans, roles
    3, 7, 8, // owners, teachers, staff
    4, 5, 6, // students, guardians, departments
    27, 25, // scopes, settings
    9,  // terms
    11, 10, 12, // enrollments, class_teachers, subjects
    13, 14, 15, // attendance, timetable, lessons
    16, 17, 18, // exams, papers, grades
    19, 20, 21, // fees, invoices, payments
    22, 23, 24, // announcements, mastery, aiusage
    29, 30, // subscriptions, discounts
];

/// Returns the current byte length of the binary changelog (the cursor
/// representing "everything is synced").
fn changelog_cursor() -> u64 {
    LOG.with(|cell| cell.borrow().len().unwrap_or(0))
}

/// Send a full snapshot of all visible rows across every table.
///
/// Used for initial sync when `last_cursor == 0`.  Each row is converted to
/// a `SyncDelta` with `seq = 0` and `operation = OP_INSERT`, filtered
/// through the user's `SyncFilter` before sending.
async fn send_full_snapshot(
    tx: &mpsc::Sender<Result<SyncDelta>>,
    filter: &SyncFilter,
) -> std::result::Result<(), ()> {
    use crate::db::database::tables::snapshot::snapshot_table;

    info!("[SYNC-DEBUG] WATCH → starting full snapshot");
    let mut rows_sent: usize = 0;

    for &table_num in SNAPSHOT_TABLE_ORDER {
        let table_enum = match LogTable::from_i32(table_num) {
            Some(t) => t,
            None => continue,
        };

        if !filter.table_visible(table_enum) {
            continue;
        }

        let rows = CONN.with(|cell| snapshot_table(&mut *cell.borrow_mut(), table_num));

        let rows = match rows {
            Ok(r) => r,
            Err(e) => {
                warn!(table = table_num, error = %e, "[SYNC-DEBUG] WATCH → snapshot query failed, skipping table");
                continue;
            }
        };

        let visible_count = rows
            .iter()
            .filter(|r| filter.row_visible(table_enum, &r.row_key, r.school_id.as_ref()))
            .count();
        if !rows.is_empty() {
            info!(
                table = ?table_enum,
                total = rows.len(),
                visible = visible_count,
                "[SYNC-DEBUG] WATCH → snapshot table {:?}: {} total, {} visible", table_enum, rows.len(), visible_count
            );
        }

        for row in &rows {
            if !filter.row_visible(table_enum, &row.row_key, row.school_id.as_ref()) {
                continue;
            }

            let file_urls =
                file_urls_for_delta(table_enum, OP_INSERT, &row.row_key, Some(&row.insert_data));

            let delta = SyncDelta {
                seq: 0,
                table: table_num,
                operation: OP_INSERT,
                row_key: row.row_key.clone(),
                data: Some(row.insert_data.clone()),
                file_urls,
            };

            if tx.send(Ok(delta)).await.is_err() {
                return Err(());
            }

            rows_sent += 1;
            // Yield periodically to avoid starving other tasks
            if rows_sent % 200 == 0 {
                tokio::task::yield_now().await;
            }
        }
    }

    info!(
        rows = rows_sent,
        "[SYNC-DEBUG] WATCH → full snapshot complete, {} rows sent", rows_sent
    );
    Ok(())
}

/// Long-running loop that streams `SyncDelta` messages to a client.
///
/// 1. Builds a `SyncFilter` based on the user's level, roles and memberships.
/// 2. If `last_cursor == 0`, performs a full snapshot of all visible data.
/// 3. Reads changelog records, groups by table, fetches changed rows via
///    timestamp-based queries, and sends filtered `SyncDelta`s.
/// 4. Reads delete records from the sidecar file and sends `OP_DELETE` deltas.
/// 5. Sleeps for `POLL_INTERVAL`, then repeats from the latest cursor.
///
/// Returns `Err` when the receiver is dropped (client disconnected) or
/// when a database error occurs.  The spawned task should treat any
/// error as a signal to stop.
async fn watch_loop(
    tx: &mpsc::Sender<Result<SyncDelta>>,
    user: &User,
    last_cursor: i64,
    notify: &tokio::sync::Notify,
) -> std::result::Result<(), ()> {
    use crate::db::database::tables::snapshot::snapshot_table_since;

    // Build the filter at stream open.
    let mut filter = match SyncFilter::build(user) {
        Ok(f) => f,
        Err(e) => {
            let _ = tx.send(Err(e)).await;
            return Err(());
        }
    };

    // Treat last_cursor as a byte-offset cursor into the binary changelog.
    let mut cursor: u64 = if last_cursor <= 0 {
        0
    } else {
        last_cursor as u64
    };

    // Track the delete-file cursor separately.  On initial connect we
    // start from the current end of the deletes file (the full snapshot
    // already captured all live rows, so historical deletes are moot).
    let mut delete_cursor: u64 = LOG.with(|cell| cell.borrow().delete_cursor().unwrap_or(0));

    // --- Initial sync: if cursor == 0, dump all tables ---
    if cursor == 0 {
        info!(user_id = %user.id, "[SYNC-DEBUG] WATCH → cold start (cursor=0), sending full snapshot");
        if send_full_snapshot(tx, &filter).await.is_err() {
            return Err(());
        }
        // After snapshot, set cursor to the current changelog length so we
        // don't re-send entries that were created while we were snapshotting.
        cursor = LOG.with(|cell| cell.borrow().len().unwrap_or(0));
        delete_cursor = LOG.with(|cell| cell.borrow().delete_cursor().unwrap_or(0));
        info!(user_id = %user.id, cursor = cursor, delete_cursor = delete_cursor, "[SYNC-DEBUG] WATCH → snapshot done, cursor={}, delete_cursor={}", cursor, delete_cursor);
    }

    // --- Incremental sync loop ---
    loop {
        let records = match LOG.with(|cell| cell.borrow().read_from(cursor)) {
            Ok(r) => r,
            Err(e) => {
                tracing::error!("changelog read failed: {e}");
                let _ = tx.send(Err(Error::Internal)).await;
                return Err(());
            }
        };

        let (delete_records, new_delete_cursor) =
            match LOG.with(|cell| cell.borrow().read_deletes_from(delete_cursor)) {
                Ok(r) => r,
                Err(e) => {
                    tracing::error!("deletes read failed: {e}");
                    let _ = tx.send(Err(Error::Internal)).await;
                    return Err(());
                }
            };

        if records.is_empty() && delete_records.is_empty() {
            tokio::select! {
                _ = notify.notified() => {
                    info!(user_id = %user.id, "[SYNC-DEBUG] WATCH → woke up via notify");
                },
                _ = tokio::time::sleep(POLL_INTERVAL) => {},
            }
            continue;
        }

        info!(
            user_id = %user.id,
            changelog_records = records.len(),
            delete_records = delete_records.len(),
            cursor = cursor,
            "[SYNC-DEBUG] WATCH → incremental: {} changelog records, {} delete records from cursor={}",
            records.len(), delete_records.len(), cursor
        );

        cursor += (records.len() as u64) * 24;
        delete_cursor = new_delete_cursor;

        let mut needs_rebuild = false;

        // Collect changed tables with minimum timestamps (Insert/Update only).
        let mut table_min_ts: HashMap<u8, i64> = HashMap::new();

        for record in &records {
            // Detect membership/scope changes that require a filter rebuild
            if should_rebuild_filter_record(record, user) {
                needs_rebuild = true;
            }

            // Skip deletes here — they are handled via the delete sidecar
            if record.op == OP_DELETE as u8 {
                continue;
            }

            let table_enum = match LogTable::from_i32(record.table as i32) {
                Some(t) => t,
                None => continue,
            };

            if !filter.table_visible(table_enum) {
                continue;
            }

            let entry = table_min_ts.entry(record.table).or_insert(record.created);
            if record.created < *entry {
                *entry = record.created;
            }
        }

        // Fetch changed rows from real tables and send as upsert deltas
        for (&table_num, &min_ts) in &table_min_ts {
            let tbl_dbg = LogTable::from_i32(table_num as i32)
                .map(|t| format!("{t:?}"))
                .unwrap_or_else(|| format!("Unknown({table_num})"));
            info!(user_id = %user.id, table = %tbl_dbg, min_ts = min_ts, "[SYNC-DEBUG] WATCH → fetching changed rows for {:?} since ts={}", tbl_dbg, min_ts);
            let table_enum = match LogTable::from_i32(table_num as i32) {
                Some(t) => t,
                None => continue,
            };

            let rows = CONN.with(|cell| {
                snapshot_table_since(&mut *cell.borrow_mut(), table_num as i32, min_ts)
            });

            let rows = match rows {
                Ok(r) => r,
                Err(e) => {
                    warn!(table = table_num, error = %e, "snapshot_since query failed, skipping");
                    continue;
                }
            };

            for row in &rows {
                if !filter.row_visible(table_enum, &row.row_key, row.school_id.as_ref()) {
                    continue;
                }

                let file_urls = file_urls_for_delta(
                    table_enum,
                    OP_INSERT,
                    &row.row_key,
                    Some(&row.insert_data),
                );

                let delta = SyncDelta {
                    seq: cursor as i64,
                    table: table_num as i32,
                    operation: OP_INSERT, // upsert semantics — client applies last-write-wins
                    row_key: row.row_key.clone(),
                    data: Some(row.insert_data.clone()),
                    file_urls,
                };

                info!(
                    user_id = %user.id,
                    table = table_num,
                    row_key = %row.row_key,
                    seq = cursor,
                    "[SYNC-DEBUG] WATCH → sending upsert delta table={} key={}",
                    table_num, row.row_key
                );

                if tx.send(Ok(delta)).await.is_err() {
                    return Err(());
                }
            }
        }

        // Process deletes from the sidecar file
        for delete in &delete_records {
            let table_enum = match LogTable::from_i32(delete.table as i32) {
                Some(t) => t,
                None => continue,
            };

            if !filter.table_visible(table_enum) {
                continue;
            }

            // For deletes we don't have school_id — but we can extract
            // it from the row_key for tables that embed it.
            let school_id = table_enum.school_from_key(&delete.key);
            if !filter.row_visible(table_enum, &delete.key, school_id.as_ref()) {
                continue;
            }

            let delta = SyncDelta {
                seq: cursor as i64,
                table: delete.table as i32,
                operation: OP_DELETE,
                row_key: delete.key.clone(),
                data: None,
                file_urls: vec![],
            };

            let tbl_dbg = LogTable::from_i32(delete.table as i32)
                .map(|t| format!("{t:?}"))
                .unwrap_or_else(|| format!("Unknown({})", delete.table));
            info!(
                user_id = %user.id,
                table = %tbl_dbg,
                row_key = %delete.key,
                "[SYNC-DEBUG] WATCH → sending delete delta table={} key={}",
                tbl_dbg, delete.key
            );

            if tx.send(Ok(delta)).await.is_err() {
                return Err(());
            }
        }

        // Rebuild filter if membership/scope tables changed
        if needs_rebuild {
            info!(user_id = %user.id, "[SYNC-DEBUG] WATCH → rebuilding sync filter (membership/scope change detected)");
            if let Ok(f) = SyncFilter::build(user) {
                filter = f;
            }
        }
    }
}

/// Conservative check: returns `true` if the record's table is a
/// membership or scope table that *could* affect the given user's
/// filter.  Without row data in the record we cannot tell whether
/// the change is about *this* user, so we rebuild on any change to
/// these tables.  Rebuilds are cheap (a few SQL queries).
fn should_rebuild_filter_record(record: &Record, user: &User) -> bool {
    let table = match LogTable::from_i32(record.table as i32) {
        Some(t) => t,
        None => return false,
    };

    match table {
        LogTable::Owners
        | LogTable::Teachers
        | LogTable::Staff
        | LogTable::Guardians
        | LogTable::Students => true,
        LogTable::Scopes if user.level == Level::System => true,
        _ => false,
    }
}

/// Process a single MutationBatch and return a PushAck.
fn process_batch(conn: &mut SqliteConnection, user: &User, batch: &MutationBatch) -> PushAck {
    info!(
        user_id = %user.id,
        batch_id = %batch.batch_id,
        mutations = batch.mutations.len(),
        "[SYNC-DEBUG] process_batch start"
    );
    if batch.mutations.len() > MAX_MUTATIONS_PER_BATCH {
        return PushAck {
            batch_id: batch.batch_id.clone(),
            success: false,
            error: Some(format!(
                "batch too large: {} mutations (max {})",
                batch.mutations.len(),
                MAX_MUTATIONS_PER_BATCH
            )),
            server_seq: 0,
            results: vec![],
        };
    }

    // Build a dependency-sorted processing order.
    //
    // Inserts/updates are sorted parent-first so FK parents exist before
    // children.  Deletes are sorted child-first (reverse dependency order)
    // so children are removed before their parents.
    //
    // We keep the original index so we can place results back in the
    // caller's expected order.
    let mut order: Vec<usize> = (0..batch.mutations.len()).collect();
    order.sort_by_key(|&i| {
        let m = &batch.mutations[i];
        let dep = LogTable::from_i32(m.table)
            .map(|t| t.dependency_order())
            .unwrap_or(255);
        if m.operation == OP_DELETE {
            // Reverse order for deletes: higher dependency_order first
            (m.operation, std::cmp::Reverse(dep), i)
        } else {
            (m.operation, std::cmp::Reverse(255 - dep), i)
        }
    });

    let mut results_by_index: HashMap<usize, MutationResult> =
        HashMap::with_capacity(batch.mutations.len());
    let mut last_cursor: u64 = 0;
    let mut all_success = true;

    // Build push context with pre-loaded membership data
    let ctx = PushContext::build(conn, user.clone());

    // Pre-scan: build a map of user IDs created in this batch (for invitation flow).
    // Key: row_key of user Insert, Value: original index in batch.
    let mut user_inserts: HashMap<String, usize> = HashMap::new();
    for (i, mutation) in batch.mutations.iter().enumerate() {
        if mutation.table == LogTable::Users as i32 && mutation.operation == OP_INSERT {
            user_inserts.insert(mutation.row_key.clone(), i);
        }
    }

    // Track which user mutations have been processed (for invitation conflict handling).
    // Maps batch index → MutationResult (pre-filled when processed out of order).
    let mut processed: HashMap<usize, MutationResult> = HashMap::new();

    for &orig_idx in &order {
        // Skip if already processed (invitation flow may process user inserts early)
        if processed.contains_key(&orig_idx) {
            let result = processed.remove(&orig_idx).unwrap();
            info!(
                user_id = %user.id,
                batch_id = %batch.batch_id,
                orig_idx = orig_idx,
                success = result.success,
                code = result.code,
                "[SYNC-DEBUG] process_batch: mutation[{orig_idx}] already processed (invitation flow), success={}, code={}",
                result.success, result.code
            );
            if !result.success {
                all_success = false;
            }
            results_by_index.insert(orig_idx, result);
            continue;
        }

        let mutation = &batch.mutations[orig_idx];
        let tbl = LogTable::from_i32(mutation.table)
            .map(|t| format!("{t:?}"))
            .unwrap_or_else(|| format!("Unknown({})", mutation.table));
        let op = match mutation.operation {
            0 => "Insert",
            1 => "Update",
            2 => "Delete",
            _ => "?",
        };
        info!(
            user_id = %user.id,
            batch_id = %batch.batch_id,
            orig_idx = orig_idx,
            table = %tbl,
            op = op,
            row_key = %mutation.row_key,
            "[SYNC-DEBUG] process_batch: processing mutation[{orig_idx}] {op} on {tbl} key={}",
            mutation.row_key
        );
        let result = process_mutation(
            conn,
            &ctx,
            mutation,
            orig_idx as i32,
            &user_inserts,
            &batch.mutations,
            &mut processed,
            &mut last_cursor,
        );

        info!(
            user_id = %user.id,
            batch_id = %batch.batch_id,
            orig_idx = orig_idx,
            success = result.success,
            code = result.code,
            error = result.error.as_deref().unwrap_or(""),
            "[SYNC-DEBUG] process_batch: mutation[{orig_idx}] result: success={}, code={}, error={:?}",
            result.success, result.code, result.error
        );
        if !result.success {
            all_success = false;
        }
        results_by_index.insert(orig_idx, result);
    }

    // Reassemble results in original batch order.
    let results: Vec<MutationResult> = (0..batch.mutations.len())
        .map(|i| {
            results_by_index.remove(&i).unwrap_or(MutationResult {
                index: i as i32,
                success: false,
                error: Some("internal error: missing result".into()),
                code: CODE_VALIDATION,
                file_urls: vec![],
            })
        })
        .collect();

    info!(
        user_id = %user.id,
        batch_id = %batch.batch_id,
        success = all_success,
        server_seq = last_cursor,
        total = results.len(),
        "[SYNC-DEBUG] process_batch done: success={}, server_seq={}, {} results",
        all_success, last_cursor, results.len()
    );

    PushAck {
        batch_id: batch.batch_id.clone(),
        success: all_success,
        error: None,
        server_seq: last_cursor as i64,
        results,
    }
}

/// Process a single mutation within a batch.
fn process_mutation(
    conn: &mut SqliteConnection,
    ctx: &PushContext,
    mutation: &Mutation,
    index: i32,
    user_inserts: &HashMap<String, usize>,
    all_mutations: &[Mutation],
    processed: &mut HashMap<usize, MutationResult>,
    last_cursor: &mut u64,
) -> MutationResult {
    // Parse the table
    let table = match LogTable::from_i32(mutation.table) {
        Some(t) => t,
        None => {
            return MutationResult {
                index,
                success: false,
                error: Some("unknown table".into()),
                code: CODE_VALIDATION,
                file_urls: vec![],
            };
        }
    };

    // Parse the action
    let action = match table.action_for_op(mutation.operation) {
        Some(a) => a,
        None => {
            return MutationResult {
                index,
                success: false,
                error: Some("invalid operation".into()),
                code: CODE_VALIDATION,
                file_urls: vec![],
            };
        }
    };

    // Check row data size limit
    let data_size = mutation
        .insert
        .as_ref()
        .map(prost::Message::encoded_len)
        .or_else(|| mutation.update.as_ref().map(prost::Message::encoded_len))
        .unwrap_or(0);
    if data_size > MAX_ROW_DATA_SIZE {
        return MutationResult {
            index,
            success: false,
            error: Some(format!(
                "row data too large: {data_size} bytes (max {MAX_ROW_DATA_SIZE})"
            )),
            code: CODE_VALIDATION,
            file_urls: vec![],
        };
    }

    // Validate that the oneof variant in InsertData/UpdateData matches the declared table number.
    // A mismatch means the client constructed the Mutation incorrectly — the server would dispatch
    // on the oneof and execute the wrong SQL, leading to confusing FK or validation errors.
    if mutation.operation == OP_INSERT {
        if let Some(data) = mutation.insert.as_ref() {
            if !apply::validate_insert(mutation.table, data) {
                let actual_variant = insert_oneof_name(data);
                tracing::error!(
                    declared_table = mutation.table,
                    actual_oneof = %actual_variant,
                    row_key = %mutation.row_key,
                    "ONEOF MISMATCH: mutation.table={} but InsertData.row is {actual_variant}",
                    mutation.table
                );
                return MutationResult {
                    index,
                    success: false,
                    error: Some(format!(
                        "insert data mismatch: table={} but row data is {actual_variant}",
                        mutation.table
                    )),
                    code: CODE_VALIDATION,
                    file_urls: vec![],
                };
            }
        }
    }
    if mutation.operation == OP_UPDATE {
        if let Some(data) = mutation.update.as_ref() {
            if !apply::validate_update(mutation.table, data) {
                let actual_variant = update_oneof_name(data);
                tracing::error!(
                    declared_table = mutation.table,
                    actual_oneof = %actual_variant,
                    row_key = %mutation.row_key,
                    "ONEOF MISMATCH: mutation.table={} but UpdateData.row is {actual_variant}",
                    mutation.table
                );
                return MutationResult {
                    index,
                    success: false,
                    error: Some(format!(
                        "update data mismatch: table={} but row data is {actual_variant}",
                        mutation.table
                    )),
                    code: CODE_VALIDATION,
                    file_urls: vec![],
                };
            }
        }
    }

    // Special handling for user Inserts (invitation validation)
    if table == LogTable::Users && mutation.operation == OP_INSERT {
        return match conn.transaction(|conn| {
            Ok(process_user_insert(
                conn,
                &ctx.user,
                mutation,
                index,
                last_cursor,
            ))
        }) {
            Ok(result) => result,
            Err(e) => {
                let e: diesel::result::Error = e;
                let err: Error = e.into();
                let (msg, code) = error_to_mutation_code(&err);
                tracing::error!("user insert transaction error: {err}");
                MutationResult {
                    index,
                    success: false,
                    error: Some(msg.into()),
                    code,
                    file_urls: vec![],
                }
            }
        };
    }

    // Handle member table inserts with invitation flow
    if table.is_member_table() && mutation.operation == OP_INSERT {
        return match conn.transaction(|conn| {
            Ok(process_member_insert(
                conn,
                ctx,
                mutation,
                index,
                table,
                user_inserts,
                all_mutations,
                processed,
                last_cursor,
            ))
        }) {
            Ok(result) => result,
            Err(e) => {
                let e: diesel::result::Error = e;
                let err: Error = e.into();
                let (msg, code) = error_to_mutation_code(&err);
                tracing::error!("member insert transaction error: {err}");
                MutationResult {
                    index,
                    success: false,
                    error: Some(msg.into()),
                    code,
                    file_urls: vec![],
                }
            }
        };
    }

    // Permission check
    if let Err(result) = check_permission(
        conn,
        ctx,
        table,
        action,
        &mutation.row_key,
        mutation.insert.as_ref(),
        index,
    ) {
        return result;
    }

    // Apply mutation to database and log it — wrapped in a savepoint
    match conn.transaction(|conn| {
        Ok(apply_and_log(
            conn,
            &ctx.user,
            mutation,
            index,
            table,
            last_cursor,
        ))
    }) {
        Ok(result) => result,
        Err(e) => {
            let e: diesel::result::Error = e;
            let err: Error = e.into();
            let (msg, code) = error_to_mutation_code(&err);
            tracing::error!("apply transaction error: {err}");
            MutationResult {
                index,
                success: false,
                error: Some(msg.into()),
                code,
                file_urls: vec![],
            }
        }
    }
}

/// Map an `Error` to a `(message, code)` pair for `MutationResult`.
fn error_to_mutation_code(err: &Error) -> (&'static str, i32) {
    match err {
        Error::Conflict => ("record already exists", CODE_CONFLICT),
        Error::ForeignKey => ("referenced record does not exist", CODE_FK_VIOLATION),
        Error::DatabaseLocked => ("database is busy, try again", CODE_DATABASE_LOCKED),
        _ => ("internal error", CODE_VALIDATION),
    }
}

/// Validate and process a user Insert mutation.
fn process_user_insert(
    conn: &mut SqliteConnection,
    user: &User,
    mutation: &Mutation,
    index: i32,
    last_cursor: &mut u64,
) -> MutationResult {
    // Extract user insert data for validation
    let user_row = match mutation.insert.as_ref().and_then(|d| d.row.as_ref()) {
        Some(insert_data::Row::User(r)) => r,
        _ => {
            return MutationResult {
                index,
                success: false,
                error: Some("expected user insert data".into()),
                code: CODE_VALIDATION,
                file_urls: vec![],
            };
        }
    };

    // Parse status, level, phone, and name from the insert data
    let status = match Status::try_from(user_row.status) {
        Ok(s) => s,
        Err(_) => {
            return MutationResult {
                index,
                success: false,
                error: Some("invalid status".into()),
                code: CODE_VALIDATION,
                file_urls: vec![],
            };
        }
    };
    let level = match Level::try_from(user_row.level) {
        Ok(l) => l,
        Err(_) => {
            return MutationResult {
                index,
                success: false,
                error: Some("invalid level".into()),
                code: CODE_VALIDATION,
                file_urls: vec![],
            };
        }
    };
    let phone_str = &user_row.phone;
    let name = &user_row.name;

    // Rule: ALL user Inserts MUST have status = Invited
    if status != Status::Invited {
        return MutationResult {
            index,
            success: false,
            error: Some("user insert must have status Invited".into()),
            code: CODE_VALIDATION,
            file_urls: vec![],
        };
    }

    // Rule: level checks
    match level {
        Level::Normal => {
            // Anyone can create normal invited users (as side effect of member creation)
        }
        Level::System => {
            // Require Users.Create permission
            let mut required = Permissions::new();
            required[Resource::Users] = Actions::from(Action::Create);
            if user.level == Level::Super {
                // Super bypasses
            } else {
                let result = Authorize::authorize(
                    conn,
                    crate::types::token::Token {
                        user: user.id,
                        phone: user.phone,
                        purpose: crate::types::token::Purpose::Access,
                        created: chrono::Utc::now(),
                        expiry: chrono::Utc::now(),
                    },
                    Organisation::System,
                    required,
                );
                if result.is_err() {
                    return MutationResult {
                        index,
                        success: false,
                        error: Some("permission denied: cannot create system users".into()),
                        code: CODE_PERMISSION_DENIED,
                        file_urls: vec![],
                    };
                }
            }
        }
        Level::Super => {
            // Only super users can create super users
            if user.level != Level::Super {
                return MutationResult {
                    index,
                    success: false,
                    error: Some("only super users can create super users".into()),
                    code: CODE_PERMISSION_DENIED,
                    file_urls: vec![],
                };
            }
        }
    }

    // Validate id from row_key
    let id: Id = match mutation.row_key.parse() {
        Ok(id) => id,
        Err(_) => {
            return MutationResult {
                index,
                success: false,
                error: Some("invalid user id".into()),
                code: CODE_VALIDATION,
                file_urls: vec![],
            };
        }
    };

    // Validate and normalize phone
    let phone: Phone = match phone_str.parse() {
        Ok(p) => p,
        Err(_) => {
            return MutationResult {
                index,
                success: false,
                error: Some("invalid phone number".into()),
                code: CODE_VALIDATION,
                file_urls: vec![],
            };
        }
    };

    // Build a server-controlled UserInsert and apply directly
    // Only phone + name come from the client; everything else is server-enforced.
    let user_insert = UserInsert {
        id: id.to_string(),
        phone: String::from(phone),
        email: None,
        name: name.clone(),
        level: i32::from(level),
        status: i32::from(Status::Invited),
    };
    let apply_result = insert::insert_user(conn, &user_insert);

    match apply_result {
        Err(e) => {
            let is_conflict = e.conflict();
            MutationResult {
                index,
                success: false,
                error: Some(if is_conflict {
                    "user already exists".into()
                } else {
                    "validation error".into()
                }),
                code: if is_conflict {
                    CODE_CONFLICT
                } else {
                    CODE_VALIDATION
                },
                file_urls: vec![],
            }
        }
        Ok(()) => {
            // Append to binary changelog
            let record = Record::new(user.id, LogTable::Users as u8, OP_INSERT as u8, 0);
            match LOG.with(|cell| cell.borrow_mut().append(&record)) {
                Ok(cursor) => {
                    *last_cursor = cursor;
                    MutationResult {
                        index,
                        success: true,
                        error: None,
                        code: CODE_OK,
                        file_urls: vec![],
                    }
                }
                Err(e) => {
                    tracing::error!("changelog append failed: {e}");
                    MutationResult {
                        index,
                        success: false,
                        error: Some("internal error".into()),
                        code: CODE_VALIDATION,
                        file_urls: vec![],
                    }
                }
            }
        }
    }
}

/// Process a member table Insert, handling invitation flow.
fn process_member_insert(
    conn: &mut SqliteConnection,
    ctx: &PushContext,
    mutation: &Mutation,
    index: i32,
    table: LogTable,
    user_inserts: &HashMap<String, usize>,
    all_mutations: &[Mutation],
    processed: &mut HashMap<usize, MutationResult>,
    last_cursor: &mut u64,
) -> MutationResult {
    let user = &ctx.user;
    // Extract the member's user field from the row_key.
    // For member tables the `user` field is typically the second segment of the composite key.
    let member_user_id = extract_member_user_id(table, &mutation.row_key);

    // Check if this member references a user created in the same batch (invitation flow)
    let is_invitation = member_user_id
        .as_deref()
        .map(|uid| user_inserts.contains_key(uid))
        .unwrap_or(false);

    if is_invitation {
        let user_key = member_user_id.as_deref().unwrap();
        let user_batch_idx = user_inserts[user_key];
        let user_mutation = &all_mutations[user_batch_idx];

        // Process the user insert (validates + applies with server-controlled defaults)
        let user_result = process_user_insert(
            conn,
            user,
            user_mutation,
            user_batch_idx as i32,
            last_cursor,
        );

        if user_result.success {
            // User inserted successfully — proceed with member insert
            processed.insert(user_batch_idx, user_result);

            // Permission check for the member table
            let action = match table.action_for_op(mutation.operation) {
                Some(a) => a,
                None => {
                    return MutationResult {
                        index,
                        success: false,
                        error: Some("invalid operation".into()),
                        code: CODE_VALIDATION,
                        file_urls: vec![],
                    };
                }
            };
            if let Err(result) = check_permission(
                conn,
                ctx,
                table,
                action,
                &mutation.row_key,
                mutation.insert.as_ref(),
                index,
            ) {
                return result;
            }

            return apply_and_log(conn, user, mutation, index, table, last_cursor);
        }

        // User insert failed — check if it was a phone conflict (invitation conflict resolution)
        if user_result.code != CODE_CONFLICT {
            // Non-conflict failure — fail the member too
            processed.insert(user_batch_idx, user_result);
            return MutationResult {
                index,
                success: false,
                error: Some("associated user insert failed".into()),
                code: CODE_VALIDATION,
                file_urls: vec![],
            };
        }

        // Look up existing user by phone from the user mutation's insert data
        let existing_user = match user_mutation.insert.as_ref().and_then(|d| d.row.as_ref()) {
            Some(insert_data::Row::User(user_row)) => {
                let phone: std::result::Result<Phone, _> = user_row.phone.parse();
                match phone {
                    Ok(p) => Find::<Phone, User>::find(conn, p).ok().flatten(),
                    Err(_) => None,
                }
            }
            _ => None,
        };

        match existing_user {
            Some(existing) => {
                // If existing user is Deleted, reject
                if existing.status == Status::Deleted {
                    processed.insert(
                        user_batch_idx,
                        MutationResult {
                            index: user_batch_idx as i32,
                            success: false,
                            error: Some("referenced user is deleted".into()),
                            code: CODE_NOT_FOUND,
                            file_urls: vec![],
                        },
                    );
                    return MutationResult {
                        index,
                        success: false,
                        error: Some("referenced user is deleted".into()),
                        code: CODE_NOT_FOUND,
                        file_urls: vec![],
                    };
                }

                // Mark the user mutation as conflict (code 2)
                warn!(
                    orphan_id = %user_mutation.row_key,
                    existing_id = %existing.id,
                    phone = %existing.phone,
                    "invitation conflict resolved — rewriting member to existing user"
                );
                processed.insert(
                    user_batch_idx,
                    MutationResult {
                        index: user_batch_idx as i32,
                        success: false,
                        error: Some("user already exists — conflict resolved".into()),
                        code: CODE_CONFLICT,
                        file_urls: vec![],
                    },
                );

                // Log a Delete for the orphaned user ID so all clients clean up
                let delete_record = Record::new(user.id, LogTable::Users as u8, OP_DELETE as u8, 0);
                if let Ok(cursor) = LOG.with(|cell| cell.borrow_mut().append(&delete_record)) {
                    *last_cursor = cursor;
                    // Also append to deletes sidecar with the orphaned row_key
                    if let Err(e) = LOG.with(|cell| {
                        cell.borrow_mut()
                            .append_delete(LogTable::Users as u8, &user_mutation.row_key)
                    }) {
                        tracing::error!("deletes append failed for orphan: {e}");
                    }
                }

                // Permission check for the member table
                let action = match table.action_for_op(mutation.operation) {
                    Some(a) => a,
                    None => {
                        return MutationResult {
                            index,
                            success: false,
                            error: Some("invalid operation".into()),
                            code: CODE_VALIDATION,
                            file_urls: vec![],
                        };
                    }
                };
                if let Err(result) = check_permission(
                    conn,
                    ctx,
                    table,
                    action,
                    &mutation.row_key,
                    mutation.insert.as_ref(),
                    index,
                ) {
                    return result;
                }

                // Rewrite the member's user field to the existing user's ID and insert
                let corrected_key =
                    rewrite_member_user_key(table, &mutation.row_key, &existing.id.to_string());

                // Build corrected InsertData with the existing user's ID
                let corrected_data = match mutation.insert.as_ref() {
                    Some(data) => rewrite_member_user(data, &existing.id.to_string()),
                    None => None,
                };

                // Apply member insert with corrected user ID to real table
                let member_apply = match corrected_data.as_ref() {
                    Some(data) => apply::apply_insert(conn, table as i32, &corrected_key, data),
                    None => {
                        tracing::error!("process_member_insert: missing insert data for member");
                        Err(Error::Internal)
                    }
                };
                if let Err(e) = member_apply {
                    return MutationResult {
                        index,
                        success: false,
                        error: Some(
                            if e.conflict() {
                                "member already exists"
                            } else {
                                "validation error"
                            }
                            .into(),
                        ),
                        code: if e.conflict() {
                            CODE_CONFLICT
                        } else {
                            CODE_VALIDATION
                        },
                        file_urls: vec![],
                    };
                }

                // Log the member insert with corrected key
                let member_record = Record::new(user.id, table as u8, OP_INSERT as u8, 0);
                match LOG.with(|cell| cell.borrow_mut().append(&member_record)) {
                    Ok(cursor) => {
                        *last_cursor = cursor;
                        MutationResult {
                            index,
                            success: true,
                            error: None,
                            code: CODE_OK,
                            file_urls: vec![],
                        }
                    }
                    Err(e) => {
                        tracing::error!("changelog append failed: {e}");
                        MutationResult {
                            index,
                            success: false,
                            error: Some("failed to log member insert".into()),
                            code: CODE_VALIDATION,
                            file_urls: vec![],
                        }
                    }
                }
            }
            None => {
                // Could not find existing user — unexpected, treat as error
                processed.insert(
                    user_batch_idx,
                    MutationResult {
                        index: user_batch_idx as i32,
                        success: false,
                        error: Some("phone conflict but existing user not found".into()),
                        code: CODE_VALIDATION,
                        file_urls: vec![],
                    },
                );
                MutationResult {
                    index,
                    success: false,
                    error: Some("invitation conflict resolution failed".into()),
                    code: CODE_VALIDATION,
                    file_urls: vec![],
                }
            }
        }
    } else {
        // Not an invitation — normal member insert
        let action = match table.action_for_op(mutation.operation) {
            Some(a) => a,
            None => {
                return MutationResult {
                    index,
                    success: false,
                    error: Some("invalid operation".into()),
                    code: CODE_VALIDATION,
                    file_urls: vec![],
                };
            }
        };
        if let Err(result) = check_permission(
            conn,
            ctx,
            table,
            action,
            &mutation.row_key,
            mutation.insert.as_ref(),
            index,
        ) {
            return result;
        }
        apply_and_log(conn, &ctx.user, mutation, index, table, last_cursor)
    }
}

/// Check if the user has permission for the given resource+action.
/// Returns Ok(()) if permitted, Err(MutationResult) if denied.
fn check_permission(
    conn: &mut SqliteConnection,
    ctx: &PushContext,
    table: LogTable,
    action: Action,
    row_key: &str,
    insert_data: Option<&InsertData>,
    index: i32,
) -> std::result::Result<(), MutationResult> {
    let user = &ctx.user;

    // Super users bypass all checks
    if user.level == Level::Super {
        return Ok(());
    }

    let resource = match table.resource() {
        Some(r) => r,
        None => {
            return Err(MutationResult {
                index,
                success: false,
                error: Some("unknown resource for table".into()),
                code: CODE_VALIDATION,
                file_urls: vec![],
            });
        }
    };

    // Determine organisation context — try row_key first, then fall back to insert data
    let organisation = match table
        .school_from_key(row_key)
        .or_else(|| school_from_insert(table, insert_data))
    {
        Some(school_id) => Organisation::School(school_id),
        None => Organisation::System,
    };

    match user.level {
        Level::Super => Ok(()), // already handled above, but exhaustive match
        Level::System => {
            // System users: use full Authorize trait (role-based)
            let mut required = Permissions::new();
            required[resource] = Actions::from(action);

            let auth_token = crate::types::token::Token {
                user: user.id,
                phone: user.phone,
                purpose: crate::types::token::Purpose::Access,
                created: chrono::Utc::now(),
                expiry: chrono::Utc::now() + chrono::Duration::hours(1),
            };

            let org_debug = format!("{:?}", organisation);
            match Authorize::authorize(conn, auth_token, organisation, required) {
                Ok(()) => Ok(()),
                Err(_) => {
                    warn!(
                        user_id = %user.id,
                        resource = ?resource,
                        action = ?action,
                        org = %org_debug,
                        "system user permission denied on push"
                    );
                    Err(MutationResult {
                        index,
                        success: false,
                        error: Some("permission denied".into()),
                        code: CODE_PERMISSION_DENIED,
                        file_urls: vec![],
                    })
                }
            }
        }
        Level::Normal => {
            match organisation {
                Organisation::System => {
                    // Normal users cannot push to system-scoped tables.
                    // Exception: user Inserts (invitation) are handled before
                    // check_permission is called.
                    warn!(
                        user_id = %user.id,
                        resource = ?resource,
                        "normal user denied push to system context"
                    );
                    Err(MutationResult {
                        index,
                        success: false,
                        error: Some("permission denied".into()),
                        code: CODE_PERMISSION_DENIED,
                        file_urls: vec![],
                    })
                }
                Organisation::Account => {
                    // Own account operations — allowed
                    Ok(())
                }
                Organisation::School(school_id) => {
                    // Normal users: membership = full school access (fine-grained deferred)
                    if ctx.schools.contains(&school_id) {
                        Ok(())
                    } else {
                        warn!(
                            user_id = %user.id,
                            school_id = %school_id,
                            "normal user denied push to non-member school"
                        );
                        Err(MutationResult {
                            index,
                            success: false,
                            error: Some("permission denied".into()),
                            code: CODE_PERMISSION_DENIED,
                            file_urls: vec![],
                        })
                    }
                }
            }
        }
    }
}

fn school_from_insert(table: LogTable, data: Option<&InsertData>) -> Option<Id> {
    let data = data?;
    match &data.row {
        Some(insert_data::Row::Exam(r)) => r.school.parse().ok(),
        Some(insert_data::Row::Fee(r)) => r.school.parse().ok(),
        Some(insert_data::Row::Invoice(r)) => r.school.parse().ok(),
        Some(insert_data::Row::Announcement(r)) => r.school.parse().ok(),
        Some(insert_data::Row::Payment(r)) => r.school.as_ref()?.parse().ok(),
        Some(insert_data::Row::Role(r)) => r.school.as_ref()?.parse().ok(),
        _ => None,
    }
}

/// Generate presigned PUT URLs for file-bearing mutations (used in PushAck).
fn file_urls_for_mutation(
    table: LogTable,
    op: i32,
    row_key: &str,
    insert_data: Option<&InsertData>,
) -> Vec<FileUrl> {
    if op == OP_DELETE {
        return vec![];
    }

    match table {
        LogTable::Users => {
            let user_id = row_key;
            let path = format!("users/{}/profile", user_id);
            let put_url = sign::url(&path, sign::TTL, true);
            vec![FileUrl {
                path,
                put_url: Some(put_url),
                get_url: None,
                expiry: Utc::now().timestamp() + sign::TTL as i64,
            }]
        }
        LogTable::Schools => {
            let school_id = row_key;
            let path = format!("schools/{}/logo", school_id);
            let put_url = sign::url(&path, sign::TTL, true);
            vec![FileUrl {
                path,
                put_url: Some(put_url),
                get_url: None,
                expiry: Utc::now().timestamp() + sign::TTL as i64,
            }]
        }
        LogTable::Students => {
            if let Some(data) = insert_data {
                if let Some(insert_data::Row::Student(s)) = &data.row {
                    if s.documents.is_some() {
                        let parts: Vec<&str> = row_key.split('|').collect();
                        if parts.len() >= 2 {
                            let path = format!("students/{}/{}/documents", parts[0], parts[1]);
                            let put_url = sign::url(&path, sign::TTL, true);
                            return vec![FileUrl {
                                path,
                                put_url: Some(put_url),
                                get_url: None,
                                expiry: Utc::now().timestamp() + sign::TTL as i64,
                            }];
                        }
                    }
                }
            }
            vec![]
        }
        _ => vec![],
    }
}

/// Generate presigned GET URLs for file-bearing records (used in SyncDelta).
fn file_urls_for_delta(
    table: LogTable,
    op: i32,
    row_key: &str,
    data: Option<&InsertData>,
) -> Vec<FileUrl> {
    if op == OP_DELETE {
        return vec![];
    }

    match table {
        LogTable::Users => {
            let path = format!("users/{}/profile", row_key);
            let get_url = sign::url(&path, sign::TTL, false);
            vec![FileUrl {
                path,
                put_url: None,
                get_url: Some(get_url),
                expiry: Utc::now().timestamp() + sign::TTL as i64,
            }]
        }
        LogTable::Schools => {
            let path = format!("schools/{}/logo", row_key);
            let get_url = sign::url(&path, sign::TTL, false);
            vec![FileUrl {
                path,
                put_url: None,
                get_url: Some(get_url),
                expiry: Utc::now().timestamp() + sign::TTL as i64,
            }]
        }
        LogTable::Students => {
            if let Some(data) = data {
                if let Some(insert_data::Row::Student(s)) = &data.row {
                    if s.documents.is_some() {
                        let parts: Vec<&str> = row_key.split('|').collect();
                        if parts.len() >= 2 {
                            let path = format!("students/{}/{}/documents", parts[0], parts[1]);
                            let get_url = sign::url(&path, sign::TTL, false);
                            return vec![FileUrl {
                                path,
                                put_url: None,
                                get_url: Some(get_url),
                                expiry: Utc::now().timestamp() + sign::TTL as i64,
                            }];
                        }
                    }
                }
            }
            vec![]
        }
        _ => vec![],
    }
}

/// Apply a mutation to the database and append a record to the binary
/// changelog.  Returns a `MutationResult`.
fn apply_and_log(
    conn: &mut SqliteConnection,
    user: &User,
    mutation: &Mutation,
    index: i32,
    table: LogTable,
    last_cursor: &mut u64,
) -> MutationResult {
    let op = match mutation.operation {
        0 => "Insert",
        1 => "Update",
        2 => "Delete",
        _ => "?",
    };
    let oneof_info = if mutation.operation == OP_INSERT {
        mutation
            .insert
            .as_ref()
            .map(|d| insert_oneof_name(d))
            .unwrap_or("None")
    } else if mutation.operation == OP_UPDATE {
        mutation
            .update
            .as_ref()
            .map(|d| update_oneof_name(d))
            .unwrap_or("None")
    } else {
        "N/A"
    };
    info!(
        user_id = %user.id,
        table = ?table,
        op = op,
        row_key = %mutation.row_key,
        oneof = %oneof_info,
        "[SYNC-DEBUG] apply_and_log: {op} on {table:?} key={} oneof={oneof_info}",
        mutation.row_key
    );
    // The columns bitmask for the changelog record.
    // With typed updates, individual fields are Option — the bitmask is no longer
    // sent by the client. We write 0; the changelog consumer relies on the typed
    // update data rather than the bitmask.
    let columns_mask: u16 = 0;

    // 1. Apply to real table using typed apply
    let apply_result = apply::apply_mutation(
        conn,
        mutation.table,
        mutation.operation,
        &mutation.row_key,
        mutation.insert.as_ref(),
        mutation.update.as_ref(),
    );

    match apply_result {
        Err(e) => {
            let is_conflict = e.conflict();
            let is_fk = e.foreign_key();
            let is_locked = matches!(e, Error::DatabaseLocked);
            info!(
                table = ?table,
                row_key = %mutation.row_key,
                op = mutation.operation,
                conflict = is_conflict,
                fk = is_fk,
                locked = is_locked,
                error = %e,
                "[SYNC-DEBUG] apply_and_log FAILED: table={table:?} key={} op={} error={e} (conflict={is_conflict}, fk={is_fk}, locked={is_locked})",
                mutation.row_key, mutation.operation
            );
            let (msg, code) = if is_conflict {
                ("record already exists", CODE_CONFLICT)
            } else if is_fk {
                ("referenced record does not exist", CODE_FK_VIOLATION)
            } else if is_locked {
                ("database is busy, try again", CODE_DATABASE_LOCKED)
            } else {
                ("validation error", CODE_VALIDATION)
            };
            return MutationResult {
                index,
                success: false,
                error: Some(msg.into()),
                code,
                file_urls: vec![],
            };
        }
        Ok(()) => {
            info!(
                table = ?table,
                row_key = %mutation.row_key,
                "[SYNC-DEBUG] apply_and_log: DB write OK for {table:?} key={}",
                mutation.row_key
            );
        }
    }

    // 2. Append to binary changelog
    let record = Record::new(user.id, table as u8, mutation.operation as u8, columns_mask);

    let file_urls = file_urls_for_mutation(
        table,
        mutation.operation,
        &mutation.row_key,
        mutation.insert.as_ref(),
    );

    match LOG.with(|cell| cell.borrow_mut().append(&record)) {
        Ok(cursor) => {
            *last_cursor = cursor;

            // For deletes, also append to the deletes sidecar so the
            // watch loop can stream row_key-bearing delete deltas.
            if mutation.operation == OP_DELETE {
                if let Err(e) = LOG.with(|cell| {
                    cell.borrow_mut()
                        .append_delete(table as u8, &mutation.row_key)
                }) {
                    tracing::error!("deletes append failed: {e}");
                }
            }

            MutationResult {
                index,
                success: true,
                error: None,
                code: CODE_OK,
                file_urls,
            }
        }
        Err(e) => {
            tracing::error!("changelog append failed: {e}");
            MutationResult {
                index,
                success: false,
                error: Some("internal error".into()),
                code: CODE_VALIDATION,
                file_urls: vec![],
            }
        }
    }
}

/// Extract the user ID from a member table's row_key.
/// Member table composite keys typically have school as first, user as second segment.
fn extract_member_user_id(table: LogTable, row_key: &str) -> Option<String> {
    let parts: Vec<&str> = row_key.split('|').collect();
    match table {
        // owners(school, user) → index 1
        LogTable::Owners => parts.get(1).map(|s| s.to_string()),
        // teachers(school, user) → index 1
        LogTable::Teachers => parts.get(1).map(|s| s.to_string()),
        // staff(school, user) → index 1
        LogTable::Staff => parts.get(1).map(|s| s.to_string()),
        // students have adm not user as second key — user is optional field, not PK
        // For students, the invitation flow references optional user field in row data
        LogTable::Students => None,
        // guardians(school, user, student) → index 1
        LogTable::Guardians => parts.get(1).map(|s| s.to_string()),
        _ => None,
    }
}

/// Clone an InsertData and rewrite the `user` field in a member table's insert message.
fn rewrite_member_user(data: &InsertData, new_user: &str) -> Option<InsertData> {
    let mut cloned = data.clone();
    match &mut cloned.row {
        Some(insert_data::Row::Owner(r)) => r.user = new_user.to_string(),
        Some(insert_data::Row::Teacher(r)) => r.user = new_user.to_string(),
        Some(insert_data::Row::StaffMember(r)) => r.user = new_user.to_string(),
        Some(insert_data::Row::Guardian(r)) => r.user = new_user.to_string(),
        // Students don't have a required user field (it's optional)
        Some(insert_data::Row::Student(r)) => r.user = Some(new_user.to_string()),
        _ => return None,
    }
    Some(cloned)
}

/// Rewrite the user segment in a member table's composite row_key.
/// Returns a human-readable name for the InsertData oneof variant.
fn insert_oneof_name(data: &InsertData) -> &'static str {
    match &data.row {
        Some(insert_data::Row::User(_)) => "User",
        Some(insert_data::Row::School(_)) => "School",
        Some(insert_data::Row::Owner(_)) => "Owner",
        Some(insert_data::Row::Student(_)) => "Student",
        Some(insert_data::Row::Guardian(_)) => "Guardian",
        Some(insert_data::Row::Department(_)) => "Department",
        Some(insert_data::Row::Teacher(_)) => "Teacher",
        Some(insert_data::Row::StaffMember(_)) => "StaffMember",
        Some(insert_data::Row::Term(_)) => "Term",
        Some(insert_data::Row::ClassTeacher(_)) => "ClassTeacher",
        Some(insert_data::Row::Enrollment(_)) => "Enrollment",
        Some(insert_data::Row::Subject(_)) => "Subject",
        Some(insert_data::Row::Attendance(_)) => "Attendance",
        Some(insert_data::Row::Timetable(_)) => "Timetable",
        Some(insert_data::Row::Lesson(_)) => "Lesson",
        Some(insert_data::Row::Exam(_)) => "Exam",
        Some(insert_data::Row::Paper(_)) => "Paper",
        Some(insert_data::Row::Grade(_)) => "Grade",
        Some(insert_data::Row::Fee(_)) => "Fee",
        Some(insert_data::Row::Invoice(_)) => "Invoice",
        Some(insert_data::Row::Payment(_)) => "Payment",
        Some(insert_data::Row::Announcement(_)) => "Announcement",
        Some(insert_data::Row::Mastery(_)) => "Mastery",
        Some(insert_data::Row::AiUsage(_)) => "AiUsage",
        Some(insert_data::Row::Settings(_)) => "Settings",
        Some(insert_data::Row::Role(_)) => "Role",
        Some(insert_data::Row::Scope(_)) => "Scope",
        Some(insert_data::Row::Plan(_)) => "Plan",
        Some(insert_data::Row::Subscription(_)) => "Subscription",
        Some(insert_data::Row::Discount(_)) => "Discount",
        None => "None",
    }
}

/// Returns a human-readable name for the UpdateData oneof variant.
fn update_oneof_name(data: &crate::proto::services::sync::UpdateData) -> &'static str {
    match &data.row {
        Some(update_data::Row::User(_)) => "User",
        Some(update_data::Row::School(_)) => "School",
        Some(update_data::Row::Student(_)) => "Student",
        Some(update_data::Row::Guardian(_)) => "Guardian",
        Some(update_data::Row::Department(_)) => "Department",
        Some(update_data::Row::Teacher(_)) => "Teacher",
        Some(update_data::Row::StaffMember(_)) => "StaffMember",
        Some(update_data::Row::Term(_)) => "Term",
        Some(update_data::Row::ClassTeacher(_)) => "ClassTeacher",
        Some(update_data::Row::Attendance(_)) => "Attendance",
        Some(update_data::Row::Timetable(_)) => "Timetable",
        Some(update_data::Row::Exam(_)) => "Exam",
        Some(update_data::Row::Paper(_)) => "Paper",
        Some(update_data::Row::Grade(_)) => "Grade",
        Some(update_data::Row::Fee(_)) => "Fee",
        Some(update_data::Row::Invoice(_)) => "Invoice",
        Some(update_data::Row::Payment(_)) => "Payment",
        Some(update_data::Row::Announcement(_)) => "Announcement",
        Some(update_data::Row::Mastery(_)) => "Mastery",
        Some(update_data::Row::AiUsage(_)) => "AiUsage",
        Some(update_data::Row::Settings(_)) => "Settings",
        Some(update_data::Row::Role(_)) => "Role",
        Some(update_data::Row::Plan(_)) => "Plan",
        Some(update_data::Row::Subscription(_)) => "Subscription",
        Some(update_data::Row::Discount(_)) => "Discount",
        None => "None",
    }
}

fn rewrite_member_user_key(table: LogTable, row_key: &str, new_user_id: &str) -> String {
    let mut parts: Vec<&str> = row_key.split('|').collect();
    match table {
        LogTable::Owners | LogTable::Teachers | LogTable::Staff | LogTable::Guardians => {
            if parts.len() >= 2 {
                parts[1] = new_user_id;
            }
        }
        _ => {}
    }
    parts.join("|")
}

#[cfg(test)]
mod tests {
    use super::*;

    // --- action_for_op tests ---

    #[test]
    fn action_for_op_enrollments_insert() {
        assert_eq!(
            LogTable::Enrollments.action_for_op(OP_INSERT),
            Some(Action::Assign)
        );
    }

    #[test]
    fn action_for_op_enrollments_delete() {
        assert_eq!(
            LogTable::Enrollments.action_for_op(OP_DELETE),
            Some(Action::Unassign)
        );
    }

    #[test]
    fn action_for_op_scopes_insert() {
        assert_eq!(
            LogTable::Scopes.action_for_op(OP_INSERT),
            Some(Action::Assign)
        );
    }

    #[test]
    fn action_for_op_scopes_delete() {
        assert_eq!(
            LogTable::Scopes.action_for_op(OP_DELETE),
            Some(Action::Unassign)
        );
    }

    #[test]
    fn action_for_op_attendance_insert() {
        assert_eq!(
            LogTable::Attendance.action_for_op(OP_INSERT),
            Some(Action::Mark)
        );
    }

    #[test]
    fn action_for_op_attendance_update() {
        assert_eq!(
            LogTable::Attendance.action_for_op(OP_UPDATE),
            Some(Action::Mark)
        );
    }

    #[test]
    fn action_for_op_schools_insert() {
        assert_eq!(
            LogTable::Schools.action_for_op(OP_INSERT),
            Some(Action::Create)
        );
    }

    #[test]
    fn action_for_op_schools_update() {
        assert_eq!(
            LogTable::Schools.action_for_op(OP_UPDATE),
            Some(Action::Update)
        );
    }

    #[test]
    fn action_for_op_schools_delete() {
        assert_eq!(
            LogTable::Schools.action_for_op(OP_DELETE),
            Some(Action::Delete)
        );
    }

    #[test]
    fn action_for_op_invalid_op() {
        assert_eq!(LogTable::Schools.action_for_op(99), None);
    }

    // --- school_from_key Scopes fix tests ---

    #[test]
    fn scopes_school_from_key_with_school() {
        let key = "bbbbbbbbbbbbbbbbbbbbbbbb|cccccccccccccccccccccccc|dddddddddddddddddddddddd";
        let result = LogTable::Scopes.school_from_key(key);
        assert!(result.is_some());
        assert_eq!(result.unwrap().to_string(), "bbbbbbbbbbbbbbbbbbbbbbbb");
    }

    #[test]
    fn scopes_school_from_key_system_scope_empty_school() {
        // System-scoped scope: school is empty string (representing NULL)
        let key = "|cccccccccccccccccccccccc|dddddddddddddddddddddddd";
        let result = LogTable::Scopes.school_from_key(key);
        assert!(result.is_none());
    }

    #[test]
    fn scopes_school_from_key_too_few_parts() {
        let key = "bbbbbbbbbbbbbbbbbbbbbbbb|cccccccccccccccccccccccc";
        let result = LogTable::Scopes.school_from_key(key);
        assert!(result.is_none());
    }

    #[test]
    fn scopes_school_from_key_empty_string() {
        let result = LogTable::Scopes.school_from_key("");
        assert!(result.is_none());
    }

    // --- pk_field_names tests ---

    #[test]
    fn pk_field_names_users() {
        assert_eq!(LogTable::Users.pk_field_names(), &["id"]);
    }

    #[test]
    fn pk_field_names_owners() {
        assert_eq!(LogTable::Owners.pk_field_names(), &["school", "user"]);
    }

    #[test]
    fn pk_field_names_students() {
        assert_eq!(LogTable::Students.pk_field_names(), &["school", "adm"]);
    }

    #[test]
    fn pk_field_names_attendance() {
        assert_eq!(
            LogTable::Attendance.pk_field_names(),
            &[
                "school", "year", "term", "grade", "stream", "student", "date"
            ]
        );
    }

    #[test]
    fn pk_field_names_exams() {
        assert_eq!(LogTable::Exams.pk_field_names(), &["id"]);
    }

    #[test]
    fn pk_field_names_subscriptions() {
        assert_eq!(
            LogTable::Subscriptions.pk_field_names(),
            &["school", "plan", "year", "term", "student"]
        );
    }

    // =========================================================================
    // Integration tests — require an in-memory SQLite database with migrations
    // =========================================================================

    use crate::db::changelog::LOG;
    use crate::db::database::MIGRATIONS;
    use crate::db::database::traits::{Create, Find};
    use crate::proto::services::sync::*;
    use diesel::{Connection, RunQueryDsl};
    use diesel_migrations::MigrationHarness;

    /// Create a fresh in-memory SQLite connection with all migrations applied.
    fn test_conn() -> SqliteConnection {
        let mut conn = SqliteConnection::establish(":memory:").unwrap();
        conn.run_pending_migrations(MIGRATIONS).unwrap();
        diesel::sql_query(
            "PRAGMA foreign_keys = ON; PRAGMA journal_mode = WAL; PRAGMA synchronous = NORMAL; PRAGMA busy_timeout = 5000;",
        )
        .execute(&mut conn)
        .unwrap();
        conn
    }

    /// Create a User struct with the given level and status, ready for DB insertion.
    fn make_user(level: Level, status: Status) -> User {
        User {
            id: Id::default(),
            phone: "0759762268".parse().unwrap(),
            email: None,
            name: "Test User".into(),
            level,
            status,
            created: chrono::Utc::now().timestamp(),
            updated: chrono::Utc::now().timestamp(),
        }
    }

    /// Create a User with a specific phone number (for conflict tests).
    fn make_user_with_phone(level: Level, status: Status, phone: &str) -> User {
        User {
            id: Id::default(),
            phone: phone.parse().unwrap(),
            email: None,
            name: "Test User".into(),
            level,
            status,
            created: chrono::Utc::now().timestamp(),
            updated: chrono::Utc::now().timestamp(),
        }
    }

    fn user_insert_data(id: &Id, phone: &str, name: &str, level: i32, status: i32) -> InsertData {
        InsertData {
            row: Some(insert_data::Row::User(UserInsert {
                id: id.to_string(),
                phone: phone.into(),
                email: None,
                name: name.into(),
                level,
                status,
            })),
        }
    }

    fn school_insert_data(id: &Id) -> InsertData {
        InsertData {
            row: Some(insert_data::Row::School(SchoolInsert {
                id: id.to_string(),
                name: "Test School".into(),
                motto: None,
                phone: None,
                email: None,
                county: 1,
                domain: None,
                established: None,
                status: 1,
            })),
        }
    }

    fn owner_insert_data(school: &Id, user: &Id) -> InsertData {
        InsertData {
            row: Some(insert_data::Row::Owner(OwnerInsert {
                school: school.to_string(),
                user: user.to_string(),
            })),
        }
    }

    fn teacher_insert_data(school: &Id, user: &Id) -> InsertData {
        InsertData {
            row: Some(insert_data::Row::Teacher(TeacherInsert {
                school: school.to_string(),
                user: user.to_string(),
                hired: None,
                role: None,
                department: None,
                status: 1,
            })),
        }
    }

    // --- Push integration tests ---

    #[test]
    fn test_push_insert_school() {
        let mut conn = test_conn();

        // Create a super user in DB so process_batch can work
        let user = make_user(Level::Super, Status::Active);
        Create::<User>::create(&mut conn, user.clone()).unwrap();

        let school_id = Id::default();
        let batch = MutationBatch {
            batch_id: "batch-1".into(),
            mutations: vec![Mutation {
                table: LogTable::Schools as i32,
                operation: OP_INSERT,
                row_key: school_id.to_string(),
                insert: Some(school_insert_data(&school_id)),
                update: None,
            }],
        };

        let ack = process_batch(&mut conn, &user, &batch);
        assert!(ack.success, "push should succeed: {:?}", ack);
        assert_eq!(ack.results.len(), 1);
        assert_eq!(ack.results[0].code, CODE_OK);

        // Verify the changelog has a record for the school insert
        let records = LOG.with(|cell| cell.borrow().read_from(0).unwrap());
        assert!(
            records
                .iter()
                .any(|r| r.table == LogTable::Schools as u8 && r.op == OP_INSERT as u8),
            "changelog should contain the school insert"
        );
    }

    #[test]
    fn test_push_insert_user_invitation() {
        let mut conn = test_conn();

        // Super user performing the push
        let pusher = make_user(Level::Super, Status::Active);
        Create::<User>::create(&mut conn, pusher.clone()).unwrap();

        // Create a school first (needed for foreign key on owners)
        let school_id = Id::default();
        let school_batch = MutationBatch {
            batch_id: "batch-school".into(),
            mutations: vec![Mutation {
                table: LogTable::Schools as i32,
                operation: OP_INSERT,
                row_key: school_id.to_string(),
                insert: Some(school_insert_data(&school_id)),
                update: None,
            }],
        };
        let ack = process_batch(&mut conn, &pusher, &school_batch);
        assert!(ack.success, "school insert should succeed: {:?}", ack);

        // Now push user + owner in same batch (invitation flow)
        let invited_id = Id::default();
        let batch = MutationBatch {
            batch_id: "batch-invite".into(),
            mutations: vec![
                Mutation {
                    table: LogTable::Users as i32,
                    operation: OP_INSERT,
                    row_key: invited_id.to_string(),
                    insert: Some(user_insert_data(
                        &invited_id,
                        "0712345678",
                        "Invited User",
                        i32::from(Level::Normal),
                        i32::from(Status::Invited),
                    )),
                    update: None,
                },
                Mutation {
                    table: LogTable::Owners as i32,
                    operation: OP_INSERT,
                    row_key: format!("{}|{}", school_id, invited_id),
                    insert: Some(owner_insert_data(&school_id, &invited_id)),
                    update: None,
                },
            ],
        };

        let ack = process_batch(&mut conn, &pusher, &batch);
        assert!(ack.success, "invitation batch should succeed: {:?}", ack);
        assert_eq!(ack.results.len(), 2);
        assert_eq!(ack.results[0].code, CODE_OK, "user insert should succeed");
        assert_eq!(ack.results[1].code, CODE_OK, "owner insert should succeed");

        // Verify user exists in DB
        let found: Option<User> = Find::<Id, User>::find(&mut conn, invited_id).unwrap();
        assert!(found.is_some(), "invited user should exist in DB");
        let found = found.unwrap();
        assert_eq!(found.status, Status::Invited);
        assert_eq!(found.level, Level::Normal);
    }

    #[test]
    fn test_push_phone_conflict_resolution() {
        let mut conn = test_conn();

        // Super user performing the pushes
        let pusher = make_user_with_phone(Level::Super, Status::Active, "0759762268");
        Create::<User>::create(&mut conn, pusher.clone()).unwrap();

        // Create a school
        let school_id = Id::default();
        let school_batch = MutationBatch {
            batch_id: "batch-school".into(),
            mutations: vec![Mutation {
                table: LogTable::Schools as i32,
                operation: OP_INSERT,
                row_key: school_id.to_string(),
                insert: Some(school_insert_data(&school_id)),
                update: None,
            }],
        };
        process_batch(&mut conn, &pusher, &school_batch);

        // First: create user A with phone 0700111222
        let user_a_id = Id::default();
        let first_batch = MutationBatch {
            batch_id: "batch-first".into(),
            mutations: vec![
                Mutation {
                    table: LogTable::Users as i32,
                    operation: OP_INSERT,
                    row_key: user_a_id.to_string(),
                    insert: Some(user_insert_data(
                        &user_a_id,
                        "0700111222",
                        "User A",
                        i32::from(Level::Normal),
                        i32::from(Status::Invited),
                    )),
                    update: None,
                },
                Mutation {
                    table: LogTable::Owners as i32,
                    operation: OP_INSERT,
                    row_key: format!("{}|{}", school_id, user_a_id),
                    insert: Some(owner_insert_data(&school_id, &user_a_id)),
                    update: None,
                },
            ],
        };
        let ack = process_batch(&mut conn, &pusher, &first_batch);
        assert!(ack.success, "first invite should succeed: {:?}", ack);

        // Create a second school for the conflict test
        let school_b_id = Id::default();
        let school_b_batch = MutationBatch {
            batch_id: "batch-school-b".into(),
            mutations: vec![Mutation {
                table: LogTable::Schools as i32,
                operation: OP_INSERT,
                row_key: school_b_id.to_string(),
                insert: Some(school_insert_data(&school_b_id)),
                update: None,
            }],
        };
        process_batch(&mut conn, &pusher, &school_b_batch);

        // Second: create user B with SAME phone 0700111222 + teacher at school B
        let user_b_id = Id::default();
        let conflict_batch = MutationBatch {
            batch_id: "batch-conflict".into(),
            mutations: vec![
                Mutation {
                    table: LogTable::Users as i32,
                    operation: OP_INSERT,
                    row_key: user_b_id.to_string(),
                    insert: Some(user_insert_data(
                        &user_b_id,
                        "0700111222",
                        "User B",
                        i32::from(Level::Normal),
                        i32::from(Status::Invited),
                    )),
                    update: None,
                },
                Mutation {
                    table: LogTable::Teachers as i32,
                    operation: OP_INSERT,
                    row_key: format!("{}|{}", school_b_id, user_b_id),
                    insert: Some(teacher_insert_data(&school_b_id, &user_b_id)),
                    update: None,
                },
            ],
        };
        let ack = process_batch(&mut conn, &pusher, &conflict_batch);

        // User mutation should be CODE_CONFLICT (phone already exists)
        assert_eq!(
            ack.results[0].code, CODE_CONFLICT,
            "user insert should return conflict: {:?}",
            ack.results[0]
        );
        // Teacher mutation should succeed (rewritten to use existing user A's ID)
        assert_eq!(
            ack.results[1].code, CODE_OK,
            "teacher insert should succeed with rewritten user: {:?}",
            ack.results[1]
        );

        // User B should NOT exist in the database
        let found_b: Option<User> = Find::<Id, User>::find(&mut conn, user_b_id).unwrap();
        assert!(found_b.is_none(), "orphaned user B should not exist");

        // User A should still exist
        let found_a: Option<User> = Find::<Id, User>::find(&mut conn, user_a_id).unwrap();
        assert!(found_a.is_some(), "original user A should still exist");

        // changelog should contain a Delete record for the orphaned user
        let records = LOG.with(|cell| cell.borrow().read_from(0).unwrap());
        assert!(
            records
                .iter()
                .any(|r| r.table == LogTable::Users as u8 && r.op == OP_DELETE as u8),
            "changelog should contain a Delete for the orphaned user ID"
        );
    }

    #[test]
    fn test_push_permission_denied_normal_user() {
        let mut conn = test_conn();

        // Create a super user to set up the school
        let super_user = make_user_with_phone(Level::Super, Status::Active, "0759762268");
        Create::<User>::create(&mut conn, super_user.clone()).unwrap();

        let school_id = Id::default();
        let school_batch = MutationBatch {
            batch_id: "batch-school".into(),
            mutations: vec![Mutation {
                table: LogTable::Schools as i32,
                operation: OP_INSERT,
                row_key: school_id.to_string(),
                insert: Some(school_insert_data(&school_id)),
                update: None,
            }],
        };
        process_batch(&mut conn, &super_user, &school_batch);

        // Create a normal user who is NOT a member of the school
        let normal_user = make_user_with_phone(Level::Normal, Status::Active, "0711111111");
        Create::<User>::create(&mut conn, normal_user.clone()).unwrap();

        // Normal user tries to insert a department in a school they're not a member of
        let dept_data = InsertData {
            row: Some(insert_data::Row::Department(DepartmentInsert {
                school: school_id.to_string(),
                name: "Math".into(),
                description: None,
            })),
        };
        let batch = MutationBatch {
            batch_id: "batch-denied".into(),
            mutations: vec![Mutation {
                table: LogTable::Departments as i32,
                operation: OP_INSERT,
                row_key: format!("{}|Math", school_id),
                insert: Some(dept_data),
                update: None,
            }],
        };

        let ack = process_batch(&mut conn, &normal_user, &batch);
        assert!(!ack.success);
        assert_eq!(ack.results[0].code, CODE_PERMISSION_DENIED);
    }

    #[test]
    fn test_push_normal_user_cannot_create_system_user() {
        let mut conn = test_conn();

        // Create a normal user
        let normal_user = make_user_with_phone(Level::Normal, Status::Active, "0759762268");
        Create::<User>::create(&mut conn, normal_user.clone()).unwrap();

        // Normal user tries to create a system-level user
        let sys_user_id = Id::default();
        let batch = MutationBatch {
            batch_id: "batch-sys".into(),
            mutations: vec![Mutation {
                table: LogTable::Users as i32,
                operation: OP_INSERT,
                row_key: sys_user_id.to_string(),
                insert: Some(user_insert_data(
                    &sys_user_id,
                    "0722222222",
                    "System User",
                    i32::from(Level::System),
                    i32::from(Status::Invited),
                )),
                update: None,
            }],
        };

        let ack = process_batch(&mut conn, &normal_user, &batch);
        assert!(!ack.success);
        assert_eq!(
            ack.results[0].code, CODE_PERMISSION_DENIED,
            "normal user should not be able to create system users: {:?}",
            ack.results[0]
        );
    }

    // --- SyncFilter tests ---

    #[test]
    fn test_filter_super_sees_all() {
        let filter = SyncFilter::Super;

        // table_visible: Super sees every table
        assert!(filter.table_visible(LogTable::Schools));
        assert!(filter.table_visible(LogTable::Users));
        assert!(filter.table_visible(LogTable::Plans));
        assert!(filter.table_visible(LogTable::Scopes));

        // row_visible: Super sees every row regardless of school
        let school_id = Id::default();
        assert!(filter.row_visible(LogTable::Schools, &school_id.to_string(), Some(&school_id),));
        assert!(filter.row_visible(LogTable::Users, &Id::default().to_string(), None,));
    }

    #[test]
    fn test_filter_normal_sees_own_schools() {
        let user_id = Id::default();
        let school_a = Id::default();
        let school_b = Id::default();

        let mut schools = HashSet::new();
        schools.insert(school_a);
        // user is member of school_a only, NOT school_b

        let filter = SyncFilter::Normal {
            schools,
            co_members: HashSet::new(),
            user_id,
        };

        // table_visible: Normal user with schools can see school-scoped tables
        assert!(filter.table_visible(LogTable::Departments));
        assert!(filter.table_visible(LogTable::Users));
        assert!(filter.table_visible(LogTable::Plans));

        // row_visible: school_a data visible, school_b not
        assert!(
            filter.row_visible(
                LogTable::Departments,
                &format!("{}|Science", school_a),
                Some(&school_a),
            ),
            "should see school_a data"
        );
        assert!(
            !filter.row_visible(
                LogTable::Departments,
                &format!("{}|Art", school_b),
                Some(&school_b),
            ),
            "should NOT see school_b data"
        );
        // Own user row should be visible
        assert!(
            filter.row_visible(LogTable::Users, &user_id.to_string(), None),
            "should see own user row"
        );
        // Plans should be visible to everyone
        assert!(
            filter.row_visible(LogTable::Plans, &Id::default().to_string(), None),
            "should see plans"
        );
    }

    #[test]
    fn test_filter_normal_sees_co_members() {
        let user_id = Id::default();
        let co_member_id = Id::default();
        let stranger_id = Id::default();
        let school = Id::default();

        let mut schools = HashSet::new();
        schools.insert(school);
        let mut co_members = HashSet::new();
        co_members.insert(co_member_id);

        let filter = SyncFilter::Normal {
            schools,
            co_members,
            user_id,
        };

        // Co-member user row should be visible
        assert!(
            filter.row_visible(LogTable::Users, &co_member_id.to_string(), None),
            "should see co-member user row"
        );
        // Stranger user row should NOT be visible
        assert!(
            !filter.row_visible(LogTable::Users, &stranger_id.to_string(), None),
            "should NOT see stranger user row"
        );
    }

    #[test]
    fn test_delete_operation() {
        let mut conn = test_conn();

        let super_user = make_user(Level::Super, Status::Active);
        Create::<User>::create(&mut conn, super_user.clone()).unwrap();

        // Insert a school first
        let school_id = Id::default();
        let insert_batch = MutationBatch {
            batch_id: "batch-ins".into(),
            mutations: vec![Mutation {
                table: LogTable::Schools as i32,
                operation: OP_INSERT,
                row_key: school_id.to_string(),
                insert: Some(school_insert_data(&school_id)),
                update: None,
            }],
        };
        let ack = process_batch(&mut conn, &super_user, &insert_batch);
        assert!(ack.success, "school insert should succeed");

        // Now delete the school
        let delete_batch = MutationBatch {
            batch_id: "batch-del".into(),
            mutations: vec![Mutation {
                table: LogTable::Schools as i32,
                operation: OP_DELETE,
                row_key: school_id.to_string(),
                insert: None,
                update: None,
            }],
        };
        let ack = process_batch(&mut conn, &super_user, &delete_batch);
        assert!(ack.success, "school delete should succeed: {:?}", ack);
        assert_eq!(ack.results[0].code, CODE_OK);

        // Verify the changelog has a record for the school delete
        let records = LOG.with(|cell| cell.borrow().read_from(0).unwrap());
        assert!(
            records
                .iter()
                .any(|r| r.table == LogTable::Schools as u8 && r.op == OP_DELETE as u8),
            "changelog should contain the school delete"
        );
    }

    #[test]
    fn test_update_operation() {
        let mut conn = test_conn();

        let super_user = make_user(Level::Super, Status::Active);
        Create::<User>::create(&mut conn, super_user.clone()).unwrap();

        // Insert a school
        let school_id = Id::default();
        let insert_batch = MutationBatch {
            batch_id: "batch-ins".into(),
            mutations: vec![Mutation {
                table: LogTable::Schools as i32,
                operation: OP_INSERT,
                row_key: school_id.to_string(),
                insert: Some(school_insert_data(&school_id)),
                update: None,
            }],
        };
        let ack = process_batch(&mut conn, &super_user, &insert_batch);
        assert!(ack.success, "school insert should succeed");

        // Update the school name
        let updated_school = UpdateData {
            row: Some(update_data::Row::School(SchoolUpdate {
                name: Some("Updated School Name".into()),
                motto: Some("New Motto".into()),
                phone: None,
                email: None,
                county: None,
                domain: None,
                established: None,
                status: None,
            })),
        };
        let update_batch = MutationBatch {
            batch_id: "batch-upd".into(),
            mutations: vec![Mutation {
                table: LogTable::Schools as i32,
                operation: OP_UPDATE,
                row_key: school_id.to_string(),
                insert: None,
                update: Some(updated_school),
            }],
        };
        let ack = process_batch(&mut conn, &super_user, &update_batch);
        assert!(ack.success, "school update should succeed: {:?}", ack);
        assert_eq!(ack.results[0].code, CODE_OK);

        // Verify the changelog has a record for the school update
        let records = LOG.with(|cell| cell.borrow().read_from(0).unwrap());
        assert!(
            records
                .iter()
                .any(|r| r.table == LogTable::Schools as u8 && r.op == OP_UPDATE as u8),
            "changelog should contain the school update"
        );

        // Verify the actual data in the database reflects the update
        use crate::db::schema::schools::dsl as s;
        use diesel::prelude::*;
        let name: String = s::schools
            .filter(s::id.eq(school_id.to_string()))
            .select(s::name)
            .first(&mut conn)
            .expect("school should exist");
        assert_eq!(name, "Updated School Name");
    }

    #[test]
    fn test_should_rebuild_filter_on_membership_change() {
        let user = make_user(Level::Normal, Status::Active);

        // Owner table record — conservative: always triggers rebuild
        let record_owner = Record::new(Id::default(), LogTable::Owners as u8, OP_INSERT as u8, 0);
        assert!(
            should_rebuild_filter_record(&record_owner, &user),
            "owner insert should trigger filter rebuild"
        );

        // Teacher table record — conservative: always triggers rebuild
        let record_teacher =
            Record::new(Id::default(), LogTable::Teachers as u8, OP_INSERT as u8, 0);
        assert!(
            should_rebuild_filter_record(&record_teacher, &user),
            "teacher insert should trigger filter rebuild (conservative)"
        );

        // School insert should NOT trigger rebuild
        let record_school = Record::new(Id::default(), LogTable::Schools as u8, OP_INSERT as u8, 0);
        assert!(
            !should_rebuild_filter_record(&record_school, &user),
            "school insert should NOT trigger filter rebuild"
        );

        // Scopes should NOT trigger for Normal users
        let record_scopes = Record::new(Id::default(), LogTable::Scopes as u8, OP_INSERT as u8, 0);
        assert!(
            !should_rebuild_filter_record(&record_scopes, &user),
            "scopes insert should NOT trigger rebuild for normal user"
        );

        // Scopes SHOULD trigger for System users
        let system_user = make_user(Level::System, Status::Active);
        assert!(
            should_rebuild_filter_record(&record_scopes, &system_user),
            "scopes insert should trigger rebuild for system user"
        );
    }

    #[test]
    fn test_batch_size_limit() {
        let mut conn = test_conn();

        let super_user = make_user(Level::Super, Status::Active);
        Create::<User>::create(&mut conn, super_user.clone()).unwrap();

        // Create a batch with more than MAX_MUTATIONS_PER_BATCH mutations
        let mutations: Vec<Mutation> = (0..MAX_MUTATIONS_PER_BATCH + 1)
            .map(|_| {
                let id = Id::default();
                Mutation {
                    table: LogTable::Schools as i32,
                    operation: OP_INSERT,
                    row_key: id.to_string(),
                    insert: Some(school_insert_data(&id)),
                    update: None,
                }
            })
            .collect();

        let batch = MutationBatch {
            batch_id: "batch-too-big".into(),
            mutations,
        };

        let ack = process_batch(&mut conn, &super_user, &batch);
        assert!(!ack.success, "oversized batch should be rejected");
        assert!(
            ack.error
                .as_ref()
                .map(|e| e.contains("batch too large"))
                .unwrap_or(false),
            "error should mention batch size: {:?}",
            ack.error
        );
        assert!(
            ack.results.is_empty(),
            "no mutation results should be returned for rejected batch"
        );
    }

    #[test]
    fn test_changelog_record_roundtrip() {
        let id = Id::default();
        let record = Record::new(id, 5, 1, 0b0000_0000_0000_0111);
        let bytes = record.to_bytes();
        let decoded = Record::from_bytes(&bytes);
        assert_eq!(record, decoded);
    }

    /// Reproduces the exact scenario from production logs:
    /// Fresh DB, single batch with School Insert + Owner Insert.
    /// The owner references the school just created and the pushing user.
    #[test]
    fn test_school_plus_owner_same_batch() {
        let mut conn = test_conn();

        // Create the pushing super user in DB
        let user = make_user(Level::Super, Status::Active);
        Create::<User>::create(&mut conn, user.clone()).unwrap();

        let school_id = Id::default();

        let batch = MutationBatch {
            batch_id: "batch-school-owner".into(),
            mutations: vec![
                Mutation {
                    table: LogTable::Schools as i32,
                    operation: OP_INSERT,
                    row_key: school_id.to_string(),
                    insert: Some(school_insert_data(&school_id)),
                    update: None,
                },
                Mutation {
                    table: LogTable::Owners as i32,
                    operation: OP_INSERT,
                    row_key: format!("{}|{}", school_id, user.id),
                    insert: Some(owner_insert_data(&school_id, &user.id)),
                    update: None,
                },
            ],
        };

        let ack = process_batch(&mut conn, &user, &batch);

        // Both mutations must succeed
        assert_eq!(ack.results.len(), 2, "expected 2 results");
        assert!(
            ack.results[0].success,
            "school insert should succeed: code={} error={:?}",
            ack.results[0].code, ack.results[0].error
        );
        assert!(
            ack.results[1].success,
            "owner insert should succeed: code={} error={:?}",
            ack.results[1].code, ack.results[1].error
        );
        assert!(ack.success, "batch should succeed: {:?}", ack);

        // Verify owner row exists in DB
        use crate::db::schema::owners::dsl as o;
        use diesel::prelude::*;
        let count: i64 = o::owners
            .filter(o::school.eq(school_id.to_string()))
            .filter(o::user.eq(user.id.to_string()))
            .count()
            .get_result(&mut conn)
            .unwrap();
        assert_eq!(count, 1, "owner row should exist in database");
    }
}
