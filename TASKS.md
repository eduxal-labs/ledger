# Ledger — Task Board

---

## Phase 0 — Foundation (Port from `../server/`)

### Task 01: Port and expand database traits
**Files to create/modify:** `src/db/database/traits.rs`
**Reference files to read:** `../server/src/db/database/traits.rs`
**Depends on:** None

**Specification:**
The current `traits.rs` has only `Create`, `Find`, `Update`, and `Database`. Port the following additional traits from `../server/src/db/database/traits.rs`:

```rust
pub trait Load<Input, Output> {
    fn load(&mut self, input: Input) -> Result<Vec<Output>>;
}

pub trait Authorize {
    fn authorize(
        &mut self,
        token: Token,
        organisation: Organisation,
        permissions: Permissions,
    ) -> Result<()>;
}

pub trait List<Filter, Offset, Output> {
    fn list(&mut self, filter: Filter, offset: Option<Offset>, limit: u32) -> Result<Paginated<Output, Offset>>;
}

pub trait Search<Query, Offset, Output> {
    fn search(&mut self, query: Query, offset: Option<Offset>, limit: u32) -> Result<Paginated<Output, Offset>>;
}

pub trait Delete<Filter, Target = (), Output = ()> {
    fn delete(&mut self, filter: Filter) -> Result<Output>;
}

pub trait Purge<Filter, Target = (), Output = ()> {
    fn purge(&mut self, filter: Filter) -> Result<Output>;
}
```

Also update the `Database` trait and `impl<T: Authorize> Database for LocalKey<RefCell<T>>` to include all new methods, matching the pattern in `../server/src/db/database/traits.rs`.

Required imports to add: `Token` from `crate::types::token`, `Organisation` and `Permissions` from the new role types (Task 02). Since Task 02 doesn't exist yet, use temporary placeholder imports and fix them when Task 02 is done. Or, better: implement Tasks 01 and 02 together if the executor sees both are needed.

Note: `Paginated<O, Offset>` does not exist yet. Create `src/types/paginated.rs`:
```rust
pub struct Paginated<T, O> {
    pub items: Vec<T>,
    pub next: Option<O>,
}

pub trait Offset<O: Copy> {
    fn offset(&self) -> O;
}
```
Add `pub mod paginated;` to `src/types/mod.rs`.

**Update after completion:**
- [ ] Mark this task `[x]`

---

### Task 02: Port and expand Action, Actions, Resource types
**Files to create/modify:** `src/types/role/action.rs`, `src/types/role/actions.rs`, `src/types/role/resource.rs`, `src/types/role/mod.rs`
**Reference files to read:** `../server/src/types/role/action.rs`, `../server/src/types/role/actions.rs`, `../server/src/types/role/resource.rs`, `../server/src/types/role/mod.rs`
**Depends on:** None

**Specification:**

Create `src/types/role/` directory with four files:

**`action.rs`** — Port from `../server/` but expand and change repr:
```rust
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(u16)]
pub enum Action {
    Create   = 1,      // bit 0
    Read     = 2,      // bit 1
    Update   = 4,      // bit 2
    Delete   = 8,      // bit 3
    Purge    = 16,     // bit 4
    Assign   = 32,     // bit 5
    Unassign = 64,     // bit 6
    Mark     = 128,    // bit 7
    Approve  = 256,    // bit 8
}
```
Add `From<proto::Action>` and `From<Action> for proto::Action` conversions (proto types won't exist until Phase 1 — add `#[cfg(feature = "proto")]` or leave as TODO comments that the executor fills in when protos are created).

**`actions.rs`** — Port from `../server/` but change inner type from `u8` to `u16`:
```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Actions(u16);

const ALL_BITS: u16 = Action::Create as u16
    | Action::Read as u16
    | Action::Update as u16
    | Action::Delete as u16
    | Action::Purge as u16
    | Action::Assign as u16
    | Action::Unassign as u16
    | Action::Mark as u16
    | Action::Approve as u16;
```
Port all operator impls (`Add`, `Sub`, `AddAssign`, `SubAssign`) from `../server/` changing `u8` → `u16`. Port `contains`, `is_empty`, `len`, `bits`, `iter` methods. Update `iter()` to include the 4 new actions. Port all tests and add tests for new actions.

**`resource.rs`** — Port from `../server/` but expand from 5 to 18 resources:
```rust
#[derive(Debug, macros::Count, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Resource {
    Users = 1,
    Schools = 2,
    Owners = 3,
    Teachers = 4,
    Staff = 5,
    Students = 6,
    Departments = 7,
    Classes = 8,
    Attendance = 9,
    Lessons = 10,
    Exams = 11,
    Grades = 12,
    Fees = 13,
    Payments = 14,
    Announcements = 15,
    Roles = 16,
    Plans = 17,
    AI = 18,
}
```
Port `TryFrom<u8>`, `TryFrom<i32>`, `From<Resource> for usize` (0-based: `(resource as u8 - 1) as usize`). Add `VARIANTS` constant array. Proto conversions left as TODO until Phase 1.

**`mod.rs`**:
```rust
mod action;
mod actions;
mod resource;

pub use action::*;
pub use actions::*;
pub use resource::*;
```

Add `pub mod role;` to `src/types/mod.rs`.

**Update after completion:**
- [ ] Mark this task `[x]`

---

### Task 03: Port Permissions struct
**Files to create/modify:** `src/types/role/permissions.rs`, `src/types/role/mod.rs`
**Reference files to read:** `../server/src/types/role/permissions.rs`
**Depends on:** Task 02

**Specification:**

Port `Permissions` from `../server/` with these changes:
- Array size: `[Actions; Resource::COUNT]` (18 instead of 5)
- Binary encoding: **3 bytes** per non-empty resource: `[resource_id: u8, actions_lo: u8, actions_hi: u8]` (little-endian u16), instead of 2 bytes
- `TryFrom<&[u8]>`: validate length is multiple of 3 (not 2), parse 3-byte tuples
- `From<&Permissions> for Vec<u8>`: emit 3-byte tuples
- `ToSql<Binary, Sqlite>` and `FromSql<Binary, Sqlite>`: same as `../server/` but with 3-byte encoding
- Port all operator impls (`Add`, `Sub`, `AddAssign`, `SubAssign`, `PartialEq` variants)
- Port `Index<Resource>`, `IndexMut<Resource>`, `is_empty()`
- Port the `system()` constructor but update for new resources (give system users `Read` on: Users, Schools, Owners, Teachers, Staff, Students, Departments, Classes, Roles, Plans)
- Proto conversions: leave as TODO until Phase 1
- Port all tests from `../server/`, update expected byte lengths (×3 instead of ×2)

Add `mod permissions;` and `pub use permissions::*;` to `src/types/role/mod.rs`.

**Update after completion:**
- [ ] Mark this task `[x]`

---

### Task 04: Port Organisation enum
**Files to create/modify:** `src/types/role/organisation.rs`, `src/types/role/mod.rs`
**Reference files to read:** `../server/src/types/role/organisation.rs`
**Depends on:** None

**Specification:**

Port `Organisation` enum as-is from `../server/`:
```rust
pub enum Organisation {
    System,
    Account,
    School(Id),
}
```
Port all impls: `From<Option<Id>>`, `From<Id>`, `optional()`, `PartialEq<Id>`, `FromStr`, `TryFrom<String>`, `TryFrom<Option<T>>`.

Add `mod organisation;` and `pub use organisation::*;` to `src/types/role/mod.rs`.

**Update after completion:**
- [ ] Mark this task `[x]`

---

### Task 05: Port member Role/Roles bitmask
**Files to create/modify:** `src/types/member/role.rs`, `src/types/member/mod.rs`, `src/types/mod.rs`
**Reference files to read:** `../server/src/types/member/role.rs`
**Depends on:** None

**Specification:**

Create `src/types/member/` directory. Port `role.rs` as-is from `../server/`:
- `Role` enum: Owner=1, Guardian=2, Student=4, Teacher=8, Staff=16
- `Roles` bitmask wrapper (u8) with all operator impls, `Diesel` `ToSql`/`FromSql`, `TryFrom<i32>`, `From<Roles> for i32`
- Proto conversions: leave as TODO (proto for member types doesn't exist yet)
- Port all tests

Create `src/types/member/mod.rs`:
```rust
mod role;
pub use role::*;
```

Add `pub mod member;` to `src/types/mod.rs`.

**Update after completion:**
- [ ] Mark this task `[x]`

---

### Task 06: Port Membership struct
**Files to create/modify:** `src/types/member/membership.rs`, `src/types/member/mod.rs`
**Reference files to read:** `../server/src/types/member/membership.rs`
**Depends on:** Task 05

**Specification:**

Port `Membership` struct from `../server/`:
```rust
#[derive(Debug, Clone, PartialEq, Eq, Hash, Queryable, QueryableByName)]
pub struct Membership {
    #[diesel(sql_type = Binary)]
    pub id: Id,
    #[diesel(sql_type = Text)]
    pub name: String,
    #[diesel(sql_type = Integer)]
    pub roles: Roles,
    #[diesel(sql_type = BigInt)]
    pub created: DateTime,
}
```
Proto conversion: leave as TODO.

Add `mod membership;` and `pub use membership::*;` to `src/types/member/mod.rs`.

**Update after completion:**
- [ ] Mark this task `[x]`

---

### Task 07: Port Role struct and Authorize implementation
**Files to create/modify:** `src/types/role/role.rs`, `src/db/database/authorize.rs`, `src/db/database/mod.rs`, `src/types/role/mod.rs`
**Reference files to read:** `../server/src/types/role/role.rs`, `../server/src/db/database/authorize.rs`
**Depends on:** Tasks 01, 02, 03, 04, 05

**Specification:**

**`src/types/role/role.rs`** — Port from `../server/`:
- `Role` struct with `Queryable`, `QueryableByName` derives, fields: `id: Id`, `name: String`, `permissions: Permissions`, `created: DateTime`
- `Reference` insertable struct (for `roles` table)
- `Assigner` insertable struct (for `role_assignments`/`scopes` table)
- `Assignment` struct for role assignment queries
- `Update` struct (name + additions/subtractions)
- `Role::SELECT` constant tuple
- `Role::new()`, `Role::reference()`, `Role::system()` methods
- Proto conversions: TODO
- `Offset<DateTime>` impls

Add `mod role;` and `pub use role::*;` to `src/types/role/mod.rs`.

**`src/db/database/authorize.rs`** — Port from `../server/`:
- `impl Authorize for SqliteConnection` — exact same logic: Super bypass, school active check, user active check, Owner bypass for school context, system+school role merging, permission subtraction loop
- `impl Load<(Id, &User), Role>` — load school-scoped roles
- `impl Load<&User, Role>` — load system-scoped roles
- `impl Load<(Option<Id>, &User), Role>` — combined loader
- Adapt table names to match `../ledger/` Diesel schema (e.g. `role_assignments` may be `scopes` in ledger — check `src/db/schema/schema.rs`)

Register `mod authorize;` in `src/db/database/mod.rs`.

**Update after completion:**
- [ ] Mark this task `[x]`

---

### Task 08: Update `roles.permissions` column from `text` to `blob`
**Files to create/modify:** New migration in `migrations/`, then regenerate `src/db/schema/schema.rs`
**Depends on:** Task 03

**Specification:**

Create a new Diesel migration that changes the `roles.permissions` column from `text` to `blob`. Since SQLite doesn't support `ALTER COLUMN`, this requires:

1. Create new table `roles_new` with `permissions blob NOT NULL`
2. Copy data (for existing rows, permissions will need a default empty blob `X''`)
3. Drop `roles`
4. Rename `roles_new` to `roles`
5. Recreate any indexes and triggers that referenced `roles`

Run `diesel migration run` and `diesel print-schema > src/db/schema/schema.rs` to regenerate.

Check the current migration for the exact `roles` table definition to know all columns and constraints.

**Update after completion:**
- [ ] Mark this task `[x]`

---

## Phase 1 — Proto Definitions

### Task 09: Create `protos/types/role.proto`
**Files to create/modify:** `protos/types/role.proto`
**Depends on:** Task 02 (for the enum values)

**Specification:**

```protobuf
syntax = "proto3";
package role;

enum Resource {
  USERS = 0;
  SCHOOLS = 1;
  OWNERS = 2;
  TEACHERS = 3;
  STAFF = 4;
  STUDENTS = 5;
  DEPARTMENTS = 6;
  CLASSES = 7;
  ATTENDANCE = 8;
  LESSONS = 9;
  EXAMS = 10;
  GRADES = 11;
  FEES = 12;
  PAYMENTS = 13;
  ANNOUNCEMENTS = 14;
  ROLES = 15;
  PLANS = 16;
  AI = 17;
}

enum Action {
  CREATE = 0;
  READ = 1;
  UPDATE = 2;
  DELETE = 3;
  PURGE = 4;
  ASSIGN = 5;
  UNASSIGN = 6;
  MARK = 7;
  APPROVE = 8;
}

message Permission {
  Resource resource = 1;
  repeated Action actions = 2;
}

message Role {
  string id = 1;
  string name = 2;
  repeated Permission permissions = 3;
  int64 created = 4;
}

message Assignment {
  string id = 1;
  string name = 2;
  int64 assigned = 3;
  optional string profile = 4;
}
```

Register in `build.rs` if needed (type-only protos may be auto-discovered via imports).

**Update after completion:**
- [ ] Mark this task `[x]`

---

### Task 10: Create `protos/types/member.proto`
**Files to create/modify:** `protos/types/member.proto`
**Depends on:** None

**Specification:**

```protobuf
syntax = "proto3";
package member;

enum Role {
  OWNER = 0;
  GUARDIAN = 1;
  STUDENT = 2;
  TEACHER = 3;
  STAFF = 4;
}

message Membership {
  string id = 1;
  string name = 2;
  repeated Role roles = 3;
  optional string logo = 4;
  int64 created = 5;
}
```

**Update after completion:**
- [ ] Mark this task `[x]`

---

### Task 11: Create `protos/services/sync.proto`
**Files to create/modify:** `protos/services/sync.proto`, `build.rs`
**Depends on:** Task 09

**Specification:**

Create `protos/services/sync.proto` with:

```protobuf
syntax = "proto3";
package sync;

import "types/role.proto";

service Sync {
  rpc PushChanges (stream MutationBatch) returns (stream PushAck);
  rpc WatchChanges (WatchRequest) returns (stream SyncDelta);
}

message MutationBatch {
  string batch_id = 1;
  repeated Mutation mutations = 2;
}

message Mutation {
  int32 table = 1;
  int32 operation = 2;
  string row_key = 3;
  optional int32 columns = 4;
  RowData data = 5;
}

message PushAck {
  string batch_id = 1;
  bool success = 2;
  optional string error = 3;
  int64 server_seq = 4;
  repeated MutationResult results = 5;
}

message MutationResult {
  int32 index = 1;
  bool success = 2;
  optional string error = 3;
  int32 code = 4;
  repeated FileUrl file_urls = 5;
}

message WatchRequest {
  int64 last_seq = 1;
}

message SyncDelta {
  int64 seq = 1;
  int32 table = 2;
  int32 operation = 3;
  string row_key = 4;
  RowData data = 5;
  repeated FileUrl file_urls = 6;
}

message FileUrl {
  string path = 1;
  optional string put_url = 2;
  optional string get_url = 3;
  int64 expiry = 4;
}

// RowData = oneof with 30 per-table *Row messages.
// Each *Row message mirrors the SQL schema columns as optional fields.
// The full message definitions will be fleshed out when implementing the sync handlers.
message RowData {
  oneof row {
    UserRow user = 1;
    SchoolRow school = 2;
    OwnerRow owner = 3;
    StudentRow student = 4;
    GuardianRow guardian = 5;
    DepartmentRow department = 6;
    TeacherRow teacher = 7;
    StaffRow staff_member = 8;
    TermRow term = 9;
    ClassTeacherRow class_teacher = 10;
    EnrollmentRow enrollment = 11;
    SubjectRow subject = 12;
    AttendanceRow attendance = 13;
    TimetableRow timetable = 14;
    LessonRow lesson = 15;
    ExamRow exam = 16;
    PaperRow paper = 17;
    GradeRow grade = 18;
    FeeRow fee = 19;
    InvoiceRow invoice = 20;
    PaymentRow payment = 21;
    AnnouncementRow announcement = 22;
    MasteryRow mastery = 23;
    AiUsageRow ai_usage = 24;
    SettingsRow settings = 25;
    RoleRow role = 26;
    ScopeRow scope = 27;
    PlanRow plan = 28;
    SubscriptionRow subscription = 29;
    DiscountRow discount = 30;
  }
}

// Stub messages — columns to be filled in based on schema.sql
message UserRow { string id = 1; /* remaining columns TBD */ }
message SchoolRow { string id = 1; }
message OwnerRow { string school = 1; string user = 2; }
message StudentRow { string school = 1; int32 adm = 2; }
message GuardianRow { string school = 1; int32 adm = 2; string user = 3; }
message DepartmentRow { string school = 1; string id = 2; }
message TeacherRow { string school = 1; string user = 2; }
message StaffRow { string school = 1; string user = 2; }
message TermRow { string school = 1; int32 year = 2; int32 term = 3; }
message ClassTeacherRow { string school = 1; int32 year = 2; int32 term = 3; int32 grade = 4; int32 stream = 5; string teacher = 6; }
message EnrollmentRow { string school = 1; int32 year = 2; int32 adm = 3; }
message SubjectRow { string school = 1; int32 year = 2; int32 term = 3; int32 grade = 4; int32 index = 5; }
message AttendanceRow { string school = 1; int32 adm = 2; int32 date = 3; }
message TimetableRow { string school = 1; int32 year = 2; int32 term = 3; int32 grade = 4; int32 stream = 5; int32 day = 6; int32 period = 7; }
message LessonRow { string school = 1; string id = 2; }
message ExamRow { string school = 1; string id = 2; }
message PaperRow { string school = 1; string exam = 2; int32 grade = 3; int32 subject = 4; optional int32 paper = 5; }
message GradeRow { string school = 1; string exam = 2; int32 student = 3; int32 subject = 4; optional int32 paper = 5; }
message FeeRow { string school = 1; string id = 2; }
message InvoiceRow { string school = 1; string id = 2; }
message PaymentRow { string school = 1; string id = 2; }
message AnnouncementRow { string school = 1; string id = 2; }
message MasteryRow { string school = 1; int32 adm = 2; int32 grade = 3; int32 subject = 4; }
message AiUsageRow { string school = 1; string id = 2; }
message SettingsRow { string school = 1; }
message RoleRow { string id = 1; }
message ScopeRow { string role = 1; string user = 2; optional string school = 3; }
message PlanRow { string id = 1; }
message SubscriptionRow { string school = 1; string plan = 2; }
message DiscountRow { string id = 1; }
```

Register in `build.rs`: add `"protos/services/sync.proto"` to the `.compile_protos()` call.

**Update after completion:**
- [ ] Mark this task `[x]`

---

### Task 12: Wire proto conversions for Role/Permissions/Actions/Resource types
**Files to create/modify:** `src/types/role/action.rs`, `src/types/role/actions.rs`, `src/types/role/resource.rs`, `src/types/role/permissions.rs`, `src/types/member/role.rs`, `src/proto/types/mod.rs`
**Depends on:** Tasks 02, 03, 05, 09, 10

**Specification:**

Add `pub mod role { tonic::include_proto!("role"); }` and `pub mod member { tonic::include_proto!("member"); }` to `src/proto/types/mod.rs`.

Then add all proto conversion impls that were left as TODO in Tasks 02-06:
- `From<proto::Action> for Action` and vice versa (9 variants)
- `From<proto::Resource> for Resource` and vice versa (18 variants)
- `TryFrom<i32> for Resource` via proto
- `From<Actions> for Vec<i32>` (proto action list)
- `From<Permissions> for Vec<proto::Permission>` and `TryFrom<&[proto::Permission]> for Permissions`
- `From<member::Role> for Role` and vice versa for member roles
- `From<Roles> for Vec<i32>` (proto role list)

Follow exact patterns from `../server/` source files.

**Update after completion:**
- [ ] Mark this task `[x]`

---

## Phase 2 — Sync Engine

### Task 13: Create `server_logs` migration + Diesel model
**Files to create/modify:** New migration, `src/db/schema/schema.rs` (regenerated), `src/types/server_log.rs`, `src/types/mod.rs`
**Depends on:** Task 08

**Specification:**

Create migration `up.sql`:
```sql
CREATE TABLE server_logs (
    seq       INTEGER PRIMARY KEY AUTOINCREMENT,
    user_id   TEXT    NOT NULL,
    tbl       SMALLINT NOT NULL,
    op        SMALLINT NOT NULL,
    row_key   TEXT    NOT NULL,
    row_data  TEXT,
    school_id TEXT,
    created   BIGINT  NOT NULL DEFAULT (unixepoch('now'))
);
```

Run `diesel migration run` + `diesel print-schema > src/db/schema/schema.rs`.

Create `src/types/server_log.rs`:
```rust
// ServerLog struct with Queryable, Insertable derives
// Fields matching the table columns
```

**Update after completion:**
- [ ] Mark this task `[x]`

---

### Task 14: Implement PushChanges handler
**Files to create/modify:** `src/proto/services/sync.rs`, `src/services/sync.rs`, `src/db/database/tables/server_logs.rs`
**Depends on:** Tasks 11, 13

**Specification:**

1. Create proto service trait in `src/proto/services/sync.rs`:
   - Clean `Sync` trait with `push_changes` and `watch_changes` methods
   - Tonic adapter impl

2. Implement `src/services/sync.rs`:
   - `push_changes`: receive `MutationBatch` stream, for each batch:
     - Validate user permissions for each mutation
     - Apply mutation to the target table
     - Log to `server_logs`
     - Return `PushAck` with per-mutation results

3. `src/db/database/tables/server_logs.rs`:
   - `Create<ServerLog>` impl for inserting log entries

4. **User creation validation** (in push handler):
   - ALL user Inserts MUST have `status = Invited` → reject with code 3 otherwise
   - `level = Normal` → allow (anyone can invite normal users)
   - `level = System` → require `Users.Create` permission → reject with code 1 if missing
   - `level = Super` → require pusher is Super → reject with code 1 otherwise
   - Server only accepts `phone` + `name` from client, sets defaults for all other fields

5. **Invitation-aware member creation** (phone conflict resolution):
   When processing an Insert on a member table (`owners`, `teachers`, `staff`, `students`, `guardians`):
   - Check if the batch also contains a user Insert that the member's `user` field references
   - If yes → this is an invitation flow:
     a. Try to insert the user
     b. If phone conflict (UNIQUE constraint violation):
        - Look up existing user by phone
        - If existing user `status == Deleted` → reject with code 4
        - Rewrite the member's `user` field to the existing user's ID
        - Insert the member with the corrected user ID
        - Return code 2 (conflict) for the user mutation
        - Return code 0 (success) for the member mutation with corrected row data
        - Log to `server_logs`:
          * `Delete` on `users` table for the orphaned user ID (all clients clean up)
          * `Insert` on the member table with the corrected user field
   - If no → normal member creation, just validate and insert

**Update after completion:**
- [ ] Mark this task `[x]`

---

### Task 15: Implement WatchChanges handler
**Files to create/modify:** `src/services/sync.rs`
**Depends on:** Task 14

**Specification:**

Implement `watch_changes`:
- Receive `WatchRequest` with `last_seq`
- Query `server_logs WHERE seq > last_seq`
- Filter by user permissions (see AGENT.md — Sync Permission Filtering)
- Stream `SyncDelta` messages
- Keep connection open, poll for new entries periodically

**Update after completion:**
- [ ] Mark this task `[x]`

---

### Task 16: Implement permission filtering for sync
**Files to create/modify:** `src/services/sync.rs`
**Depends on:** Tasks 07, 14, 15

**Specification:**

Apply the three-tier permission model to sync:
- **Super:** no filtering
- **System:** filter by resources the user has Read access on (globally)
- **Normal:** filter by school membership + own user row + all plans

**Update after completion:**
- [ ] Mark this task `[x]`

---

