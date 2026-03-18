# Ledger — Task Board

## Fix: 4 Tables Missing From Sync Engine (subject_catalog, topics, streams, mpesa)

### Problem Summary

Four tables (`subject_catalog`, `topics`, `streams`, `mpesa`) are never synced to clients — neither on initial sync (cold start) nor on incremental sync (watch loop). This is caused by three layered bugs:

1. **Bug #1 — Missing `LogTable` variants:** The `LogTable` enum in `src/services/sync.rs` stops at `Discounts = 30`. It has no variants for `SubjectCatalog` (31), `Topics` (32), `Streams` (33), or `Mpesa` (34). The `from_i32` method returns `None` for these IDs, causing the sync engine to silently skip them.

2. **Bug #2 — Missing from `SNAPSHOT_TABLE_ORDER`:** The `SNAPSHOT_TABLE_ORDER` constant (which drives the initial full snapshot) only lists table IDs 1–30. IDs 31–34 are absent, so `send_full_snapshot` never visits these tables.

3. **Bug #3 — Filter blindspot for global catalog tables:** `subject_catalog` and `topics` are global system-level tables with no `school` column (`school_id: None`). The `SyncFilter::row_visible` method returns `false` for Normal/System users when `school_id` is `None` — unless the table is `Users` or the resource is `Plans`. These two catalog tables need the same "always visible for read" exemption that `Plans` gets.

### Affected Tables

| Table | snapshot.rs ID | In `LogTable`? | In `SNAPSHOT_TABLE_ORDER`? | Filter issue? | Scope |
|---|---|---|---|---|---|
| `subject_catalog` | 31 | ❌ | ❌ | ❌ `school_id=None`, filtered out for Normal+System users | Global |
| `topics` | 32 | ❌ | ❌ | ❌ `school_id=None`, filtered out for Normal+System users | Global |
| `streams` | 33 | ❌ | ❌ | ✅ Has `school_id`, filter works | School-scoped |
| `mpesa` | 34 | ❌ | ❌ | ✅ Has `school_id`, filter works | School-scoped |

### Important Context

- The push side (`actions.rs`) already writes table IDs 31–34 to the changelog correctly via `append_log`. The problem is exclusively on the **read/watch side**.
- `LogTable::Subjects` (ID 12) is the `subject_teachers` join table, NOT the global subjects catalog. The naming is a coincidence.
- `subject_catalog` and `topics` map to `Resource::Subjects` on the push permission side. They should also map to `Resource::Subjects` on the watch side.
- `streams` maps to `Resource::Schools` on the push permission side (school-scoped). Same for `mpesa`.
- The `resource()` method on `LogTable` is used by `table_visible` and `row_visible` for permission filtering. New variants need correct resource mappings.
- The `school_from_key` method on `LogTable` is used to extract school_id from row_keys for delete sidecar records. New variants need correct implementations.

---

### Task 01: Add 4 new variants to `LogTable` enum and update all methods

**Files to modify:** `ledger/src/services/sync.rs`
**Reference files to read:** None needed — all context is below.
**Depends on:** Nothing
**Parallel group:** P1

**Specification:**

In `src/services/sync.rs`, make the following changes:

**1a. Add 4 new variants to the `LogTable` enum (after `Discounts = 30`):**

```rust
SubjectCatalog = 31,
Topics = 32,
Streams = 33,
Mpesa = 34,
```

**1b. Add 4 new arms to `LogTable::from_i32` (after the `30 => Discounts` arm):**

```rust
31 => Some(Self::SubjectCatalog),
32 => Some(Self::Topics),
33 => Some(Self::Streams),
34 => Some(Self::Mpesa),
```

**1c. Add 4 new arms to `LogTable::resource` (inside the match):**

```rust
Self::SubjectCatalog | Self::Topics => Some(Resource::Subjects),
Self::Streams | Self::Mpesa => Some(Resource::Schools),
```

These mappings match the existing `action_permission` function in `actions.rs` where:
- `CREATE_SUBJECT` / `CREATE_TOPIC` → `Resource::Subjects`
- `CREATE_STREAM` / `CREATE_MPESA` → `Resource::Schools`

**1d. Add arms to `LogTable::school_from_key`:**

- `SubjectCatalog` and `Topics` are global (no school column) → return `None`. Add them to the existing system-level arm:
  ```rust
  Self::Users | Self::Plans | Self::SubjectCatalog | Self::Topics => None,
  ```
- `Streams` has composite key `school|grade|stream` → the existing wildcard `_` arm already handles this correctly (takes first `|`-delimited segment). No change needed.
- `Mpesa` has key `school` (just the school ID, no delimiter) → the existing wildcard `_` arm handles this too. No change needed.

Both `Streams` and `Mpesa` fall through to the existing wildcard. No explicit arms needed for them.

**Update after completion:**
- [x] Mark this task `[x]`
- [ ] Orchestrator: git commit after this task

---

### Task 02: Add 4 new table IDs to `SNAPSHOT_TABLE_ORDER`

**Files to modify:** `ledger/src/services/sync.rs`
**Reference files to read:** None needed.
**Depends on:** Task 01
**Parallel group:** P2

**Specification:**

In `src/services/sync.rs`, the `SNAPSHOT_TABLE_ORDER` constant (L614–626) drives which tables are dumped during initial cold-start sync. Add the 4 missing table IDs.

Add `31, 32` (subject_catalog, topics) near the beginning of the array — these are global catalogs that other tables (like `subject_teachers`, `papers`, `mastery`) reference. They should be synced **before** school-scoped data that depends on them.

Add `33` (streams) alongside the other school-structure tables (near enrollments/class_teachers).

Add `34` (mpesa) at the end alongside other school-config tables.

**Change the constant to:**

```rust
const SNAPSHOT_TABLE_ORDER: &[i32] = &[
    1, 2, 28, 26, // users, schools, plans, roles
    31, 32,       // subject_catalog, topics (global catalogs — before school data that references them)
    3, 7, 8,      // owners, teachers, staff
    4, 5, 6,      // students, guardians, departments
    27, 25,       // scopes, settings
    9,            // terms
    33,           // streams
    11, 10, 12,   // enrollments, class_teachers, subjects (subject_teachers)
    13, 14, 15,   // attendance, timetable, lessons
    16, 17, 18,   // exams, papers, grades
    19, 20, 21,   // fees, invoices, payments
    22, 23, 24,   // announcements, mastery, aiusage
    29, 30,       // subscriptions, discounts
    34,           // mpesa
];
```

**Update after completion:**
- [x] Mark this task `[x]`
- [ ] Orchestrator: git commit after this task

---

### Task 03: Exempt `subject_catalog` and `topics` from school-based filtering in `SyncFilter`

**Files to modify:** `ledger/src/services/sync.rs`
**Reference files to read:** None needed — all context inlined below.
**Depends on:** Task 01 (needs the new `LogTable` variants to exist)
**Parallel group:** P2 (can run in parallel with Task 02 since they edit different functions)

**Specification:**

`subject_catalog` and `topics` are global system-level tables (no school column). Every user — Normal, System, Super — must receive them on sync. Currently, `row_visible` returns `false` for `school_id: None` rows unless they are `Users` or `Plans`. We need to add `Subjects` (`Resource::Subjects`) to the exemption list.

The `Resource::Subjects` resource governs both `LogTable::SubjectCatalog` and `LogTable::Topics` (as specified in Task 01's `resource()` mapping).

**3a. In `table_visible` — System variant (around line 497):**

Current code has:
```rust
if table == LogTable::Users || resource == Resource::Plans {
    return true;
}
```

Change to:
```rust
if table == LogTable::Users || resource == Resource::Plans || resource == Resource::Subjects {
    return true;
}
```

**3b. In `table_visible` — Normal variant (around line 515):**

Current code has:
```rust
if table == LogTable::Users || resource == Resource::Plans {
    return true;
}
```

Change to:
```rust
if table == LogTable::Users || resource == Resource::Plans || resource == Resource::Subjects {
    return true;
}
```

**3c. In `row_visible` — System variant (around line 555):**

Current code has:
```rust
// Plans are always visible
if resource == Resource::Plans {
    return true;
}
```

Change to:
```rust
// Plans and global subject catalogs are always visible
if resource == Resource::Plans || resource == Resource::Subjects {
    return true;
}
```

**3d. In `row_visible` — Normal variant (around line 596):**

Current code has:
```rust
// Plans always visible
if resource == Resource::Plans {
    return true;
}
```

Change to:
```rust
// Plans and global subject catalogs are always visible
if resource == Resource::Plans || resource == Resource::Subjects {
    return true;
}
```

**Important safety note:** `LogTable::Subjects` (ID 12, the `subject_teachers` join table) maps to `Resource::Classes` via `LogTable::resource()` — NOT `Resource::Subjects`. So this change will NOT accidentally make `subject_teachers` globally visible. Only `SubjectCatalog` (31) and `Topics` (32) map to `Resource::Subjects`. These are different resources.

**Update after completion:**
- [x] Mark this task `[x]`
- [ ] Orchestrator: git commit after this task

---

### Task 04: Verify `should_rebuild_filter_record` needs no changes for new variants

**Files to modify:** `ledger/src/services/sync.rs` (only if a change is needed)
**Reference files to read:** None needed — context inlined.
**Depends on:** Task 01
**Parallel group:** P2 (can run in parallel with Tasks 02 and 03)

**Specification:**

The `should_rebuild_filter_record` function (L1017–1032) checks whether a changelog record should trigger a filter rebuild (used when membership/scope tables change). Currently it checks for specific `LogTable` values that affect user visibility (owners, teachers, staff, scopes, etc.).

Read the function and verify that none of the 4 new tables (`SubjectCatalog`, `Topics`, `Streams`, `Mpesa`) need to trigger a filter rebuild. They should NOT — none of them affect user memberships or role scopes. But confirm this by reading the function body and noting that no change is needed.

**If no change is needed:** Mark the task complete with a note saying "Verified — no change required."

**Verified — no change required.** None of `SubjectCatalog`, `Topics`, `Streams`, or `Mpesa` affect user memberships or role scopes. The function's wildcard `_ => false` arm already handles them correctly.

**Update after completion:**
- [x] Mark this task `[x]`
- [ ] Orchestrator: git commit after this task (only if changes were made)

---

### Task 05: Verify compilation and run tests

**Files to modify:** None (read-only verification)
**Depends on:** Tasks 01, 02, 03, 04
**Parallel group:** P3

**Specification:**

1. Run `cargo build` from the `ledger/` directory. Fix any compilation errors.
2. Run `cargo test` from the `ledger/` directory. All existing tests must pass.
3. Specifically verify:
   - The `test_filter_super_sees_all` test still passes.
   - The `test_filter_normal_sees_own_schools` test still passes.
   - The `test_filter_normal_sees_co_members` test still passes.
   - The `test_changelog_record_roundtrip` test still passes.

If there are compile errors related to the changes (e.g. non-exhaustive match patterns on `LogTable`), identify and fix them. Common things that might need fixing:
- Any `match` on `LogTable` elsewhere that doesn't have a wildcard `_` arm.

If any test fails, investigate and fix.

**Update after completion:**
- [x] Mark this task `[x]`
- [ ] Orchestrator: final git commit with message `fix: add missing tables (subject_catalog, topics, streams, mpesa) to sync engine`

---

### Execution Notes for Orchestrator

- **Tasks 01 and 02** both modify the same file (`src/services/sync.rs`) but touch completely different sections (enum/methods vs. a single constant). They CAN be done by the same executor sequentially, or Task 02 can be folded into Task 01 to avoid conflicts. **Recommended: assign Tasks 01 + 02 to a single executor as one batch.**
- **Tasks 03 and 04** also modify `src/services/sync.rs` and depend on Task 01. They touch different functions so can run in parallel, but since all four tasks edit the same file, **safest approach: run all of 01→02→03→04 sequentially with one executor.**
- **Task 05** is a verification step that runs after all code changes.
- Suggested commit messages:
  - After Tasks 01+02: `fix: add SubjectCatalog, Topics, Streams, Mpesa to LogTable and SNAPSHOT_TABLE_ORDER`
  - After Tasks 03+04: `fix: make subject catalog and topics globally visible in sync filter`
  - After Task 05 (if fixes needed): `fix: resolve compile/test issues from sync table additions`
