# Ledger — Task Board

> Tasks are ordered by dependency and priority. Execute top-to-bottom.
> This file lives at `eduxal/LEDGER_TASKS.md` because the agent's project root is `eduxal/`.
> The executor should read/write server files at `/home/abdihakim/Documents/GITHUB/eduxal-labs/ledger/`.

---

### Task L0: Commit current server state

**Files to create/modify:** None (git operations only)
**Context files to read (if needed):** None
**Depends on:** Nothing

**Specification:**

Create meaningful, chunked git commits of the current dirty working tree in the `ledger` repo.
Run `git status` first to see all uncommitted changes. Group the commits logically:

1. `chore: update dependencies` — `Cargo.toml`, `Cargo.lock`
2. `db: update schema and migrations` — `migrations/`, `src/db/schema/`
3. `feat: database layer updates` — `src/db/database/`, `src/db/mod.rs`
4. `feat: type system updates` — `src/types/`
5. `feat: proto and service layer updates` — `src/proto/`, `src/services/`, `protos/`
6. `chore: build and config updates` — `build.rs`, `src/config/`, `src/main.rs`, `src/server.rs`
7. `docs: update AGENT.md and TASKS.md` — `AGENT.md`, `TASKS.md`

Use `git add <paths>` + `git commit -m "<message>"` for each group. Skip any group that has no changes. Do NOT use `git add .`.

**Update after completion:**
- [ ] Mark this task `[x]`

---

### Task L1: Write the new `sync.proto`

**Files to create/modify:** `/home/abdihakim/Documents/GITHUB/eduxal-labs/ledger/protos/services/sync.proto`
**Context files to read (if needed):** `eduxal/CONVERSATION_CONTEXT.md` §5j (full proto definition)
**Depends on:** Task L0

**Specification:**

Replace the entire contents of `protos/services/sync.proto` with the action-based proto definition from CONVERSATION_CONTEXT §5j. The new proto has:

**Service definition:**
```protobuf
service Sync {
  rpc PushActions(stream ActionRequest) returns (stream ActionResponse);
  rpc WatchChanges(WatchRequest) returns (stream SyncDelta);
}
```

**New messages (push direction):**
- `ActionRequest` — `{id: int32, action: int32, payload: bytes}`
- `ActionResponse` — `{id: int32, success: bool, code: int32, error: string, rows: repeated ActionRow, file_urls: repeated FileUrl}`
- `ActionRow` — `{table: int32, operation: int32, row_key: string, data: InsertData}`
- 77 action payload messages (`CreateSchoolPayload`, `UpdateSchoolPayload`, ..., `DeleteDiscountPayload`)
- 2 batch record messages (`AttendanceRecord`, `GradeRecord`)

**Kept messages (watch direction — copy from current file):**
- `WatchRequest` — unchanged
- `SyncDelta` — unchanged
- `FileUrl` — unchanged
- `InsertData` oneof — unchanged (all 30 `*Insert` messages)
- All 30 `*Insert` messages (`UserInsert`, `SchoolInsert`, ..., `DiscountInsert`) — copy exactly from current file

**Removed messages:**
- `MutationBatch`, `Mutation`, `PushAck`, `MutationResult` — delete
- `UpdateData` oneof and all 25 `*Update` messages — delete

Keep the `import` lines for `types/member.proto` and `types/role.proto` if any `*Insert` messages reference them. Otherwise remove unused imports.

After writing the file, run `cargo build 2>&1 | head -100` to verify protobuf compilation. The build will have errors in `src/services/sync.rs` and `src/proto/services/sync.rs` because they reference removed types — that is expected and will be fixed in later tasks.

**Update after completion:**
- [ ] Mark this task `[x]`

---

### Task L2: Update the proto adapter (`src/proto/services/sync.rs`)

**Files to create/modify:** `/home/abdihakim/Documents/GITHUB/eduxal-labs/ledger/src/proto/services/sync.rs`
**Context files to read (if needed):** None — current file content is inlined below
**Depends on:** Task L1

**Specification:**

The proto adapter bridges the generated tonic code with the app's `Sync` trait. Update it to match the new `sync.proto` RPC signatures.

**Current file (to be replaced entirely):**
The current file defines:
- `trait Sync` with `push_changes(token, Streaming<MutationBatch>) -> Result<mpsc::Receiver<PushAck>>`
- `watch_changes(token, WatchRequest) -> Result<Self::WatchStream>`
- `impl sync_server::Sync for T` adapter

**New file content:**

```rust
tonic::include_proto!("sync");
use crate::types::{error::Result, token::Token};
use std::future::Future;
pub use sync_server::SyncServer;
use tokio::sync::mpsc;
use tokio_stream::Stream;
use tonic::{Request, Response, Status, Streaming};

pub trait Sync: Send + ::std::marker::Sync + 'static + Sized {
    type Config: Send + ::std::marker::Sync + 'static;
    type WatchStream: Stream<Item = Result<SyncDelta>> + Send + 'static;

    fn new(config: Self::Config) -> SyncServer<Self>;

    fn push_actions(
        &self,
        token: Token,
        stream: Streaming<ActionRequest>,
    ) -> impl Future<Output = Result<mpsc::Receiver<ActionResponse>>> + Send;

    fn watch_changes(
        &self,
        token: Token,
        request: WatchRequest,
    ) -> impl Future<Output = Result<Self::WatchStream>> + Send;
}

#[tonic::async_trait]
impl<T: Sync> sync_server::Sync for T {
    type PushActionsStream =
        tokio_stream::wrappers::ReceiverStream<std::result::Result<ActionResponse, Status>>;
    type WatchChangesStream =
        std::pin::Pin<Box<dyn Stream<Item = std::result::Result<SyncDelta, Status>> + Send>>;

    async fn push_actions(
        &self,
        request: Request<Streaming<ActionRequest>>,
    ) -> std::result::Result<Response<Self::PushActionsStream>, Status> {
        let token: Token = request
            .metadata()
            .get("authorization")
            .ok_or_else(|| Status::unauthenticated("missing authorization"))?
            .to_str()
            .map_err(|_| Status::unauthenticated("invalid authorization"))?
            .strip_prefix("Bearer ")
            .ok_or_else(|| Status::unauthenticated("missing Bearer prefix"))?
            .parse()?;

        let stream = request.into_inner();
        let rx = Sync::push_actions(self, token, stream).await?;

        let (tx_out, rx_out) = mpsc::channel(64);
        tokio::spawn(async move {
            let mut rx = rx;
            while let Some(resp) = rx.recv().await {
                if tx_out.send(Ok(resp)).await.is_err() {
                    break;
                }
            }
        });

        Ok(Response::new(tokio_stream::wrappers::ReceiverStream::new(
            rx_out,
        )))
    }

    async fn watch_changes(
        &self,
        request: Request<WatchRequest>,
    ) -> std::result::Result<Response<Self::WatchChangesStream>, Status> {
        let token: Token = request
            .metadata()
            .get("authorization")
            .ok_or_else(|| Status::unauthenticated("missing authorization"))?
            .to_str()
            .map_err(|_| Status::unauthenticated("invalid authorization"))?
            .strip_prefix("Bearer ")
            .ok_or_else(|| Status::unauthenticated("missing Bearer prefix"))?
            .parse()?;

        let watch_request = request.into_inner();
        let stream = Sync::watch_changes(self, token, watch_request).await?;

        let mapped = Box::pin(tokio_stream::StreamExt::map(stream, |result| {
            result.map_err(|e| Status::from(e))
        }));

        Ok(Response::new(mapped))
    }
}
```

After writing, run `cargo build 2>&1 | head -100` — expect errors only in `src/services/sync.rs` (the implementation), not in this adapter file.

**Update after completion:**
- [ ] Mark this task `[x]`

---

### Task L3: Create the action dispatcher (`src/db/database/tables/actions.rs`)

**Files to create/modify:**
- `/home/abdihakim/Documents/GITHUB/eduxal-labs/ledger/src/db/database/tables/actions.rs` (NEW)
- `/home/abdihakim/Documents/GITHUB/eduxal-labs/ledger/src/db/database/tables/mod.rs` (add `pub mod actions;`)

**Context files to read (if needed):** Read `src/db/database/tables/insert.rs`, `src/db/database/tables/update.rs`, `src/db/database/tables/delete.rs` to see the function signatures the handlers will call.

**Depends on:** Task L1

**Specification:**

Create `actions.rs` — the central dispatcher that maps a `SyncAction` integer to a handler function. Each handler:
1. Deserializes the `payload` bytes into the appropriate `*Payload` proto message
2. Calls the existing `insert_*`, `update_*`, `delete_*` functions from `insert.rs`/`update.rs`/`delete.rs`
3. Appends changelog record(s)
4. Returns a list of `ActionRow` results

**Structure:**

```rust
use crate::db::changelog::{LOG, Record};
use crate::db::database::tables::{insert, update, delete};
use crate::proto::services::sync::*;
use crate::types::error::{Error, Result};
use crate::types::id::Id;
use crate::types::role::{Organisation, Permissions, Resource, Action, Actions};
use crate::types::user::User;
use diesel::SqliteConnection;
use prost::Message;

/// SyncAction integer values — must match the client's SyncAction enum.
pub mod sync_action {
    pub const CREATE_SCHOOL: i32 = 0;
    pub const UPDATE_SCHOOL: i32 = 1;
    pub const DELETE_SCHOOL: i32 = 2;
    pub const CREATE_TEACHER: i32 = 3;
    pub const UPDATE_TEACHER: i32 = 4;
    pub const DELETE_TEACHER: i32 = 5;
    pub const CREATE_STAFF: i32 = 6;
    pub const UPDATE_STAFF: i32 = 7;
    pub const DELETE_STAFF: i32 = 8;
    pub const CREATE_OWNER: i32 = 9;
    pub const DELETE_OWNER: i32 = 10;
    pub const CREATE_STUDENT: i32 = 11;
    pub const UPDATE_STUDENT: i32 = 12;
    pub const DELETE_STUDENT: i32 = 13;
    pub const ENROLL_STUDENT: i32 = 14;
    pub const UNENROLL_STUDENT: i32 = 15;
    pub const CREATE_GUARDIAN: i32 = 16;
    pub const UPDATE_GUARDIAN: i32 = 17;
    pub const DELETE_GUARDIAN: i32 = 18;
    pub const CREATE_DEPARTMENT: i32 = 19;
    pub const UPDATE_DEPARTMENT: i32 = 20;
    pub const DELETE_DEPARTMENT: i32 = 21;
    pub const CREATE_TERM: i32 = 22;
    pub const UPDATE_TERM: i32 = 23;
    pub const DELETE_TERM: i32 = 24;
    pub const ASSIGN_CLASS_TEACHER: i32 = 25;
    pub const UNASSIGN_CLASS_TEACHER: i32 = 26;
    pub const ASSIGN_SUBJECT: i32 = 27;
    pub const UNASSIGN_SUBJECT: i32 = 28;
    pub const CREATE_TIMETABLE_ENTRY: i32 = 29;
    pub const UPDATE_TIMETABLE_ENTRY: i32 = 30;
    pub const DELETE_TIMETABLE_ENTRY: i32 = 31;
    pub const MARK_ATTENDANCE: i32 = 32;
    pub const DELETE_ATTENDANCE: i32 = 33;
    pub const CREATE_LESSON: i32 = 34;
    pub const DELETE_LESSON: i32 = 35;
    pub const CREATE_EXAM: i32 = 36;
    pub const UPDATE_EXAM: i32 = 37;
    pub const DELETE_EXAM: i32 = 38;
    pub const CREATE_PAPER: i32 = 39;
    pub const UPDATE_PAPER: i32 = 40;
    pub const DELETE_PAPER: i32 = 41;
    pub const MARK_GRADES: i32 = 42;
    pub const UPDATE_GRADE: i32 = 43;
    pub const DELETE_GRADE: i32 = 44;
    pub const UPDATE_MASTERY: i32 = 45;
    pub const CREATE_FEE: i32 = 46;
    pub const UPDATE_FEE: i32 = 47;
    pub const DELETE_FEE: i32 = 48;
    pub const CREATE_INVOICE: i32 = 49;
    pub const UPDATE_INVOICE: i32 = 50;
    pub const DELETE_INVOICE: i32 = 51;
    pub const CREATE_PAYMENT: i32 = 52;
    pub const UPDATE_PAYMENT: i32 = 53;
    pub const DELETE_PAYMENT: i32 = 54;
    pub const APPROVE_PAYMENT: i32 = 55;
    pub const CREATE_ANNOUNCEMENT: i32 = 56;
    pub const UPDATE_ANNOUNCEMENT: i32 = 57;
    pub const DELETE_ANNOUNCEMENT: i32 = 58;
    pub const CREATE_ROLE: i32 = 59;
    pub const UPDATE_ROLE: i32 = 60;
    pub const DELETE_ROLE: i32 = 61;
    pub const ASSIGN_ROLE: i32 = 62;
    pub const UNASSIGN_ROLE: i32 = 63;
    pub const UPDATE_USER: i32 = 64;
    pub const DELETE_USER: i32 = 65;
    pub const UPDATE_SETTINGS: i32 = 66;
    pub const CREATE_PLAN: i32 = 67;
    pub const UPDATE_PLAN: i32 = 68;
    pub const DELETE_PLAN: i32 = 69;
    pub const UPDATE_AI_USAGE: i32 = 70;
    pub const CREATE_SUBSCRIPTION: i32 = 71;
    pub const UPDATE_SUBSCRIPTION: i32 = 72;
    pub const DELETE_SUBSCRIPTION: i32 = 73;
    pub const CREATE_DISCOUNT: i32 = 74;
    pub const UPDATE_DISCOUNT: i32 = 75;
    pub const DELETE_DISCOUNT: i32 = 76;
}

/// Result of executing a single action. Contains the rows to return to the client.
pub struct ActionResult {
    pub rows: Vec<ActionRow>,
    pub file_urls: Vec<FileUrl>,
}

/// Maps SyncAction integer to (Resource, Action) for authorization.
pub fn action_permission(action_id: i32) -> Result<(Resource, Action, Option<Id>)> {
    // Implementation: big match on action_id returning the required (Resource, Action).
    // For school-scoped actions, the school ID is extracted from the payload in execute_action.
    // This function returns the resource/action pair only.
    todo!("Implement in task body")
}

/// Central dispatcher: deserialize payload, authorize, execute, return rows.
pub fn execute_action(
    conn: &mut SqliteConnection,
    user: &User,
    action_id: i32,
    payload: &[u8],
) -> Result<ActionResult> {
    match action_id {
        sync_action::CREATE_SCHOOL => handle_create_school(conn, user, payload),
        sync_action::UPDATE_SCHOOL => handle_update_school(conn, user, payload),
        // ... all 77 action handlers
        _ => Err(Error::Internal),
    }
}
```

For this task, create the file with:
1. The `sync_action` constants module (complete — all 77 values)
2. The `ActionResult` struct
3. The `action_permission` function mapping each action ID to `(Resource, Action)` — full match with all 77 arms
4. The `execute_action` dispatcher — full match with all 77 arms, each calling a `handle_*` function
5. **Stub** every `handle_*` function with `todo!()` — they will be implemented in Tasks L4–L8

Also add `pub mod actions;` to `src/db/database/tables/mod.rs`.

Run `cargo check 2>&1 | head -50` — expect compilation success (all handlers are `todo!()` stubs so they type-check).

**Update after completion:**
- [ ] Mark this task `[x]`

---

### Task L4: Implement action handlers — Schools, Users, Settings, Plans

**Files to create/modify:** `/home/abdihakim/Documents/GITHUB/eduxal-labs/ledger/src/db/database/tables/actions.rs`
**Context files to read (if needed):** Read `insert.rs`, `update.rs`, `delete.rs` to see exact function signatures for the tables in this group.
**Depends on:** Task L3

**Specification:**

Implement the `handle_*` functions for these actions (replacing `todo!()`):

**Schools (3 actions):**
- `handle_create_school` — Special: includes owner invitation. Decode `CreateSchoolPayload`. In a transaction: (1) look up owner by phone — if not found, create invited user; (2) insert school; (3) insert owner record; (4) insert default settings. Append changelog records for each table touched. Return `ActionResult` with all created rows as `ActionRow` entries.
- `handle_update_school` — Decode `UpdateSchoolPayload`. Authorize `Schools.Update` for the school. Call `update::update_school`. Append changelog. Return updated row.
- `handle_delete_school` — Decode `DeleteSchoolPayload`. Authorize `Schools.Delete`. Call `update::update_school` with `status = deleted`. Append changelog. Return updated row.

**Users (2 actions):**
- `handle_update_user` — Decode `UpdateUserPayload`. Authorize `Users.Update`. Call `update::update_user`. Append changelog. Return updated row.
- `handle_delete_user` — Decode `DeleteUserPayload`. Authorize `Users.Delete`. Set `status = deleted`. Append changelog. Return updated row.

**Settings (1 action):**
- `handle_update_settings` — Decode `UpdateSettingsPayload`. Authorize `Schools.Update` for the school. Upsert settings. Append changelog. Return settings row.

**Plans (3 actions):**
- `handle_create_plan` — Decode `CreatePlanPayload`. Authorize `Plans.Create` (System/Super only). Insert plan. Append changelog. Return plan row.
- `handle_update_plan` — Decode `UpdatePlanPayload`. Authorize `Plans.Update`. Update plan. Append changelog. Return updated row.
- `handle_delete_plan` — Decode `DeletePlanPayload`. Authorize `Plans.Delete`. Delete plan. Append changelog.

Each handler must:
1. Decode payload with `prost::Message::decode(payload)` — return `Error::Internal` on decode failure
2. Call `conn.authorize(token, org, perms)` where `org` = `Organisation::School(school_id)` for school-scoped or `Organisation::System` for system-scoped
3. Execute DB operations using existing `insert::*`, `update::*`, `delete::*` functions
4. Append to changelog via `LOG.with(|log| log.borrow_mut().append(record))`
5. Build and return `ActionResult` with `ActionRow` entries. Each `ActionRow` contains `InsertData` with the appropriate `*Insert` variant built from the row data using existing `From` impls in `rows.rs`.

**Update after completion:**
- [ ] Mark this task `[x]`

---

### Task L5: Implement action handlers — Member invitation pattern (Teachers, Staff, Owners, Guardians)

**Files to create/modify:** `/home/abdihakim/Documents/GITHUB/eduxal-labs/ledger/src/db/database/tables/actions.rs`
**Context files to read (if needed):** Read `insert.rs`, `update.rs`, `delete.rs`, `users.rs` (for phone lookup)
**Depends on:** Task L4

**Specification:**

Implement the member invitation handlers. These all follow the same pattern:

**Pattern (for Create*):**
1. Decode the `Create*Payload` (contains `user_id`, `phone`, `name`, optional `email`)
2. Authorize `{Resource}.Create` for the school
3. Look up user by phone:
   - **Found:** Use existing user's ID for the member record. Return the existing user row + new member row.
   - **Not found:** Create user with `status = Invited`, `level = Normal`. Create member record pointing to new user. Return both rows.
4. Append changelog records for all tables touched (users + member table)
5. Return `ActionResult` with all rows

**Teachers (3 actions):**
- `handle_create_teacher` — Invitation pattern. Resource: `Teachers.Create`.
- `handle_update_teacher` — Decode `UpdateTeacherPayload`. Authorize `Teachers.Update`. Update teacher. Return row.
- `handle_delete_teacher` — Decode `DeleteTeacherPayload`. Authorize `Teachers.Delete`. Delete teacher. Return nothing.

**Staff (3 actions):**
- `handle_create_staff` — Invitation pattern. Resource: `Staff.Create`.
- `handle_update_staff` — Decode `UpdateStaffPayload`. Authorize `Staff.Update`. Update staff. Return row.
- `handle_delete_staff` — Decode `DeleteStaffPayload`. Authorize `Staff.Delete`. Delete staff.

**Owners (2 actions):**
- `handle_create_owner` — Invitation pattern. Resource: `Owners.Create`.
- `handle_delete_owner` — Decode `DeleteOwnerPayload`. Authorize `Owners.Delete`. Delete owner.

**Guardians (3 actions):**
- `handle_create_guardian` — Invitation pattern. Resource: `Students.Create` (guardians are under Students resource). Also requires `student` ADM in payload.
- `handle_update_guardian` — Decode `UpdateGuardianPayload`. Authorize `Students.Update`. Update guardian.
- `handle_delete_guardian` — Decode `DeleteGuardianPayload`. Authorize `Students.Delete`. Delete guardian.

**Phone conflict resolution:** If the client sent a `user_id` but a different user already has that phone, the server:
1. Uses the existing user (ignoring the client's `user_id`)
2. Creates the member pointing to the existing user
3. Returns both rows — the client reconciles by deleting the orphaned local user

**Update after completion:**
- [ ] Mark this task `[x]`

---

### Task L6: Implement action handlers — Students, Enrollments, Departments, Terms

**Files to create/modify:** `/home/abdihakim/Documents/GITHUB/eduxal-labs/ledger/src/db/database/tables/actions.rs`
**Context files to read (if needed):** Read `insert.rs`, `update.rs`, `delete.rs`
**Depends on:** Task L3

**Specification:**

Implement handlers for these straightforward CRUD + assign actions:

**Students (3 actions):**
- `handle_create_student` — Decode `CreateStudentPayload`. Authorize `Students.Create`. Insert student. Append changelog. Return student row.
- `handle_update_student` — Decode `UpdateStudentPayload`. Authorize `Students.Update`. Update student. Return updated row.
- `handle_delete_student` — Decode `DeleteStudentPayload`. Authorize `Students.Delete`. Soft-delete student (status change). Return updated row.

**Enrollments (2 actions):**
- `handle_enroll_student` — Decode `EnrollStudentPayload`. Authorize `Students.Assign`. Insert enrollment. Return enrollment row.
- `handle_unenroll_student` — Decode `UnenrollStudentPayload`. Authorize `Students.Unassign`. Delete enrollment.

**Departments (3 actions):**
- `handle_create_department` — Decode `CreateDepartmentPayload`. Authorize `Departments.Create`. Insert department. Return row.
- `handle_update_department` — Decode `UpdateDepartmentPayload`. Authorize `Departments.Update`. Update department. Return row.
- `handle_delete_department` — Decode `DeleteDepartmentPayload`. Authorize `Departments.Delete`. Delete department.

**Terms (3 actions):**
- `handle_create_term` — Decode `CreateTermPayload`. Authorize `Classes.Create` (terms are academic structure). Insert term. Return row.
- `handle_update_term` — Decode `UpdateTermPayload`. Authorize `Classes.Update`. Update term. Return row.
- `handle_delete_term` — Decode `DeleteTermPayload`. Authorize `Classes.Delete`. Delete term.

Each handler follows the standard pattern: decode → authorize → execute → changelog → return.

**Update after completion:**
- [ ] Mark this task `[x]`

---

### Task L7: Implement action handlers — Classes, Attendance, Lessons, Exams, Grades

**Files to create/modify:** `/home/abdihakim/Documents/GITHUB/eduxal-labs/ledger/src/db/database/tables/actions.rs`
**Context files to read (if needed):** Read `insert.rs`, `update.rs`, `delete.rs`
**Depends on:** Task L3

**Specification:**

**Classes (7 actions):**
- `handle_assign_class_teacher` — Decode `AssignClassTeacherPayload`. Authorize `Classes.Assign`. Insert class_teacher. Return row.
- `handle_unassign_class_teacher` — Decode `UnassignClassTeacherPayload`. Authorize `Classes.Unassign`. Delete class_teacher.
- `handle_assign_subject` — Decode `AssignSubjectPayload`. Authorize `Classes.Assign`. Insert subject. Return row.
- `handle_unassign_subject` — Decode `UnassignSubjectPayload`. Authorize `Classes.Unassign`. Delete subject.
- `handle_create_timetable_entry` — Decode `CreateTimetableEntryPayload`. Authorize `Classes.Create`. Insert timetable. Return row.
- `handle_update_timetable_entry` — Decode `UpdateTimetableEntryPayload`. Authorize `Classes.Update`. Update timetable. Return row.
- `handle_delete_timetable_entry` — Decode `DeleteTimetableEntryPayload`. Authorize `Classes.Delete`. Delete timetable.

**Attendance (2 actions):**
- `handle_mark_attendance` — Decode `MarkAttendancePayload`. Authorize `Attendance.Mark`. **Bulk:** iterate `records`, upsert each attendance row. Append changelog per row. Return all affected rows.
- `handle_delete_attendance` — Decode `DeleteAttendancePayload`. Authorize `Attendance.Delete`. Delete single attendance record.

**Lessons (2 actions):**
- `handle_create_lesson` — Decode `CreateLessonPayload`. Authorize `Lessons.Create`. Insert lesson. Return row.
- `handle_delete_lesson` — Decode `DeleteLessonPayload`. Authorize `Lessons.Delete`. Delete lesson.

**Exams (6 actions):**
- `handle_create_exam` — Decode `CreateExamPayload`. Authorize `Exams.Create`. Insert exam. Return row.
- `handle_update_exam` — Decode `UpdateExamPayload`. Authorize `Exams.Update`. Update exam. Return row.
- `handle_delete_exam` — Decode `DeleteExamPayload`. Authorize `Exams.Delete`. Delete exam.
- `handle_create_paper` — Decode `CreatePaperPayload`. Authorize `Exams.Create`. Insert paper. Return row.
- `handle_update_paper` — Decode `UpdatePaperPayload`. Authorize `Exams.Update`. Update paper. Return row.
- `handle_delete_paper` — Decode `DeletePaperPayload`. Authorize `Exams.Delete`. Delete paper.

**Grades (4 actions):**
- `handle_mark_grades` — Decode `MarkGradesPayload`. Authorize `Grades.Mark`. **Bulk:** iterate `records`, upsert each grade row. Append changelog per row. Return all affected rows.
- `handle_update_grade` — Decode `UpdateGradePayload`. Authorize `Grades.Update`. Update grade. Return row.
- `handle_delete_grade` — Decode `DeleteGradePayload`. Authorize `Grades.Delete`. Delete grade.
- `handle_update_mastery` — Decode `UpdateMasteryPayload`. Authorize `Grades.Mark`. Upsert mastery. Return row.

**Update after completion:**
- [ ] Mark this task `[x]`

---

### Task L8: Implement action handlers — Finance, Announcements, Roles, AI, Subscriptions, Discounts

**Files to create/modify:** `/home/abdihakim/Documents/GITHUB/eduxal-labs/ledger/src/db/database/tables/actions.rs`
**Context files to read (if needed):** Read `insert.rs`, `update.rs`, `delete.rs`
**Depends on:** Task L3

**Specification:**

**Fees (3 actions):**
- `handle_create_fee` — Authorize `Fees.Create`. Insert fee. Return row.
- `handle_update_fee` — Authorize `Fees.Update`. Update fee. Return row.
- `handle_delete_fee` — Authorize `Fees.Delete`. Delete fee.

**Invoices (3 actions):**
- `handle_create_invoice` — Authorize `Fees.Create`. Insert invoice. Return row.
- `handle_update_invoice` — Authorize `Fees.Update`. Update invoice. Return row.
- `handle_delete_invoice` — Authorize `Fees.Delete`. Delete invoice.

**Payments (4 actions):**
- `handle_create_payment` — Authorize `Payments.Create`. Insert payment. Return row.
- `handle_update_payment` — Authorize `Payments.Update`. Update payment. Return row.
- `handle_delete_payment` — Authorize `Payments.Delete`. Delete payment.
- `handle_approve_payment` — Authorize `Payments.Approve`. Update payment status to approved. Return row.

**Announcements (3 actions):**
- `handle_create_announcement` — Authorize `Announcements.Create`. Insert announcement. Return row.
- `handle_update_announcement` — Authorize `Announcements.Update`. Update announcement. Return row.
- `handle_delete_announcement` — Authorize `Announcements.Delete`. Delete announcement.

**Roles (5 actions):**
- `handle_create_role` — Authorize `Roles.Create`. Insert role. Return row.
- `handle_update_role` — Authorize `Roles.Update`. Update role. Return row.
- `handle_delete_role` — Authorize `Roles.Delete`. Delete role.
- `handle_assign_role` — Authorize `Roles.Assign`. Insert scope. Return row.
- `handle_unassign_role` — Authorize `Roles.Unassign`. Delete scope.

**AI (1 action):**
- `handle_update_ai_usage` — Authorize `AI.Update`. Upsert ai_usage. Return row.

**Subscriptions (3 actions):**
- `handle_create_subscription` — Authorize `Plans.Create`. Insert subscription. Return row.
- `handle_update_subscription` — Authorize `Plans.Update`. Update subscription. Return row.
- `handle_delete_subscription` — Authorize `Plans.Delete`. Delete subscription.

**Discounts (3 actions):**
- `handle_create_discount` — Authorize `Plans.Create`. Insert discount. Return row.
- `handle_update_discount` — Authorize `Plans.Update`. Update discount. Return row.
- `handle_delete_discount` — Authorize `Plans.Delete`. Delete discount.

**Update after completion:**
- [ ] Mark this task `[x]`

---

### Task L9: Rewrite `services/sync.rs` — push_actions flow

**Files to create/modify:** `/home/abdihakim/Documents/GITHUB/eduxal-labs/ledger/src/services/sync.rs`
**Context files to read (if needed):** Read current `src/services/sync.rs` to understand the watch loop (KEEP) and the existing push flow (REPLACE).
**Depends on:** Tasks L2, L4–L8

**Specification:**

Rewrite the `push_changes` method to become `push_actions`. The watch loop stays.

**What to remove from `services/sync.rs`:**
- The `process_batch` function and all batch-related logic
- The `process_mutation` function
- The `LogTable` enum (moved to `actions.rs` as `sync_action` constants)
- The invitation detection/pairing logic
- Batch ID tracking, mutation result mapping

**What to keep:**
- The `watch_changes` implementation (the watch loop that reads changelog + snapshots)
- The `SyncFilter` enum and filtering logic
- The `SyncService` struct and its `Config` type
- The `impl Sync for SyncService` block (update method signatures)

**New push flow:**

```rust
fn push_actions(
    &self,
    token: Token,
    stream: Streaming<ActionRequest>,
) -> impl Future<Output = Result<mpsc::Receiver<ActionResponse>>> + Send {
    async move {
        // 1. Validate token, load user
        let user = /* load user from token, same as current */;

        let (tx, rx) = mpsc::channel(64);

        // 2. Spawn task to process actions sequentially
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

                let response = process_action(&user, &request);

                if tx.send(response).await.is_err() {
                    break; // client disconnected
                }
            }
        });

        Ok(rx)
    }
}
```

**The `process_action` function:**

```rust
fn process_action(user: &User, request: &ActionRequest) -> ActionResponse {
    // Get a DB connection from the pool
    let conn = CONN.with(|c| c.borrow_mut());

    // Execute within a transaction
    let result = conn.transaction(|conn| {
        actions::execute_action(conn, user, request.action, &request.payload)
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
                Error::ForeignKey(_) => (3, format!("Foreign key error: {}", e)),
                Error::NothingToUpdate => (3, "Nothing to update".to_string()),
                Error::UserNotFound | Error::SchoolNotFound => (4, format!("{}", e)),
                _ => (3, format!("Validation error: {}", e)),
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
```

Run `cargo check 2>&1 | head -100` — project should compile cleanly.

**Update after completion:**
- [ ] Mark this task `[x]`

---

### Task L10: Delete old apply.rs and clean up

**Files to create/modify:**
- `/home/abdihakim/Documents/GITHUB/eduxal-labs/ledger/src/db/database/tables/apply.rs` — DELETE
- `/home/abdihakim/Documents/GITHUB/eduxal-labs/ledger/src/db/database/tables/mod.rs` — remove `pub mod apply;`

**Context files to read (if needed):** None
**Depends on:** Task L9

**Specification:**

Delete `apply.rs` entirely — it contained `apply_mutation`, `validate_insert`, `validate_update` which are all replaced by the action handlers.

Remove `pub mod apply;` from `mod.rs`.

Search for any remaining references to `apply::` in the codebase with `grep -rn "apply::" src/`. Fix any lingering imports.

Run `cargo check` — should compile cleanly.

**Update after completion:**
- [ ] Mark this task `[x]`

---

### Task L11: Full build + basic smoke test

**Files to create/modify:** None (verification only)
**Context files to read (if needed):** None
**Depends on:** Task L10

**Specification:**

1. Run `cargo build --release 2>&1` — must compile with zero errors.
2. Run `cargo test 2>&1` — if tests exist, they should pass (some may need updating if they tested the old mutation flow).
3. Review any remaining `todo!()` calls with `grep -rn "todo!" src/` — there should be none in production code paths.
4. Verify the server binary starts: `cargo run -- --help` or equivalent.

If there are compile errors, fix them. If there are remaining `todo!()` stubs, implement them.

**Update after completion:**
- [ ] Mark this task `[x]`

---

### Task L12: Commit the sync redesign

**Files to create/modify:** None (git operations only)
**Context files to read (if needed):** None
**Depends on:** Task L11

**Specification:**

Create structured commits for the sync redesign:

1. `proto: replace mutation-based sync with action-based sync` — `protos/services/sync.proto`
2. `feat: add action dispatcher and handler stubs` — `src/db/database/tables/actions.rs`, `src/db/database/tables/mod.rs`
3. `feat: implement all 77 action handlers` — `src/db/database/tables/actions.rs` (full implementation)
4. `feat: rewrite sync service push flow for action-based model` — `src/services/sync.rs`, `src/proto/services/sync.rs`
5. `chore: remove old mutation-based apply.rs` — delete `apply.rs`, update `mod.rs`

Use `git add <paths>` + `git commit -m "<message>"` for each.

**Update after completion:**
- [ ] Mark this task `[x]`
