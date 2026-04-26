# Authorization Improvement Tasks

## Background & Goal

The current authorization in `src/services/sync.rs` has a critical gap: `Level::Normal`
users bypass all permission checks with `Ok(())`. The infrastructure to do this correctly
already exists in `src/db/database/authorize.rs` — it just isn't wired into the sync push
path. These tasks wire it in, with no proto changes and no frontend changes required.

The full `Authorize` trait impl in `authorize.rs` already handles:
- Super user bypass
- School owner bypass (owners can do anything within their school)
- School active-status check
- School-scoped role loading
- System-scoped role merging for System users
- `check_permissions` via `required - granted` subtraction

All we need to do is:
1. Determine the `Organisation` context from each action's payload (new function)
2. Build the required `Permissions` from the already-existing `action_permission()` result
3. Call a new `authorize_user()` function that takes an already-loaded `&User` instead
   of a `Token` (avoiding a redundant DB fetch) and delegates to the existing logic
4. Replace the broken `check_action_permission` call in `process_action` with the above

---

## Reference: Key Files

| File | Role |
|---|---|
| `src/services/sync.rs` | Contains `process_action` and `check_action_permission` — the entry point being fixed |
| `src/db/database/authorize.rs` | Full `Authorize` trait impl — logic to be reused |
| `src/db/database/tables/actions.rs` | Contains `action_permission()`, `execute_action()`, `decode<T>()` — new function goes here |
| `src/db/database/traits.rs` | `Authorize`, `Load`, `Database` trait definitions |
| `src/types/role/permissions.rs` | `Permissions` struct — index by `Resource`, operators `+`/`-` |
| `src/types/role/actions.rs` | `Actions` bitmask — `Actions::from(action)`, `contains()` |
| `src/types/role/organisation.rs` | `Organisation` enum: `System`, `Account`, `School(Id)` |
| `protos/services/sync.proto` | Payload message field layout (documented inline in tasks below) |

---

## Inlined Proto Payload Reference

Critical payload field layouts (verified from sync.proto):

```
// School IS field 1 — decode with SchoolField { school: String @1 }
CreateTeacher, UpdateTeacher, DeleteTeacher
CreateStaff,   UpdateStaff,   DeleteStaff
CreateOwner,   DeleteOwner
CreateStudent, UpdateStudent, DeleteStudent
EnrollStudent, UnenrollStudent
CreateGuardian, UpdateGuardian, DeleteGuardian
CreateDepartment, UpdateDepartment, DeleteDepartment
CreateTerm, UpdateTerm, DeleteTerm
AssignClassTeacher, UnassignClassTeacher
AssignSubject, UnassignSubject
CreateTimetableEntry, UpdateTimetableEntry, DeleteTimetableEntry
MarkAttendance, DeleteAttendance
CreateLesson, DeleteLesson
CreatePaper, UpdatePaper, DeletePaper
MarkGrades, UpdateGrade, DeleteGrade, UpdateMastery
CreateStream, UpdateStream, DeleteStream
CreateMpesa, UpdateMpesa, DeleteMpesa
CreateSubscription, UpdateSubscription, DeleteSubscription
CreateDiscount, UpdateDiscount, DeleteDiscount
UpdateAiUsage, UpdateSettings

// School IS field 2, id is field 1 — decode with IdSchoolField { id: String @1, school: String @2 }
CreateExam    { id @1, school @2, ... }
CreateFee     { id @1, school @2, ... }
CreateInvoice { id @1, school @2, ... }
CreateAnnouncement { id @1, school @2, ... }

// School IS the id (record being operated on IS the school)
// decode with IdField { id: String @1 } and parse id as Organisation::School(id)
UpdateSchool  { id @1, ... }
DeleteSchool  { id @1 }

// Pure system-level — no school context, no decode needed
CREATE_SCHOOL         (creating a school is a system operation)
CREATE_PLAN, UPDATE_PLAN, DELETE_PLAN
CREATE_SUBJECT, UPDATE_SUBJECT, DELETE_SUBJECT
CREATE_TOPIC,   UPDATE_TOPIC,   DELETE_TOPIC
UPDATE_ROLE, DELETE_ROLE      (role catalog management)

// Account-level — user editing their own record
// decode with IdField { id: String @1 } and compare to user.id
UPDATE_USER

// System-level user management
DELETE_USER

// Role-scoped: optional school at field 1
// decode with OptionalSchoolField { school: Option<String> @1 }
// if Some(school) → Organisation::School(school), else → Organisation::System
ASSIGN_ROLE, UNASSIGN_ROLE

// CreateRole: optional school at field 2
// decode with CreateRoleField { id: String @1, school: Option<String> @2 }
// if Some(school) → Organisation::School(school), else → Organisation::System
CREATE_ROLE

// CreatePayment: optional school at field 3
// decode with CreatePaymentField { id: String @1, invoice: Option<String> @2, school: Option<String> @3 }
// if Some(school) → Organisation::School(school), else → Organisation::System
CREATE_PAYMENT

// DB lookup required — id is field 1 but school is not in payload
// must query the database to find the school for this record
UPDATE_EXAM,   DELETE_EXAM         → SELECT school FROM exams WHERE id = ?
UPDATE_FEE,    DELETE_FEE          → SELECT school FROM fees WHERE id = ?
UPDATE_INVOICE, DELETE_INVOICE     → SELECT school FROM invoices WHERE id = ?
UPDATE_PAYMENT, DELETE_PAYMENT,
APPROVE_PAYMENT                    → SELECT school FROM payments WHERE id = ?
UPDATE_ANNOUNCEMENT, DELETE_ANNOUNCEMENT → SELECT school FROM announcements WHERE id = ?

// File sync — school is field 1 in scheme/answer sheet payloads
UPLOAD_SCHEME, DELETE_SCHEME
UPLOAD_ANSWER_SHEET, DELETE_ANSWER_SHEET
```

---

## Inlined Code Reference: Current `process_action` and `check_action_permission`

From `src/services/sync.rs` (lines 310–396):

```rust
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
        Ok(action_result) => ActionResponse { ... },
        Err(e) => { ... }
    }
}

fn check_action_permission(
    conn: &mut diesel::SqliteConnection,
    user: &User,
    resource: Resource,
    action: Action,
) -> Result<()> {
    match user.level {
        Level::Super => Ok(()),
        Level::System => {
            let roles: Vec<Role> = Load::<&User, Role>::load(conn, &user)?;
            let mut granted = Permissions::new();
            for role in &roles {
                granted += role.permissions;
            }
            if granted[resource].contains(action) {
                Ok(())
            } else {
                warn!(...);
                Err(Error::Forbidden)
            }
        }
        Level::Normal => {
            // BUG: allows everything
            Ok(())
        }
    }
}
```

---

## Inlined Code Reference: `authorize.rs` private helpers to make public

From `src/db/database/authorize.rs` (bottom of file):

```rust
fn aggregate_permissions(roles: &[Role]) -> Permissions {
    let mut granted = Permissions::new();
    for role in roles {
        granted += role.permissions;
    }
    granted
}

fn check_permissions(required: Permissions, granted: Permissions) -> Result<()> {
    let remaining = required - granted;
    if remaining.is_empty() {
        Ok(())
    } else {
        warn!("authorize: forbidden — required permissions not satisfied");
        Err(Error::Forbidden)
    }
}
```

Both are currently `fn` (private). They need to become `pub fn` so `authorize_user` can be
a standalone public function called from `sync.rs`.

---

## Inlined Code Reference: `Authorize::authorize` School branch (the logic to reuse)

From `src/db/database/authorize.rs` `Organisation::School` branch:

```rust
Organisation::School(school_id) => {
    let school_status: i16 = schools::table
        .find(school_id)
        .select(schools::status)
        .first(self)
        .optional()?
        .ok_or(Error::SchoolNotFound)?;

    if school_status != 1 {
        return Err(Error::Forbidden);
    }

    let is_owner: bool = owners::table
        .filter(owners::school.eq(school_id))
        .filter(owners::user.eq(user.id))
        .first::<(Id, Id, i64)>(self)
        .optional()?
        .is_some();

    if is_owner {
        return Ok(());
    }

    let mut roles: Vec<Role> = self.load((school_id, &user))?;

    if user.level == Level::System {
        let system_roles: Vec<Role> = self.load(&user)?;
        roles.extend(system_roles);
    }

    let granted = aggregate_permissions(&roles);
    check_permissions(permissions, granted)
}
```

The new `authorize_user` function duplicates this logic but takes `user: &User` directly
instead of loading it from `token.user`, and skips the `user.status` check (already done
upstream in `push_actions`).

---

## Task 01 — Add `action_organisation` to `actions.rs`

**Files to modify:** `src/db/database/tables/actions.rs`
**Reference files:** `protos/services/sync.proto` (payload layouts above), `src/types/role/organisation.rs`
**Depends on:** nothing
**Parallel group:** P1

### Specification

Add the following at the bottom of `src/db/database/tables/actions.rs`, after
`action_permission` and before the `decode` helper.

**Step 1 — Add helper structs for payload decoding**

Add these prost structs near the top of the file (after the existing imports). They decode
only the fields needed for school extraction; other fields are ignored by prost:

```rust
/// Decodes a payload whose first field is the school ID string.
/// Used by the majority of school-scoped actions.
#[derive(prost::Message)]
struct SchoolField {
    #[prost(string, tag = "1")]
    school: String,
}

/// Decodes a payload whose first field is an opaque ID and second field is school.
/// Used by: CreateExam, CreateFee, CreateInvoice, CreateAnnouncement.
#[derive(prost::Message)]
struct IdSchoolField {
    #[prost(string, tag = "1")]
    id: String,
    #[prost(string, tag = "2")]
    school: String,
}

/// Decodes a payload whose only relevant field is an ID string at field 1.
/// Used for UpdateSchool/DeleteSchool (id IS the school) and UPDATE_USER/DELETE_USER.
#[derive(prost::Message)]
struct IdField {
    #[prost(string, tag = "1")]
    id: String,
}

/// Decodes payloads where school is an optional field at position 1.
/// Used by: AssignRole, UnassignRole.
#[derive(prost::Message)]
struct OptionalSchoolField {
    #[prost(string, optional, tag = "1")]
    school: Option<String>,
}

/// Decodes CreateRolePayload where school is optional at position 2.
#[derive(prost::Message)]
struct CreateRoleField {
    #[prost(string, tag = "1")]
    id: String,
    #[prost(string, optional, tag = "2")]
    school: Option<String>,
}

/// Decodes CreatePaymentPayload where school is optional at position 3.
#[derive(prost::Message)]
struct CreatePaymentField {
    #[prost(string, tag = "1")]
    id: String,
    #[prost(string, optional, tag = "2")]
    invoice: Option<String>,
    #[prost(string, optional, tag = "3")]
    school: Option<String>,
}
```

**Step 2 — Add DB lookup helpers**

Add these private helpers after the `decode` function and before the existing
`TBL_*` constants:

```rust
/// Look up the school that owns a record in a table keyed by text `id`.
/// Used for auth on update/delete payloads that carry only an ID.
fn school_for_exam(conn: &mut Conn, id: &str) -> Result<Id> {
    sql_query("SELECT school FROM exams WHERE id = ?")
        .bind::<Text, _>(id)
        .load::<SchoolIdRow>(conn)
        .map_err(|e| { tracing::error!("school_for_exam: {e}"); Error::Internal })?
        .into_iter().next()
        .map(|r| r.school)
        .ok_or(Error::Internal)
}

fn school_for_fee(conn: &mut Conn, id: &str) -> Result<Id> {
    sql_query("SELECT school FROM fees WHERE id = ?")
        .bind::<Text, _>(id)
        .load::<SchoolIdRow>(conn)
        .map_err(|e| { tracing::error!("school_for_fee: {e}"); Error::Internal })?
        .into_iter().next()
        .map(|r| r.school)
        .ok_or(Error::Internal)
}

fn school_for_invoice(conn: &mut Conn, id: &str) -> Result<Id> {
    sql_query("SELECT school FROM invoices WHERE id = ?")
        .bind::<Text, _>(id)
        .load::<SchoolIdRow>(conn)
        .map_err(|e| { tracing::error!("school_for_invoice: {e}"); Error::Internal })?
        .into_iter().next()
        .map(|r| r.school)
        .ok_or(Error::Internal)
}

fn school_for_payment(conn: &mut Conn, id: &str) -> Result<Id> {
    sql_query("SELECT school FROM payments WHERE id = ?")
        .bind::<Text, _>(id)
        .load::<SchoolIdRow>(conn)
        .map_err(|e| { tracing::error!("school_for_payment: {e}"); Error::Internal })?
        .into_iter().next()
        .map(|r| r.school)
        .ok_or(Error::Internal)
}

fn school_for_announcement(conn: &mut Conn, id: &str) -> Result<Id> {
    sql_query("SELECT school FROM announcements WHERE id = ?")
        .bind::<Text, _>(id)
        .load::<SchoolIdRow>(conn)
        .map_err(|e| { tracing::error!("school_for_announcement: {e}"); Error::Internal })?
        .into_iter().next()
        .map(|r| r.school)
        .ok_or(Error::Internal)
}
```

Also add the `SchoolIdRow` QueryableByName struct near the other row structs (or near the
top of the file with the other row types). Check `src/db/database/tables/rows.rs` for the
pattern — it uses `#[derive(QueryableByName)]` and `#[diesel(sql_type = Text)]`:

```rust
#[derive(diesel::QueryableByName)]
struct SchoolIdRow {
    #[diesel(sql_type = diesel::sql_types::Text)]
    school: Id,
}
```

**Step 3 — Add `action_organisation` function**

Add this public function immediately after `action_permission`:

```rust
/// Determine the `Organisation` context for an action.
///
/// Returns the organisation scope that authorization should be checked against:
/// - `Organisation::System`      — system/super only operations
/// - `Organisation::Account`     — user editing their own account
/// - `Organisation::School(id)`  — school-scoped operation; `id` is the school
///
/// `user_id` is the ID of the already-authenticated user, needed to distinguish
/// UPDATE_USER on own record (Account) from updating another user (System).
pub fn action_organisation(
    conn: &mut Conn,
    action_id: i32,
    user_id: Id,
    payload: &[u8],
) -> Result<Organisation> {
    use sync_action::*;

    // Helper: decode school from field 1 and parse as Id
    let school_from_field1 = |payload: &[u8]| -> Result<Organisation> {
        let p: SchoolField = decode(payload)?;
        let id: Id = p.school.parse().map_err(|_| {
            tracing::error!("action_organisation: invalid school id in field 1");
            Error::Internal
        })?;
        Ok(Organisation::School(id))
    };

    match action_id {
        // ── Pure system-level: only System/Super users ──────────────────────
        CREATE_SCHOOL
        | CREATE_PLAN | UPDATE_PLAN | DELETE_PLAN
        | CREATE_SUBJECT | UPDATE_SUBJECT | DELETE_SUBJECT
        | CREATE_TOPIC   | UPDATE_TOPIC   | DELETE_TOPIC
        | UPDATE_ROLE    | DELETE_ROLE
        | DELETE_USER => Ok(Organisation::System),

        // ── Account: user editing their own profile ──────────────────────────
        // If the payload's id matches the acting user → Account.
        // If it's someone else's id → must be System/Super.
        UPDATE_USER => {
            let p: IdField = decode(payload)?;
            if p.id.parse::<Id>().ok() == Some(user_id) {
                Ok(Organisation::Account)
            } else {
                Ok(Organisation::System)
            }
        }

        // ── School IS the id (UpdateSchool / DeleteSchool) ───────────────────
        UPDATE_SCHOOL | DELETE_SCHOOL => {
            let p: IdField = decode(payload)?;
            let id: Id = p.id.parse().map_err(|_| {
                tracing::error!("action_organisation: invalid school id for UPDATE/DELETE_SCHOOL");
                Error::Internal
            })?;
            Ok(Organisation::School(id))
        }

        // ── School at field 1 (the common case) ─────────────────────────────
        CREATE_TEACHER | UPDATE_TEACHER | DELETE_TEACHER
        | CREATE_STAFF  | UPDATE_STAFF  | DELETE_STAFF
        | CREATE_OWNER  | DELETE_OWNER
        | CREATE_STUDENT | UPDATE_STUDENT | DELETE_STUDENT
        | ENROLL_STUDENT | UNENROLL_STUDENT
        | CREATE_GUARDIAN | UPDATE_GUARDIAN | DELETE_GUARDIAN
        | CREATE_DEPARTMENT | UPDATE_DEPARTMENT | DELETE_DEPARTMENT
        | CREATE_TERM | UPDATE_TERM | DELETE_TERM
        | ASSIGN_CLASS_TEACHER | UNASSIGN_CLASS_TEACHER
        | ASSIGN_SUBJECT | UNASSIGN_SUBJECT
        | CREATE_TIMETABLE_ENTRY | UPDATE_TIMETABLE_ENTRY | DELETE_TIMETABLE_ENTRY
        | MARK_ATTENDANCE | DELETE_ATTENDANCE
        | CREATE_LESSON | DELETE_LESSON
        | CREATE_PAPER  | UPDATE_PAPER  | DELETE_PAPER
        | MARK_GRADES   | UPDATE_GRADE  | DELETE_GRADE | UPDATE_MASTERY
        | CREATE_STREAM | UPDATE_STREAM | DELETE_STREAM
        | CREATE_MPESA  | UPDATE_MPESA  | DELETE_MPESA
        | CREATE_SUBSCRIPTION | UPDATE_SUBSCRIPTION | DELETE_SUBSCRIPTION
        | CREATE_DISCOUNT | UPDATE_DISCOUNT | DELETE_DISCOUNT
        | UPDATE_AI_USAGE | UPDATE_SETTINGS
        | UPLOAD_SCHEME | DELETE_SCHEME
        | UPLOAD_ANSWER_SHEET | DELETE_ANSWER_SHEET => school_from_field1(payload),

        // ── School at field 2 (id is field 1) ───────────────────────────────
        CREATE_EXAM | CREATE_FEE | CREATE_INVOICE | CREATE_ANNOUNCEMENT => {
            let p: IdSchoolField = decode(payload)?;
            let id: Id = p.school.parse().map_err(|_| {
                tracing::error!("action_organisation: invalid school id in field 2");
                Error::Internal
            })?;
            Ok(Organisation::School(id))
        }

        // ── CreateRole: optional school at field 2 ───────────────────────────
        CREATE_ROLE => {
            let p: CreateRoleField = decode(payload)?;
            match p.school {
                Some(s) if !s.is_empty() => {
                    let id: Id = s.parse().map_err(|_| {
                        tracing::error!("action_organisation: invalid school id in CreateRole");
                        Error::Internal
                    })?;
                    Ok(Organisation::School(id))
                }
                _ => Ok(Organisation::System),
            }
        }

        // ── AssignRole / UnassignRole: optional school at field 1 ────────────
        ASSIGN_ROLE | UNASSIGN_ROLE => {
            let p: OptionalSchoolField = decode(payload)?;
            match p.school {
                Some(s) if !s.is_empty() => {
                    let id: Id = s.parse().map_err(|_| {
                        tracing::error!("action_organisation: invalid school id in role assignment");
                        Error::Internal
                    })?;
                    Ok(Organisation::School(id))
                }
                _ => Ok(Organisation::System),
            }
        }

        // ── CreatePayment: optional school at field 3 ────────────────────────
        CREATE_PAYMENT => {
            let p: CreatePaymentField = decode(payload)?;
            match p.school {
                Some(s) if !s.is_empty() => {
                    let id: Id = s.parse().map_err(|_| {
                        tracing::error!("action_organisation: invalid school id in CreatePayment");
                        Error::Internal
                    })?;
                    Ok(Organisation::School(id))
                }
                _ => Ok(Organisation::System),
            }
        }

        // ── DB lookup required ───────────────────────────────────────────────
        UPDATE_EXAM | DELETE_EXAM => {
            let p: IdField = decode(payload)?;
            Ok(Organisation::School(school_for_exam(conn, &p.id)?))
        }

        UPDATE_FEE | DELETE_FEE => {
            let p: IdField = decode(payload)?;
            Ok(Organisation::School(school_for_fee(conn, &p.id)?))
        }

        UPDATE_INVOICE | DELETE_INVOICE => {
            let p: IdField = decode(payload)?;
            Ok(Organisation::School(school_for_invoice(conn, &p.id)?))
        }

        UPDATE_PAYMENT | DELETE_PAYMENT | APPROVE_PAYMENT => {
            let p: IdField = decode(payload)?;
            Ok(Organisation::School(school_for_payment(conn, &p.id)?))
        }

        UPDATE_ANNOUNCEMENT | DELETE_ANNOUNCEMENT => {
            let p: IdField = decode(payload)?;
            Ok(Organisation::School(school_for_announcement(conn, &p.id)?))
        }

        _ => {
            tracing::error!("action_organisation: unknown action {action_id}");
            Err(Error::Internal)
        }
    }
}
```

Also add `use crate::types::role::Organisation;` to the imports at the top of `actions.rs`.

### Update after completion
- [x] Mark this task `[x]`
- [ ] Orchestrator: do NOT commit yet — wait for Task 02 and 03

---

## Task 02 — Add `authorize_user` to `authorize.rs`

**Files to modify:** `src/db/database/authorize.rs`
**Reference files:** (all logic inlined above in this document)
**Depends on:** nothing (parallel with Task 01)
**Parallel group:** P1

### Specification

**Step 1 — Make helpers public**

Change the two private functions at the bottom of `authorize.rs` from `fn` to `pub fn`:

```rust
pub fn aggregate_permissions(roles: &[Role]) -> Permissions { ... }
pub fn check_permissions(required: Permissions, granted: Permissions) -> Result<()> { ... }
```

**Step 2 — Add `authorize_user`**

Add this public function to `authorize.rs`. It mirrors `Authorize::authorize` but takes an
already-loaded `&User` instead of a `Token`, skipping the redundant DB fetch. The `status`
check is omitted because `push_actions` already verified it upstream.

```rust
/// Authorize an action for an already-loaded user against an organisation context.
///
/// This is the push-path equivalent of `Authorize::authorize` that avoids
/// re-fetching the user from the database (the caller already has the `User`).
///
/// - Super users bypass all checks immediately.
/// - `Organisation::Account` allows any active user (ownership validated by the
///   action handler itself).
/// - `Organisation::System` requires `Level::System` or above and the user must
///   hold system-scoped roles that cover the required permissions.
/// - `Organisation::School(id)` verifies school is active, grants owners a bypass,
///   loads school-scoped roles (+ system roles for System users), and checks
///   required permissions against the aggregate.
pub fn authorize_user(
    conn: &mut Conn,
    user: &User,
    organisation: Organisation,
    permissions: Permissions,
) -> Result<()> {
    // Super bypass
    if user.level == Level::Super {
        debug!(user_id = %user.id, "authorize_user: super bypass");
        return Ok(());
    }

    match organisation {
        Organisation::System => {
            if user.level != Level::System {
                warn!(user_id = %user.id, "authorize_user: non-system user attempted system op");
                return Err(Error::Forbidden);
            }
            let roles: Vec<Role> = conn.load(&user)?;
            let granted = aggregate_permissions(&roles);
            check_permissions(permissions, granted)
        }

        Organisation::Account => {
            // The action handler validates that the user can only mutate their own record.
            Ok(())
        }

        Organisation::School(school_id) => {
            // Verify the school exists and is active (status = 1)
            let school_status: i16 = schools::table
                .find(school_id)
                .select(schools::status)
                .first(conn)
                .optional()?
                .ok_or(Error::SchoolNotFound)?;

            if school_status != 1 {
                warn!(
                    user_id = %user.id,
                    school_id = %school_id,
                    "authorize_user: school is not active"
                );
                return Err(Error::Forbidden);
            }

            // School owners bypass all permission checks within their school
            let is_owner: bool = owners::table
                .filter(owners::school.eq(school_id))
                .filter(owners::user.eq(user.id))
                .first::<(Id, Id, i64)>(conn)
                .optional()?
                .is_some();

            if is_owner {
                debug!(
                    user_id = %user.id,
                    school_id = %school_id,
                    "authorize_user: school owner bypass"
                );
                return Ok(());
            }

            // Load school-scoped roles
            let mut roles: Vec<Role> = conn.load((school_id, user))?;

            // System users also get their system-scoped roles merged in
            if user.level == Level::System {
                let system_roles: Vec<Role> = conn.load(user)?;
                roles.extend(system_roles);
            }

            let granted = aggregate_permissions(&roles);
            check_permissions(permissions, granted)
        }
    }
}
```

Ensure `src/db/database/authorize.rs` has all necessary imports:
- `use crate::types::role::{Organisation, Permissions, Role};` (already present)
- `use crate::db::database::traits::Load;` (already present)
- `use tracing::{debug, warn};` (already present)

### Update after completion
- [x] Mark this task `[x]`
- [ ] Orchestrator: do NOT commit yet — wait for Task 03

---

## Task 03 — Refactor `process_action` in `sync.rs`

**Files to modify:** `src/services/sync.rs`
**Reference files:** (all relevant code inlined above)
**Depends on:** Task 01 AND Task 02 must be complete
**Parallel group:** P2 (sequential after P1)

### Specification

**Step 1 — Add import for `authorize_user`**

Add to the imports at the top of `src/services/sync.rs`:

```rust
use crate::db::database::authorize::authorize_user;
use crate::types::role::{Actions, Permissions};
```

**Step 2 — Replace `process_action` with new version**

Replace the entire `process_action` function with:

```rust
fn process_action(user: &User, request: &ActionRequest) -> ActionResponse {
    let result = CONN.with(|cell| {
        let conn = &mut *cell.borrow_mut();

        // 1. Map action → required (Resource, Action)
        let (resource, action) = actions::action_permission(request.action)?;

        // 2. Build a Permissions value containing only the one required action
        let mut required = Permissions::new();
        required[resource] = Actions::from(action);

        // 3. Determine the organisation context from the payload
        let organisation =
            actions::action_organisation(conn, request.action, user.id, &request.payload)?;

        // 4. Full authorization — Super bypass, owner bypass, role check
        authorize_user(conn, user, organisation, required)?;

        // 5. Execute inside a transaction
        conn.transaction(|conn| {
            actions::execute_action(conn, request.action, &request.payload)
        })
    });

    match result {
        Ok(action_result) => ActionResponse {
            id: request.id.clone(),
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
                id: request.id.clone(),
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

**Step 3 — Delete `check_action_permission`**

Remove the entire `check_action_permission` function. It is fully replaced by the
`authorize_user` call above.

**Step 4 — Remove now-unused imports from `sync.rs`**

After the refactor, the following may no longer be used — remove them if the compiler
reports them as unused:

```rust
use crate::types::role::{Action, Resource};  // replaced by Actions, Permissions
```

Keep:
```rust
use crate::types::role::{Actions, Permissions, Role};
```
`Role` is still used in `SyncFilter::build`.

### Update after completion
- [x] Mark this task `[x]`
- [ ] Orchestrator: do NOT commit yet — wait for Task 04

---

## Task 04 — Verify and test

**Files to modify:** `src/services/sync.rs` (test module), `src/db/database/authorize.rs` (test module)
**Depends on:** Tasks 01, 02, 03 complete
**Parallel group:** P3

### Specification

**Step 1 — Run the build**

```sh
cargo build 2>&1
```

Fix any compilation errors. Common issues to anticipate:
- `SchoolIdRow` — ensure it implements `QueryableByName` and uses the correct Diesel sql type
  for the `Id` type (which implements `FromSqlRow`). If `Id` gives trouble, use `String`
  and call `.parse::<Id>()` inside the helper.
- Missing `use` for `Organisation` in `actions.rs` — add it.
- The prost `#[derive(prost::Message)]` structs — confirm `prost` is available. It is
  available transitively via `tonic`. If the compiler can't find it, add `use prost::Message`
  or `prost::Message` explicitly in the decode bound.

**Step 2 — Run existing tests**

```sh
cargo test 2>&1
```

All existing tests must still pass.

**Step 3 — Add authorization tests to `sync.rs`**

Add to the `#[cfg(test)] mod tests` block at the bottom of `src/services/sync.rs`:

```rust
#[cfg(test)]
mod tests {
    // ... existing tests ...

    // These tests verify the action_organisation function categorizes correctly.
    // They do not require a real DB connection for the non-lookup cases.

    use crate::db::database::tables::actions::{action_organisation, sync_action};
    use crate::types::role::Organisation;
    use crate::types::id::Id;

    fn dummy_id() -> Id {
        "683d5a1b4f2e7c0019abcdef".parse().unwrap()
    }

    fn other_id() -> Id {
        "683d5a1b4f2e7c0019000001".parse().unwrap()
    }

    /// Build a minimal proto-encoded payload with a single string at field 1.
    fn encode_field1_string(s: &str) -> Vec<u8> {
        // Proto wire format: tag = (field_number << 3) | wire_type
        // field 1, wire type 2 (length-delimited): tag byte = 0x0a
        let bytes = s.as_bytes();
        let mut out = Vec::new();
        out.push(0x0a); // field 1, wire type 2
        // varint-encode length
        let mut len = bytes.len();
        loop {
            let byte = (len & 0x7f) as u8;
            len >>= 7;
            if len == 0 {
                out.push(byte);
                break;
            } else {
                out.push(byte | 0x80);
            }
        }
        out.extend_from_slice(bytes);
        out
    }

    /// Encode two string fields: field 1 = id, field 2 = school.
    fn encode_field1_field2_string(f1: &str, f2: &str) -> Vec<u8> {
        let mut out = encode_field1_string(f1);
        // field 2, wire type 2: tag byte = 0x12
        let bytes = f2.as_bytes();
        out.push(0x12);
        let mut len = bytes.len();
        loop {
            let byte = (len & 0x7f) as u8;
            len >>= 7;
            if len == 0 { out.push(byte); break; } else { out.push(byte | 0x80); }
        }
        out.extend_from_slice(bytes);
        out
    }

    #[test]
    fn create_school_is_system() {
        // CREATE_SCHOOL needs no connection — returns System immediately
        // We pass a dummy connection; the function should not use it.
        // Use a temp in-memory DB.
        let mut conn = crate::db::database::test_conn();
        let result = action_organisation(
            &mut conn,
            sync_action::CREATE_SCHOOL,
            dummy_id(),
            &[],
        );
        assert!(matches!(result, Ok(Organisation::System)));
    }

    #[test]
    fn create_plan_is_system() {
        let mut conn = crate::db::database::test_conn();
        let result = action_organisation(
            &mut conn,
            sync_action::CREATE_PLAN,
            dummy_id(),
            &[],
        );
        assert!(matches!(result, Ok(Organisation::System)));
    }

    #[test]
    fn update_user_own_is_account() {
        let id = dummy_id();
        let mut conn = crate::db::database::test_conn();
        let payload = encode_field1_string(&id.to_string());
        let result = action_organisation(
            &mut conn,
            sync_action::UPDATE_USER,
            id,
            &payload,
        );
        assert!(matches!(result, Ok(Organisation::Account)));
    }

    #[test]
    fn update_user_other_is_system() {
        let acting_user = dummy_id();
        let target_user = other_id();
        let mut conn = crate::db::database::test_conn();
        let payload = encode_field1_string(&target_user.to_string());
        let result = action_organisation(
            &mut conn,
            sync_action::UPDATE_USER,
            acting_user,
            &payload,
        );
        assert!(matches!(result, Ok(Organisation::System)));
    }

    #[test]
    fn update_school_extracts_school_id_from_id_field() {
        let school_id = dummy_id();
        let mut conn = crate::db::database::test_conn();
        let payload = encode_field1_string(&school_id.to_string());
        let result = action_organisation(
            &mut conn,
            sync_action::UPDATE_SCHOOL,
            other_id(),
            &payload,
        );
        assert!(matches!(result, Ok(Organisation::School(id)) if id == school_id));
    }

    #[test]
    fn create_teacher_extracts_school_from_field1() {
        let school_id = dummy_id();
        let mut conn = crate::db::database::test_conn();
        let payload = encode_field1_string(&school_id.to_string());
        let result = action_organisation(
            &mut conn,
            sync_action::CREATE_TEACHER,
            other_id(),
            &payload,
        );
        assert!(matches!(result, Ok(Organisation::School(id)) if id == school_id));
    }

    #[test]
    fn create_exam_extracts_school_from_field2() {
        let school_id = dummy_id();
        let exam_id = "exam_001";
        let mut conn = crate::db::database::test_conn();
        let payload = encode_field1_field2_string(exam_id, &school_id.to_string());
        let result = action_organisation(
            &mut conn,
            sync_action::CREATE_EXAM,
            other_id(),
            &payload,
        );
        assert!(matches!(result, Ok(Organisation::School(id)) if id == school_id));
    }

    #[test]
    fn assign_role_with_school_is_school_scoped() {
        let school_id = dummy_id();
        let mut conn = crate::db::database::test_conn();
        // Encode optional school at field 1
        let payload = encode_field1_string(&school_id.to_string());
        let result = action_organisation(
            &mut conn,
            sync_action::ASSIGN_ROLE,
            other_id(),
            &payload,
        );
        assert!(matches!(result, Ok(Organisation::School(id)) if id == school_id));
    }

    #[test]
    fn assign_role_without_school_is_system() {
        let mut conn = crate::db::database::test_conn();
        // Empty payload → no school field → system scope
        let result = action_organisation(
            &mut conn,
            sync_action::ASSIGN_ROLE,
            dummy_id(),
            &[],
        );
        assert!(matches!(result, Ok(Organisation::System)));
    }
}
```

Note: `crate::db::database::test_conn()` — you will need to add this helper if it does not
exist. It returns an in-memory SQLite connection for tests:

```rust
// In src/db/database/mod.rs, inside #[cfg(test)]:
#[cfg(test)]
pub fn test_conn() -> diesel::SqliteConnection {
    use diesel::Connection;
    diesel::SqliteConnection::establish(":memory:")
        .expect("failed to open in-memory SQLite")
}
```

The DB-lookup tests (UpdateExam, UpdateFee, etc.) require actual records in the DB and are
integration tests — skip them for now. The unit tests above cover all non-lookup branches.

**Step 4 — Manual smoke test (optional but recommended)**

With a running server, use the existing Flutter client to:
1. Log in as a Normal user (not owner)
2. Attempt to create a student, teacher, etc. in a school they ARE a member of → should succeed
3. Attempt the same in a school they are NOT a member of → should get a `Permission denied` error
4. Log in as the school owner → all operations should succeed (owner bypass)
5. Log in as a System user with appropriate roles → should work for allowed resources

### Update after completion
- [x] Mark this task `[x]`
- [ ] Orchestrator: commit with message:
  ```
  feat: implement full three-tier push authorization

  Replace the broken Normal-user bypass in check_action_permission with a
  complete authorization path that uses the existing Authorize infrastructure.

  Changes:
  - actions.rs: add action_organisation() — maps every action + payload to
    its Organisation context (System, Account, or School(id)). Uses prost
    helper structs for payload decoding and SQL lookups for records whose
    school is not carried in the payload.
  - authorize.rs: add authorize_user() — the push-path equivalent of
    Authorize::authorize that takes an already-loaded &User to avoid a
    redundant DB fetch. Handles Super bypass, school owner bypass, school
    active-status check, and role-based permission aggregation.
  - sync.rs: replace check_action_permission with a three-line call that
    builds required Permissions, determines Organisation context, and calls
    authorize_user. No proto changes. No frontend changes.

  Normal users can now only push actions to schools they are members of,
  and only if their school-scoped roles grant the required resource+action.
  School owners retain full bypass within their own school. System users
  continue to use system-scoped role aggregation. Super users bypass all
  checks.
  ```
