# TASKS.md — Standalone Sync Invite User Action

## Status Legend
- [ ] Pending
- [x] Complete

---

## Contract Decision

This task list assumes the new standalone sync point is an **invite-only** action, not a general-purpose user insert.

### Proposed wire contract
- Add `InviteUserPayload` to `protos/services/sync.proto` with exactly these fields:
  - `string id = 1`
  - `string phone = 2`
  - `string name = 3`
  - `int32 level = 4`
- The standalone action does **not** accept `status`, `email`, or arbitrary profile fields.
- The server always persists `status = Invited` for this action.
- No database migration is required on the Rust side.

### Action-number rule
- `src/db/database/tables/actions.rs` owns the Rust `sync_action::*` integer constants.
- Append the new constant after the current highest action (`DELETE_ANSWER_SHEET = 94`).
- Use `INVITE_USER = 95`.
- Do **not** renumber existing values `0..94`.
- Keep `89` and `90` reserved.

### Authorization rule
- Map the new standalone action to `Resource::Users` + `Action::Create`.
- Run authorization in `Organisation::System`.
- Super users still bypass authorization through the existing `authorize_user()` flow.

### Actor/target level rule
This plan follows your requested standalone-invite policy:
- `Level::Super` may invite `Normal`, `System`, or `Super`.
- `Level::System` with `Users.Create` may invite `System` users only.
- `Level::Normal` may not use the standalone invite action.
- Normal invited users created from school/member actions remain side effects of those parent actions.

This intentionally keeps standalone normal invites out of the system-user path while preserving the existing member/school side-effect invite flow for normal users.

### Backward-compatibility rule
Already-failed client sync rows must recover after the client upgrades:
1. The client will rewrite invite-shaped legacy `UPDATE_USER` logs into the new action.
2. The server will also add a fallback so a missing-row `UPDATE_USER` with an invite-shaped payload is upgraded into the new invite path.
3. If the phone resolves to an existing non-deleted user and the client used a provisional local ID, the server returns:
   - an `ActionRow` delete for the provisional local user row, and
   - an `ActionRow` upsert for the authoritative server user row.
4. That provisional-row cleanup is **response-only**. Do **not** append a changelog delete or delete-sidecar entry for a row that never existed on the server.

---

## Dependency Graph

```text
A1 → A2 → B1 → B2 → B3 → C1
```

---

## Track A: Contract + execution plumbing

### Task A1: Add the standalone invite contract and wire the new action slot
**Files to create/modify:** `protos/services/sync.proto`, `src/db/database/tables/actions.rs`
**Reference files to read:** `protos/types/user.proto`, `src/db/database/authorize.rs`, `src/services/sync.rs`
**Depends on:** nothing
**Parallel group:** sequential

**Specification:**
- In `protos/services/sync.proto`, change the users section from `2 actions` to `3 actions` and add:
  - `message InviteUserPayload { string id = 1; string phone = 2; string name = 3; int32 level = 4; }`
- Keep `UpdateUserPayload` and `DeleteUserPayload` exactly where they belong for real update/delete behavior.
- Update the top payload-count comment to reflect the extra message.
- In `src/db/database/tables/actions.rs`, append `pub const INVITE_USER: i32 = 95;` to `pub mod sync_action`.
- Add the new action to:
  - `action_permission()` as `(Resource::Users, Action::Create)`
  - `action_organisation()` as `Organisation::System`
  - `execute_action()` dispatch, routed to a dedicated `handle_invite_user(...)`
- Do not renumber any existing action IDs.
- Add an inline comment near `sync_action` noting that these integers must stay aligned with the client’s `lib/database/tables/enums.dart` values.

**Update after completion:**
- [x] Mark this task `[x]`
- [ ] Orchestrator: git commit after this task

---

### Task A2: Thread the authenticated actor into action execution
**Files to create/modify:** `src/services/sync.rs`, `src/db/database/tables/actions.rs`
**Reference files to read:** `src/services/sync.rs`, `src/db/database/tables/actions.rs`
**Depends on:** Task A1
**Parallel group:** sequential

**Specification:**
- Change the action execution path so invite-related handlers can inspect the authenticated actor.
- Preferred shape:
  - change `actions::execute_action(conn, action_id, payload)` to `actions::execute_action(conn, user, action_id, payload)`
  - keep the transaction boundary in `process_action()` exactly where it is now
  - keep authorization before execution exactly where it is now
- Update the match arms inside `execute_action()` to pass the actor only to handlers that need it. Existing handlers may ignore the new parameter if unused.
- The goal is to make the actor’s `id` and `level` available to the standalone invite path, the legacy fallback path, and the shared invite helper used by member/school side-effect invites.

**Update after completion:**
- [ ] Mark this task `[x]`
- [ ] Orchestrator: git commit after this task

---

## Track B: Shared invite logic + backward compatibility

### Task B1: Build one shared invite helper and implement `handle_invite_user`
**Files to create/modify:** `src/db/database/tables/actions.rs`
**Reference files to read:** `src/types/user/level.rs`, `src/types/user/status.rs`, `src/db/database/tables/insert.rs`
**Depends on:** Task A2
**Parallel group:** sequential

**Specification:**
- Add one private helper in `src/db/database/tables/actions.rs` for all invitation-style user creation.
- The helper should support two modes:
  - `StandaloneInvite`
  - `SideEffectInvite`
- Use a private outcome struct or equivalent tuple that carries:
  - the authoritative `UserRow`
  - whether a new server row was inserted
  - any extra response rows needed for local reconciliation (for example a provisional-row delete)
- Centralize these behaviors in that helper:
  1. Parse and normalize `phone` using the existing `Phone` type so stored phones always use the normalized Kenyan format.
  2. Parse `level` via `Level::try_from(i32)`.
  3. Enforce actor/target rules in one place:
     - `StandaloneInvite` uses the matrix from the contract section above, which means `Level::System` may invite `Level::System` only while `Level::Super` may invite any level.
     - `SideEffectInvite` always forces `Level::Normal` and does not require `Users.Create`; authorization has already been enforced by the parent school/member action.
  4. Always insert `status = Status::Invited`.
  5. Never read `email` for the standalone action; the new contract is `id + phone + name + level` only.
  6. Reuse an existing user when the normalized phone already exists **and** that user is not deleted.
  7. Reject deleted existing users on that phone with `Error::UserNotFound` so deleted identities are not silently resurrected.
  8. When the helper reuses an existing user and the client-provided provisional ID differs from the authoritative user ID, add a response-only `delete_row(TBL_USERS, provisional_id.to_string())` to the outcome so the pushing client can clean up the orphan local row.
  9. When the helper inserts a new user, create `UserInsert { id, phone, email: None, name, level, status: Invited }`, append the normal users insert changelog entry, and fetch the authoritative row back with `fetch_user()`.
  10. Never append a changelog delete or delete-sidecar row for a provisional local-only user ID.
- Implement `handle_invite_user(conn, actor, payload)` on top of that helper.
- `handle_invite_user()` must decode `InviteUserPayload`, call the shared helper in `StandaloneInvite` mode, and return an `ActionResult` containing:
  - any provisional cleanup delete row first if needed, then
  - the authoritative users upsert row.

**Update after completion:**
- [ ] Mark this task `[x]`
- [ ] Orchestrator: git commit after this task

---

### Task B2: Make legacy `UPDATE_USER` create-via-update failures recover automatically
**Files to create/modify:** `src/db/database/tables/update.rs`, `src/db/database/tables/actions.rs`
**Reference files to read:** `src/db/database/tables/update.rs`, `src/db/database/tables/actions.rs`, `src/services/sync.rs`
**Depends on:** Task B1
**Parallel group:** sequential

**Specification:**
- Fix `update::update_user()` so it does not silently succeed when `UPDATE users ... WHERE id = ?` matches zero rows.
- Use the return value from `.execute(conn)?`; if `rows_affected == 0`, return `Error::UserNotFound`.
- Then update `handle_update_user()` to distinguish real update-not-found from legacy create-via-update payloads.
- A payload counts as a legacy invite shape when all of the following are true:
  - `phone.is_some()`
  - `name.is_some()`
  - `level.is_some()`
  - `status == Some(0)` (`Invited`)
- `email` may be present in the legacy payload, but the fallback path must ignore it because the new standalone contract is intentionally smaller.
- New `handle_update_user()` behavior:
  1. If the row exists, keep the current update path.
  2. If the update misses and the payload is not invite-shaped, return `Error::UserNotFound` exactly as before.
  3. If the update misses and the payload is invite-shaped, route it through the shared helper in `StandaloneInvite` mode using:
     - `requested_id = p.id`
     - `phone = p.phone.unwrap()`
     - `name = p.name.unwrap()`
     - `level = p.level.unwrap()`
     - `provisional_id = Some(&p.id)`
- For that fallback path:
  - do not append an `OP_UPDATE` changelog entry
  - append only the real users insert changelog if a new server row is inserted
  - return the same reconciliation rows described in Task B1

**Update after completion:**
- [ ] Mark this task `[x]`
- [ ] Orchestrator: git commit after this task

---

### Task B3: Refactor all side-effect invite flows to reuse the shared helper
**Files to create/modify:** `src/db/database/tables/actions.rs`
**Reference files to read:** `src/db/database/tables/actions.rs`
**Depends on:** Task B2
**Parallel group:** sequential

**Specification:**
- Remove the duplicated user-invitation blocks from these existing paths and route them through the shared helper from Task B1:
  - `handle_create_school()`
  - `handle_create_teacher()`
  - `handle_create_staff()`
  - `handle_create_owner()`
  - `handle_create_guardian()`
  - `resolve_phone_to_user()` used by student flows
- Preserve the current parent-action response shapes. For example, `handle_create_teacher()` must still return both the users upsert row and the teachers upsert row.
- All of these side-effect flows must call the helper in `SideEffectInvite` mode so they continue to create only `Normal` invited users.
- Side-effect invite behavior after refactor:
  - existing non-deleted phone → reuse existing user
  - missing phone → create a new `Normal` + `Invited` user
  - deleted existing phone → return `Error::UserNotFound`
- Keep changelog behavior the same for the real inserted member/school rows.
- Do not broaden the scope into unrelated membership logic.

**Update after completion:**
- [ ] Mark this task `[x]`
- [ ] Orchestrator: git commit after this task

---

## Track C: Tests

### Task C1: Add regression tests for the new invite action and legacy recovery path
**Files to create/modify:** `src/db/database/tables/actions.rs`, `src/services/sync.rs`
**Reference files to read:** `src/db/database/tables/actions.rs`, `src/services/sync.rs`, `src/db/database/authorize.rs`
**Depends on:** Task B3
**Parallel group:** sequential

**Specification:**
- Add focused tests near the code they exercise. Use the project’s existing co-located `#[cfg(test)]` style.
- Minimum coverage required:
  1. `action_permission(INVITE_USER)` returns `(Resource::Users, Action::Create)`.
  2. `action_organisation(..., INVITE_USER, ...)` returns `Organisation::System`.
  3. A super user can use the standalone invite path for every allowed target level in the chosen matrix.
  4. A normal user cannot use the standalone invite path.
  5. A system user follows the requested matrix exactly:
     - `System -> System` succeeds when the actor has `Users.Create`
     - `System -> Normal` is denied for the standalone invite action
     - `System -> Super` is denied
  6. A deleted user already occupying the phone is rejected.
  7. Reusing an existing non-deleted phone returns the authoritative users row and, when IDs differ, a response-only delete row for the provisional local ID.
  8. A missing-row legacy `UPDATE_USER` with `phone + name + level + status=Invited` is upgraded into the invite path and succeeds.
  9. A missing-row non-invite `UPDATE_USER` still returns `Error::UserNotFound` and therefore sync code `4`.
  10. At least one side-effect flow still creates/reuses a `Normal` invited user through the shared helper.
- Keep the tests focused on behavior. Do not rewrite the whole sync test harness.

**Update after completion:**
- [ ] Mark this task `[x]`
- [ ] Orchestrator: git commit after this task

---

## Notes for the executor
- No Diesel migration is expected.
- No `build.rs` change is expected because `sync.proto` is already compiled.
- Be careful with changelog semantics: only real server-side inserts/updates/deletes belong in the log files.
- The local provisional-user cleanup row exists only in `ActionResponse.rows` for the pusher and should never leak into watch-stream changelog replay.
