#![allow(dead_code)]

use crate::config::Config;
use crate::config::storage::sign;
use crate::db::changelog::{LOG, Record};
use crate::db::database::CONN;
use crate::db::database::tables::actions;
use crate::db::database::traits::{Database, Load};
use crate::proto::services::sync::{
    ActionRequest, ActionResponse, FileUrl, InsertData, Sync, SyncDelta, SyncServer, WatchRequest,
};
use crate::types::error::{Error, Result};
use crate::types::id::Id;
use crate::types::role::{Action, Permissions, Resource, Role};
use crate::types::token::Token;
use crate::types::user::{Level, Status, User};
use chrono::Utc;
use diesel::Connection;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use tokio::sync::mpsc;
use tokio_stream::{Stream, StreamExt};
use tonic::Streaming;
use tracing::{info, warn};

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
    SubjectCatalog = 31,
    Topics = 32,
    Streams = 33,
    Mpesa = 34,
    SchemePages = 36,
    AnswerPages = 37,
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
            31 => Some(Self::SubjectCatalog),
            32 => Some(Self::Topics),
            33 => Some(Self::Streams),
            34 => Some(Self::Mpesa),
            36 => Some(Self::SchemePages),
            37 => Some(Self::AnswerPages),
            _ => None,
        }
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
            Self::Exams | Self::Papers | Self::SchemePages => Some(Resource::Exams),
            Self::Grades | Self::Mastery | Self::AnswerPages => Some(Resource::Grades),
            Self::Fees | Self::Invoices => Some(Resource::Fees),
            Self::Payments => Some(Resource::Payments),
            Self::Announcements => Some(Resource::Announcements),
            Self::Roles | Self::Scopes => Some(Resource::Roles),
            Self::Plans | Self::Subscriptions | Self::Discounts => Some(Resource::Plans),
            Self::AiUsage => Some(Resource::AI),
            Self::Terms => Some(Resource::Schools),
            Self::SubjectCatalog | Self::Topics => Some(Resource::Subjects),
            Self::Streams | Self::Mpesa => Some(Resource::Schools),
        }
    }

    /// Extract the school_id from a row_key, if applicable.
    /// Most school-scoped tables have the school as the first "|"-delimited segment.
    /// System tables (users, plans) return None.
    pub fn school_from_key(self, row_key: &str) -> Option<Id> {
        match self {
            // System-level tables — no school scope
            Self::Users | Self::Plans | Self::SubjectCatalog | Self::Topics => None,
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
}

pub const OP_INSERT: i32 = 0;
pub const OP_UPDATE: i32 = 1;
pub const OP_DELETE: i32 = 2;

// ---------------------------------------------------------------------------
// SyncService
// ---------------------------------------------------------------------------

pub struct SyncService<C> {
    config: Arc<C>,
}

impl<C: Config + Send + ::std::marker::Sync + 'static> Sync for SyncService<C> {
    type Config = Arc<C>;
    type WatchStream = std::pin::Pin<Box<dyn Stream<Item = Result<SyncDelta>> + Send>>;

    fn new(config: Self::Config) -> SyncServer<Self> {
        SyncServer::new(Self { config })
    }

    async fn push_actions(
        &self,
        token: Token,
        stream: Streaming<ActionRequest>,
    ) -> Result<mpsc::Receiver<ActionResponse>> {
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

        info!(user_id = %user.id, level = ?user.level, "[SYNC] PushActions stream opened");

        let (tx, rx) = mpsc::channel::<ActionResponse>(64);
        let notify = self.config.change_notifier().clone();

        tokio::spawn(async move {
            let mut stream = stream;
            while let Some(request) = stream.next().await {
                let request = match request {
                    Ok(r) => r,
                    Err(e) => {
                        warn!("Stream error: {}", e);
                        break;
                    }
                };

                info!(
                    user_id = %user.id,
                    request_id = request.id,
                    action = request.action,
                    "[SYNC] PUSH ← received action"
                );

                let response = process_action(&user, &request);

                info!(
                    user_id = %user.id,
                    request_id = response.id,
                    success = response.success,
                    code = response.code,
                    error = %response.error,
                    rows = response.rows.len(),
                    "[SYNC] PUSH → action result"
                );

                // Notify waiting watch loops if the action succeeded
                if response.success {
                    notify.notify_waiters();
                }

                if tx.send(response).await.is_err() {
                    break; // client disconnected
                }
            }
            info!(user_id = %user.id, "[SYNC] PushActions stream closed");
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

        info!(user_id = %user.id, level = ?user.level, last_cursor = request.last_seq, "[SYNC] WatchChanges stream opened");

        let (tx, rx) = mpsc::channel::<Result<SyncDelta>>(256);
        let last_cursor = request.last_seq;
        let notify = self.config.change_notifier().clone();

        tokio::spawn(async move {
            let result = watch_loop(&tx, &user, last_cursor, &notify).await;
            info!(user_id = %user.id, success = result.is_ok(), "[SYNC] WatchChanges stream closed");
            if result.is_err() {
                // Stream ended or send failed — task exits naturally
            }
        });

        Ok(Box::pin(tokio_stream::wrappers::ReceiverStream::new(rx)))
    }
}

// ---------------------------------------------------------------------------
// Push action processing
// ---------------------------------------------------------------------------

/// Process a single action request from the client.
///
/// 1. Looks up the required (Resource, Action) permission for the action.
/// 2. Checks whether the user is authorized (Super bypass, System role-check,
///    Normal membership-check).
/// 3. Executes the action handler inside a database transaction.
/// 4. Returns an `ActionResponse` with success/failure and any affected rows.
fn process_action(user: &User, request: &ActionRequest) -> ActionResponse {
    let result = CONN.with(|cell| {
        let conn = &mut *cell.borrow_mut();

        // 1. Look up the required permission
        let (resource, action) = actions::action_permission(request.action)?;

        // 2. Authorization check
        check_action_permission(conn, user, resource, action)?;

        // 3. Execute inside a transaction
        conn.transaction(|conn| actions::execute_action(conn, request.action, &request.payload))
    });

    match result {
        Ok(action_result) => ActionResponse {
            id: request.id,
            success: true,
            code: 0,
            error: String::new(),
            rows: action_result.rows,
            file_urls: action_result.file_urls,
        },
        Err(e) => {
            let (code, error) = match &e {
                Error::Forbidden => (1, "Permission denied".to_string()),
                Error::Conflict => (2, "Conflict".to_string()),
                Error::ForeignKey => (3, "Foreign key constraint violated".to_string()),
                Error::NothingToUpdate => (3, "Nothing to update".to_string()),
                Error::UserNotFound | Error::SchoolNotFound => (4, format!("{e}")),
                _ => (3, format!("Validation error: {e}")),
            };
            ActionResponse {
                id: request.id,
                success: false,
                code,
                error,
                rows: vec![],
                file_urls: vec![],
            }
        }
    }
}

/// Check whether the user has the required permission for the action.
///
/// - Super users bypass all checks.
/// - System users are checked against their system-scoped roles.
/// - Normal users are allowed for now (fine-grained school-scoped
///   authorization is deferred — membership is checked at the handler
///   level via foreign key constraints).
fn check_action_permission(
    conn: &mut diesel::SqliteConnection,
    user: &User,
    resource: Resource,
    action: Action,
) -> Result<()> {
    match user.level {
        Level::Super => Ok(()),
        Level::System => {
            // Load system-scoped roles and check aggregate permissions
            let roles: Vec<Role> = Load::<&User, Role>::load(conn, &user)?;
            let mut granted = Permissions::new();
            for role in &roles {
                granted += role.permissions;
            }
            if granted[resource].contains(action) {
                Ok(())
            } else {
                warn!(
                    user_id = %user.id,
                    resource = ?resource,
                    action = ?action,
                    "system user permission denied on push"
                );
                Err(Error::Forbidden)
            }
        }
        Level::Normal => {
            // Normal users: membership-based access.
            // For now, allow all actions — the action handlers enforce
            // school membership implicitly via FK constraints.  Full
            // school-scoped role checking is deferred.
            Ok(())
        }
    }
}

// ---------------------------------------------------------------------------
// Watch / Sync filter
// ---------------------------------------------------------------------------

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
                // Users, Plans, and global subject catalogs are always potentially visible
                if table == LogTable::Users
                    || resource == Resource::Plans
                    || resource == Resource::Subjects
                {
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
                if table == LogTable::Users
                    || resource == Resource::Plans
                    || resource == Resource::Subjects
                {
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

                // Plans and global subject catalogs are always visible
                if resource == Resource::Plans || resource == Resource::Subjects {
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

                // Plans and global subject catalogs are always visible
                if resource == Resource::Plans || resource == Resource::Subjects {
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
    31,
    32, // subject_catalog, topics (global catalogs — before school data that references them)
    3, 7, 8, // owners, teachers, staff
    4, 5, 6, // students, guardians, departments
    27, 25, // scopes, settings
    9,  // terms
    33, // streams
    11, 10, 12, // enrollments, class_teachers, subjects (subject_teachers)
    13, 14, 15, // attendance, timetable, lessons
    16, 17, 36, // exams, papers, scheme_pages
    18, 37, // grades, answer_pages
    19, 20, 21, // fees, invoices, payments
    22, 23, 24, // announcements, mastery, aiusage
    29, 30, // subscriptions, discounts
    34, // mpesa
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

    info!("[SYNC] WATCH → starting full snapshot");
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
                warn!(table = table_num, error = %e, "[SYNC] WATCH → snapshot query failed, skipping table");
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
                "[SYNC] WATCH → snapshot table {:?}: {} total, {} visible", table_enum, rows.len(), visible_count
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
        "[SYNC] WATCH → full snapshot complete, {} rows sent", rows_sent
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

    let changelog_len = LOG.with(|cell| cell.borrow().len().unwrap_or(0));

    // If the cursor is misaligned or ahead of the changelog, fall back to
    // a full snapshot.  This handles:
    //   - Corrupted/stale lastSeq from the client
    //   - Server restart with a fresh changelog
    //   - Client from a different server instance
    let cursor_valid = cursor % 24 == 0 && cursor <= changelog_len;
    if !cursor_valid && cursor != 0 {
        warn!(
            user_id = %user.id,
            cursor = cursor,
            changelog_len = changelog_len,
            "[SYNC] WATCH → invalid cursor (misaligned or ahead), falling back to full snapshot"
        );
        cursor = 0;
    }

    // Track the delete-file cursor separately.  On initial connect we
    // start from the current end of the deletes file (the full snapshot
    // already captured all live rows, so historical deletes are moot).
    let mut delete_cursor: u64 = LOG.with(|cell| cell.borrow().delete_cursor().unwrap_or(0));

    // --- Initial sync: if cursor == 0, dump all tables ---
    if cursor == 0 {
        info!(user_id = %user.id, "[SYNC] WATCH → cold start (cursor=0), sending full snapshot");
        if send_full_snapshot(tx, &filter).await.is_err() {
            return Err(());
        }
        // After snapshot, set cursor to the current changelog length so we
        // don't re-send entries that were created while we were snapshotting.
        cursor = LOG.with(|cell| cell.borrow().len().unwrap_or(0));
        delete_cursor = LOG.with(|cell| cell.borrow().delete_cursor().unwrap_or(0));
        info!(user_id = %user.id, cursor = cursor, delete_cursor = delete_cursor, "[SYNC] WATCH → snapshot done, cursor={}, delete_cursor={}", cursor, delete_cursor);

        // Send a bookmark delta so the client learns the cursor position
        // after the full snapshot.  table=0 signals "no data, just a cursor
        // update" — the client should persist seq as lastSeq and ignore
        // the rest of the fields.
        let bookmark = SyncDelta {
            seq: cursor as i64,
            table: 0,
            operation: OP_INSERT,
            row_key: String::new(),
            data: None,
            file_urls: vec![],
        };
        if tx.send(Ok(bookmark)).await.is_err() {
            return Err(());
        }
        info!(user_id = %user.id, seq = cursor, "[SYNC] WATCH → sent snapshot bookmark seq={}", cursor);
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
                    info!(user_id = %user.id, "[SYNC] WATCH → woke up via notify");
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
            "[SYNC] WATCH → incremental: {} changelog records, {} delete records from cursor={}",
            records.len(), delete_records.len(), cursor
        );

        cursor += (records.len() as u64) * 24;
        delete_cursor = new_delete_cursor;

        // If any record in this batch touches a membership/scope table,
        // rebuild the filter NOW — before populating table_min_ts — so
        // that tables which become visible in this very batch (e.g.
        // Schools and Owners when the user is just added to a school) are
        // captured in the data fetch below.  Rebuilding after the scan
        // (the old behaviour) was too late: the cursor had already
        // advanced past those records and they would never be retried.
        if records
            .iter()
            .any(|r| should_rebuild_filter_record(r, user))
        {
            info!(user_id = %user.id, "[SYNC] WATCH → pre-batch filter rebuild (membership change detected)");
            if let Ok(f) = SyncFilter::build(user) {
                filter = f;
            }
        }

        let mut deltas_sent: usize = 0;

        // Collect changed tables with minimum timestamps (Insert/Update only).
        let mut table_min_ts: HashMap<u8, i64> = HashMap::new();

        for record in &records {
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
            info!(user_id = %user.id, table = %tbl_dbg, min_ts = min_ts, "[SYNC] WATCH → fetching changed rows for {:?} since ts={}", tbl_dbg, min_ts);
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
                    "[SYNC] WATCH → sending upsert delta table={} key={}",
                    table_num, row.row_key
                );

                if tx.send(Ok(delta)).await.is_err() {
                    return Err(());
                }
                deltas_sent += 1;
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
                "[SYNC] WATCH → sending delete delta table={} key={}",
                tbl_dbg, delete.key
            );

            if tx.send(Ok(delta)).await.is_err() {
                return Err(());
            }
            deltas_sent += 1;
        }

        // If we processed changelog records but sent no visible deltas,
        // send a bookmark so the client advances its stored cursor.
        if deltas_sent == 0 && (!records.is_empty() || !delete_records.is_empty()) {
            let bookmark = SyncDelta {
                seq: cursor as i64,
                table: 0,
                operation: OP_INSERT,
                row_key: String::new(),
                data: None,
                file_urls: vec![],
            };
            if tx.send(Ok(bookmark)).await.is_err() {
                return Err(());
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

// ---------------------------------------------------------------------------
// File URL helpers
// ---------------------------------------------------------------------------

fn file_urls_for_delta(
    table: LogTable,
    op: i32,
    row_key: &str,
    _data: Option<&InsertData>,
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
            // Always return a GET URL for student profile images.
            // Row key format: "{school_id}|{adm}"
            let parts: Vec<&str> = row_key.split('|').collect();
            if parts.len() >= 2 {
                // 30 days in seconds — GET URL is valid for one month
                let get_ttl: u64 = sign::GET_TTL;
                let path = format!("schools/{}/students/{}/image", parts[0], parts[1]);
                let get_url = sign::url(&path, get_ttl, false);
                // expiry in milliseconds since epoch
                let expiry_ms = (Utc::now().timestamp() + get_ttl as i64) * 1000;
                vec![FileUrl {
                    path,
                    put_url: None,
                    get_url: Some(get_url),
                    expiry: expiry_ms,
                }]
            } else {
                vec![]
            }
        }
        LogTable::SchemePages => {
            // Row key format: "{school}|{exam}|{subject}|{paper_str}|{page}"
            // paper_str is "" when paper IS NULL (single-paper subject).
            let parts: Vec<&str> = row_key.split('|').collect();
            if parts.len() >= 5 {
                let school = parts[0];
                let exam = parts[1];
                let subject: i32 = parts[2].parse().unwrap_or(0);
                let paper: i32 = if parts[3].is_empty() {
                    0
                } else {
                    parts[3].parse().unwrap_or(0)
                };
                let page: i32 = parts[4].parse().unwrap_or(0);
                let s3_key = format!(
                    "schools/{}/exams/{}/papers/{}_{}/scheme/{}",
                    school, exam, subject, paper, page
                );
                let get_url = sign::url(&s3_key, sign::GET_TTL, false);
                let local_path = format!(
                    "submissions/{}/{}/{}_{}/scheme/{}.jpg",
                    school, exam, subject, paper, page
                );
                let expiry_ms = (Utc::now().timestamp() + sign::GET_TTL as i64) * 1000;
                vec![FileUrl {
                    path: local_path,
                    put_url: None,
                    get_url: Some(get_url),
                    expiry: expiry_ms,
                }]
            } else {
                vec![]
            }
        }
        LogTable::AnswerPages => {
            // Row key format: "{school}|{exam}|{student}|{subject}|{paper_str}|{page}"
            // paper_str is "" when paper IS NULL.
            let parts: Vec<&str> = row_key.split('|').collect();
            if parts.len() >= 6 {
                let school = parts[0];
                let exam = parts[1];
                let student: i32 = parts[2].parse().unwrap_or(0);
                let subject: i32 = parts[3].parse().unwrap_or(0);
                let paper: i32 = if parts[4].is_empty() {
                    0
                } else {
                    parts[4].parse().unwrap_or(0)
                };
                let page: i32 = parts[5].parse().unwrap_or(0);
                let s3_key = format!(
                    "schools/{}/exams/{}/papers/{}_{}/students/{}/{}",
                    school, exam, subject, paper, student, page
                );
                let get_url = sign::url(&s3_key, sign::GET_TTL, false);
                let local_path = format!(
                    "submissions/{}/{}/{}_{}/{}/{}.jpg",
                    school, exam, subject, paper, student, page
                );
                let expiry_ms = (Utc::now().timestamp() + sign::GET_TTL as i64) * 1000;
                vec![FileUrl {
                    path: local_path,
                    put_url: None,
                    get_url: Some(get_url),
                    expiry: expiry_ms,
                }]
            } else {
                vec![]
            }
        }
        _ => vec![],
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // --- school_from_key tests ---

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
        assert!(filter.row_visible(LogTable::Schools, &school_id.to_string(), Some(&school_id)));
        assert!(filter.row_visible(LogTable::Users, &Id::default().to_string(), None));
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

    // --- should_rebuild_filter tests ---

    /// Create a User struct with the given level and status.
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
    fn test_changelog_record_roundtrip() {
        let id = Id::default();
        let record = Record::new(id, 5, 1, 0b0000_0000_0000_0111);
        let bytes = record.to_bytes();
        let decoded = Record::from_bytes(&bytes);
        assert_eq!(record, decoded);
    }
}
