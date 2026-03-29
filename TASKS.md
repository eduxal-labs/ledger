# TASKS.md

---

## Feature: Student Phone-Based Invitation Pattern

### Overview

The client has changed how it creates/updates students. The optional `user` field in `CreateStudentPayload` and `UpdateStudentPayload` now carries a **phone number** (not a user ID). The server must:

1. **Create**: Validate the phone, look up the user by phone, reuse if found, or create a new invited user (server-generated ID) with the student's name. Store the resolved user ID on the student row. Return both the user and student rows in the response.
2. **Update (link/relink)**: Same phone resolution as create. If the student was previously linked to a different user, check if the old user is orphaned (no other school links) and if so, delete them.
3. **Update (unlink)**: If the client sends a non-null value that is NOT a valid phone (canonically `"-"`), set the student's `user` to `NULL`. Then check if the old user is orphaned and delete if so.

### Semantic Summary

| Operation | `user` field value | Server behavior |
|---|---|---|
| Create | `None` | No user link. Insert student with `user = NULL`. |
| Create | `Some(valid_phone)` | Resolve phone → user ID (existing or new invitation). Insert student with resolved ID. Return user + student rows. |
| Create | `Some(invalid / "-")` | Reject with validation error (code 3). |
| Update | `None` | No change to user link. Standard COALESCE behavior. |
| Update | `Some(valid_phone)` | Resolve phone → user ID. Update student's user field. If old user was different and now orphaned, delete old user. Return user + student rows. |
| Update | `Some(invalid / "-")` | Unlink: set user to `NULL`. If old user is now orphaned, delete old user. Return delete row for orphaned user + updated student row. |

### Coordination

All tasks modify files under `src/db/database/tables/`. Tasks must be **executed sequentially** (Task 1 → Task 2 → Task 3) because they all touch `actions.rs` and later tasks depend on helpers introduced in earlier tasks.

---

### Task 1: Add helper functions in `actions.rs` ✅

**Files to modify:** `src/db/database/tables/actions.rs`
**Reference files to read:** `src/types/phone.rs`, `src/db/database/tables/actions.rs` (existing helpers like `fetch_user_by_phone`, `fetch_user`)
**Depends on:** None
**Parallel group:** — (sequential, must complete before Task 2 and Task 3)

**Specification:**

Add two new helper functions near the other `fetch_*` / helper functions in `actions.rs` (around line 505, after `fetch_guardian`).

#### 1a. `user_has_school_links(conn, user_id) -> Result<bool>`

Checks whether a given user ID appears in **any** of the five member tables (`owners`, `teachers`, `staff`, `guardians`, `students`). If any row references this user, return `true`. Otherwise `false`.

Use a single SQL query with `UNION ALL` and `LIMIT 1` for efficiency:

```rust
fn user_has_school_links(conn: &mut Conn, user_id: &str) -> Result<bool> {
    let rows = sql_query(
        "SELECT 1 AS cnt FROM owners WHERE user = ? \
         UNION ALL SELECT 1 FROM teachers WHERE user = ? \
         UNION ALL SELECT 1 FROM staff WHERE user = ? \
         UNION ALL SELECT 1 FROM guardians WHERE user = ? \
         UNION ALL SELECT 1 FROM students WHERE user = ? \
         LIMIT 1",
    )
    .bind::<Text, _>(user_id)
    .bind::<Text, _>(user_id)
    .bind::<Text, _>(user_id)
    .bind::<Text, _>(user_id)
    .bind::<Text, _>(user_id)
    .load::<FkCheckRow>(conn)
    .map_err(|e| {
        tracing::error!("user_has_school_links failed: {e}");
        Error::Internal
    })?;
    Ok(!rows.is_empty())
}
```

The `FkCheckRow` struct already exists at the top of the file (line 19–22) with a `cnt` field — reuse it.

#### 1b. `resolve_phone_to_user(conn, phone_str, fallback_name) -> Result<(UserRow, bool)>`

Validates a phone string, looks up the user, creates an invited user if not found, and returns `(user_row, was_created)`.

```rust
/// Validates `phone_str` as a Kenyan phone number, looks up the user by the
/// normalised phone, and creates a new Invited user when none exists.
///
/// Returns `(resolved_user_row, true)` if a new user was created, or
/// `(existing_user_row, false)` if the phone matched an existing user.
fn resolve_phone_to_user(
    conn: &mut Conn,
    phone_str: &str,
    fallback_name: &str,
) -> Result<(UserRow, bool)> {
    use crate::types::phone::Phone;
    use std::str::FromStr;

    let phone = Phone::from_str(phone_str).map_err(|_| {
        tracing::warn!("invalid phone in student user field: {phone_str}");
        Error::InvalidPhone
    })?;
    let normalized = phone.to_string(); // "0XXXXXXXXX"

    match fetch_user_by_phone(conn, &normalized)? {
        Some(existing) => Ok((existing, false)),
        None => {
            let new_id = Id::default().to_string();
            let user_insert = UserInsert {
                id: new_id.clone(),
                phone: normalized,
                email: None,
                name: fallback_name.to_string(),
                level: 0,  // Normal
                status: 0, // Invited
            };
            insert::insert_user(conn, &user_insert)?;
            let row = fetch_user(conn, &new_id)?;
            Ok((row, true))
        }
    }
}
```

This requires adding `use crate::types::id::Id;` at the top of the file if not already present. Check existing imports — `Id` is already used via `Id::system()` in the handlers, so the import should already be there. If not, add it.

**Update after completion:**
- [x] `user_has_school_links` function added and compiles
- [x] `resolve_phone_to_user` function added and compiles
- [x] `cargo build` succeeds (no new errors)
- [x] Mark this task `[x]`
- [ ] Orchestrator: git commit `feat: add resolve_phone_to_user and user_has_school_links helpers`

---

### Task 2: Rewrite `handle_create_student` with phone-based invitation pattern ✅

**Files to modify:** `src/db/database/tables/actions.rs`
**Reference files to read:** `src/db/database/tables/actions.rs` (the `handle_create_teacher` function at L1486–1540 for the pattern to follow; the existing `handle_create_student` at L1741–1767; the helpers from Task 1)
**Depends on:** Task 1
**Parallel group:** — (sequential)

**Specification:**

Replace the body of `handle_create_student` (lines 1741–1767) with the invitation pattern. The new logic:

```rust
fn handle_create_student(conn: &mut Conn, payload: &[u8]) -> Result<ActionResult> {
    let p: CreateStudentPayload = decode(payload)?;
    let log_user = Id::system();

    let mut rows = Vec::new();

    // Resolve optional phone → user ID
    let resolved_user: Option<String> = match p.user.as_deref() {
        None => None,
        Some(phone_str) => {
            // On create, the value MUST be a valid phone. Invalid → reject.
            let (user_row, was_created) = resolve_phone_to_user(conn, phone_str, &p.name)?;
            if was_created {
                append_log(log_user, TBL_USERS as u8, OP_INSERT, 0)?;
            }
            let user_id = user_row.id.clone();
            rows.push(upsert_row(
                TBL_USERS,
                user_row.row_key(),
                InsertData {
                    row: Some(insert_data::Row::User((&user_row).into())),
                },
            ));
            Some(user_id)
        }
    };

    let student_insert = StudentInsert {
        school: p.school.clone(),
        adm: p.adm,
        user: resolved_user,
        name: p.name.clone(),
        dob: p.dob,
        gender: p.gender,
        documents: p.documents.clone(),
        admitted: p.admitted,
        status: 0, // Active
    };
    insert::insert_student(conn, &student_insert)?;
    append_log(log_user, TBL_STUDENTS as u8, OP_INSERT, 0)?;

    let row = fetch_student(conn, &p.school, p.adm)?;
    rows.push(upsert_row(
        TBL_STUDENTS,
        row.row_key(),
        InsertData {
            row: Some(insert_data::Row::Student((&row).into())),
        },
    ));

    Ok(ActionResult::with_rows(rows))
}
```

**Key behaviors:**

1. `p.user = None` → `resolved_user = None` → student inserted with `user = NULL`. Only the student row returned. Identical to old behavior.
2. `p.user = Some(valid_phone)` → `resolve_phone_to_user` either finds an existing user or creates a new Invited user (server-generated ID via `Id::default()`). The student is inserted with the resolved user ID. Both user and student rows are returned.
3. `p.user = Some(invalid_phone)` → `resolve_phone_to_user` returns `Err(Error::InvalidPhone)` → the handler returns an error → `process_action` maps it to `code: 3` (validation error).
4. When a new user is created, append a `TBL_USERS` INSERT to the changelog so other watchers pick it up.
5. The user row is always included in the response (even if the user already existed) so the pushing client gets the full user data.

**Update after completion:**
- [x] `handle_create_student` rewritten with phone resolution
- [x] `cargo build` succeeds
- [x] Mark this task `[x]`
- [ ] Orchestrator: git commit `feat: handle_create_student resolves phone to user via invitation pattern`

---

### Task 3: Rewrite `handle_update_student` with phone resolution and unlink logic ✅

**Files to modify:** `src/db/database/tables/actions.rs`
**Reference files to read:** `src/db/database/tables/actions.rs` (the current `handle_update_student` at L1769–1811; `update::update_student` in `src/db/database/tables/update.rs` at L74–105; `delete::delete_user` in `src/db/database/tables/delete.rs` at L19–24; helpers from Task 1)
**Depends on:** Task 1
**Parallel group:** — (sequential)

**Specification:**

Replace the body of `handle_update_student` (lines 1769–1811) with phone resolution, relinking, and unlinking logic.

**Important SQL detail:** The existing `update::update_student` uses `COALESCE(?, user)` for the user column. This means:
- `p.user = Some("id_string")` → sets user to `"id_string"` ✅
- `p.user = None` → `COALESCE(NULL, user)` → keeps old value ✅
- But there is NO way to set user to `NULL` through COALESCE.

For the **unlink** case, after `update_student` runs (handling all other fields), a separate SQL statement must explicitly set `user = NULL`.

```rust
fn handle_update_student(conn: &mut Conn, payload: &[u8]) -> Result<ActionResult> {
    use crate::config::storage::sign;
    use crate::types::phone::Phone;
    use chrono::Utc;
    use std::str::FromStr;

    let mut p: UpdateStudentPayload = decode(payload)?;
    let log_user = Id::system();
    let row_key = format!("{}|{}", p.school, p.adm);

    let mut rows = Vec::new();

    // Track whether we need to unlink and the old user ID for orphan check.
    let mut unlink = false;
    let mut old_user_id: Option<String> = None;

    // --- Phase 1: Resolve the user field ---
    if let Some(ref value) = p.user {
        match Phone::from_str(value) {
            Ok(phone) => {
                // Valid phone → resolve to user ID (link or relink).
                // Fetch old student first to detect relink.
                let old_student = fetch_student(conn, &p.school, p.adm)?;
                old_user_id = old_student.user.clone();

                // Determine the fallback name for a new invitation:
                // prefer the name being set in this update, else the existing student name.
                let name = p.name.as_deref().unwrap_or(&old_student.name);

                let (user_row, was_created) =
                    resolve_phone_to_user(conn, &phone.to_string(), name)?;
                if was_created {
                    append_log(log_user, TBL_USERS as u8, OP_INSERT, 0)?;
                }

                let new_user_id = user_row.id.clone();
                rows.push(upsert_row(
                    TBL_USERS,
                    user_row.row_key(),
                    InsertData {
                        row: Some(insert_data::Row::User((&user_row).into())),
                    },
                ));

                // Set the resolved ID so update_student writes it via COALESCE.
                p.user = Some(new_user_id.clone());

                // If the old user was different, we may need to clean it up.
                if old_user_id.as_deref() == Some(&new_user_id) {
                    old_user_id = None; // Same user, no orphan check needed.
                }
            }
            Err(_) => {
                // Invalid phone (including "-") → unlink.
                let old_student = fetch_student(conn, &p.school, p.adm)?;
                old_user_id = old_student.user.clone();
                unlink = true;
                p.user = None; // Don't let COALESCE touch user; we handle it below.
            }
        }
    }

    // --- Phase 2: Run the standard update (handles all other fields) ---
    update::update_student(conn, &row_key, &p)?;

    // --- Phase 3: Explicit NULL for unlink (COALESCE can't do this) ---
    if unlink {
        sql_query("UPDATE students SET user = NULL WHERE school = ? AND adm = ?")
            .bind::<Text, _>(&p.school)
            .bind::<diesel::sql_types::Integer, _>(p.adm)
            .execute(conn)
            .map_err(|e| {
                tracing::error!("clear student user link failed: {e}");
                Error::Internal
            })?;
    }

    // --- Phase 4: Orphan cleanup for old user ---
    if let Some(ref orphan_id) = old_user_id {
        if !user_has_school_links(conn, orphan_id)? {
            // The old user has no remaining school links — delete them.
            delete::delete_user(conn, orphan_id)?;
            append_log(log_user, TBL_USERS as u8, OP_DELETE, 0)?;
            append_delete_log(TBL_USERS as u8, orphan_id)?;
            rows.push(delete_row(TBL_USERS, orphan_id.clone()));
        }
    }

    // --- Phase 5: Changelog + build response ---
    append_log(log_user, TBL_STUDENTS as u8, OP_UPDATE, 0)?;

    let row = fetch_student(conn, &p.school, p.adm)?;

    // S3 presigned URLs for student profile image (preserve existing behavior).
    let path = format!("schools/{}/students/{}/image", p.school, p.adm);
    let put_url = sign::url(&path, sign::PUT_TTL, true);
    let get_url = sign::url(&path, sign::GET_TTL, false);
    let expiry_ms = (Utc::now().timestamp() + sign::GET_TTL as i64) * 1000;

    let file_url = FileUrl {
        path,
        put_url: Some(put_url),
        get_url: Some(get_url),
        expiry: expiry_ms,
    };

    rows.push(upsert_row(
        TBL_STUDENTS,
        row.row_key(),
        InsertData {
            row: Some(insert_data::Row::Student((&row).into())),
        },
    ));

    Ok(ActionResult::with_rows_and_urls(rows, vec![file_url]))
}
```

**Key behaviors:**

1. `p.user = None` → no change to user link. Straight pass-through to `update_student` via COALESCE. Only student row + file URLs returned. Identical to old behavior.

2. `p.user = Some(valid_phone)` → phone is resolved to a user ID. If the student was previously linked to a *different* user, the old user is checked for orphan status. The student's user field is updated to the new user's ID. Both the (new) user row and updated student row are returned.

3. `p.user = Some("-")` or any invalid phone → **unlink**:
   - The current student is fetched to capture `old_user_id` before any changes.
   - `update_student` runs with `p.user = None` (COALESCE preserves old value for now, other fields update normally).
   - A follow-up SQL explicitly sets `user = NULL`.
   - If `old_user_id` existed and the old user has no remaining school links across `owners`, `teachers`, `staff`, `guardians`, and `students`, the old user is hard-deleted (`DELETE FROM users`), a DELETE changelog + delete sidecar entry is appended, and a `delete_row` for the user is included in the response.
   - The updated student row (with `user = NULL`) is returned.

4. **Relink orphan check**: When relinking from user A to user B (both valid phones), user A is checked for orphan status after the student's user field is updated. If orphaned, user A is deleted. This prevents accumulation of dead invitation users.

5. All operations run inside the existing transaction wrapping `execute_action` in `process_action` (line 321 of `sync.rs`), so either everything succeeds or nothing does.

6. The S3 presigned URL logic for the student profile image is preserved exactly as it was.

**Imports needed at the top of `handle_update_student`:**
- `use crate::types::phone::Phone;` — already used in the function via `Phone::from_str`
- `use std::str::FromStr;` — for `Phone::from_str`
- `use crate::config::storage::sign;` — already present in the existing code
- `use chrono::Utc;` — already present in the existing code
- `diesel::sql_types::Integer` — already imported at the file level

**Update after completion:**
- [x] `handle_update_student` rewritten with phone resolution + unlink + orphan cleanup
- [x] `cargo build` succeeds
- [x] Mark this task `[x]`
- [ ] Orchestrator: git commit `feat: handle_update_student resolves phone, supports unlink with orphan cleanup`
