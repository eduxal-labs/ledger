# Ledger — Task Board

> Tasks are ordered by dependency and priority. Execute top-to-bottom.
> Detailed task specifications are inline below.
> Each task is self-sufficient for the executor agent.

---

### Task 01: Send final cursor as a bookmark delta after snapshot completes

**Files to modify:** `src/services/sync.rs`
**Reference files to read:** `src/services/sync.rs` (lines 639–775)
**Depends on:** None

**Problem:**
In `send_full_snapshot`, every `SyncDelta` is sent with `seq: 0`. After the snapshot completes, the `watch_loop` sets `cursor` to the current changelog head (e.g. `cursor = 768`), but this value is **never communicated to the client**. The client stores `lastSeq = 0` from the last received delta, so on reconnect it sends `lastSeq: 0` and triggers another full snapshot — an infinite loop. Worse, if no new changes occur after the snapshot, the client stays at `seq: 0` forever because the incremental loop only sends deltas when changelog records exist.

**Specification:**

In `watch_loop`, immediately after `send_full_snapshot` succeeds and the cursor/delete_cursor are updated (around line 773), send a **bookmark delta** — a `SyncDelta` with `seq` set to the new cursor value, `table: 0` (no valid table uses 0), `operation: OP_INSERT`, empty `row_key`, no `data`, no `file_urls`. This acts as a "snapshot complete" marker.

The client will receive this delta, see `seq > 0`, and persist it as `lastSeq`. On reconnect it will send this non-zero value and skip the full snapshot.

Add the following block in `watch_loop`, right after the line that logs `"snapshot done, cursor=..."` (around line 773), and before the `// --- Incremental sync loop ---` comment:

```rust
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
```

**No other files change.** The bookmark delta is a convention — `table: 0` is unused by `LogTable` (which starts at 1). The client already handles unknown tables gracefully (skips them in `DeltaWriter`), so it will simply persist the `seq` value.

**Validation:**
- After this change, a client connecting with `lastSeq: 0` will receive all snapshot deltas (with `seq: 0`) followed by one bookmark delta with `seq: <cursor>`.
- On reconnect the client sends `lastSeq: <cursor>` and enters the incremental loop directly.

**Update after completion:**
- [x] Mark this task `[x]`

---

### Task 02: Handle non-zero `lastSeq` that requires a full re-snapshot

**Files to modify:** `src/services/sync.rs`
**Reference files to read:** `src/services/sync.rs` (lines 748–775)
**Depends on:** Task 01

**Problem:**
Currently the full snapshot is only triggered when `cursor == 0`. But there are scenarios where a non-zero `lastSeq` should also trigger a full snapshot:

1. **`lastSeq` is misaligned** — the binary changelog uses fixed 24-byte records. If `lastSeq` is not a multiple of 24, `read_from` returns an error. The watch loop sends `Error::Internal` and exits.
2. **`lastSeq` exceeds the current changelog length** — this can happen if the changelog file was truncated (server restart with data loss) or the client has a stale cursor from a different server instance. `read_from` returns an empty vec, which is fine, but the client is now "ahead" of the server and will never receive historical data it missed.

**Specification:**

In `watch_loop`, after computing `cursor` from `last_cursor` (around line 750), add validation:

```rust
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
```

Place this block right after the `let mut cursor: u64 = ...` assignment and before the `let mut delete_cursor` line.

This way, an invalid non-zero cursor gracefully degrades to a full snapshot instead of crashing the stream.

**Update after completion:**
- [x] Mark this task `[x]`

---

### Task 03: Send bookmark delta when incremental batches produce no visible deltas

**Files to modify:** `src/services/sync.rs`
**Reference files to read:** `src/services/sync.rs` (lines 780–955)
**Depends on:** Task 01

**Problem:**
After an incremental batch is fully processed (upserts + deletes sent), the client needs to know the new cursor value to persist as `lastSeq`. Currently, each delta in the batch carries `seq: cursor` (the post-batch cursor), which is correct. But if a batch has zero visible deltas (all records were filtered out), the client never learns the new cursor. If the client disconnects and reconnects, it sends the old `lastSeq`, causing the server to re-read and re-filter the same changelog records. This is correct but wasteful — and on a server with heavy cross-school activity, a normal user could accumulate thousands of invisible records that get re-processed on every reconnect.

**Specification:**

In the incremental sync loop, after processing all upserts and deletes for a batch, check if any deltas were actually sent. If not, send a bookmark delta (same pattern as Task 01) so the client advances its cursor.

Add a counter at the start of the incremental processing (right before the `table_min_ts` HashMap, around line 827):

```rust
let mut deltas_sent: usize = 0;
```

Increment `deltas_sent` each time a delta is successfully sent (in both the upsert loop around line 876 and the delete loop around line 916):

```rust
// After each successful tx.send(Ok(delta)).await:
deltas_sent += 1;
```

After the delete processing loop and before the filter rebuild check (around line 940), add:

```rust
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
```

**Update after completion:**
- [x] Mark this task `[x]`

---

### Task 04: Build and verify compilation

**Files to modify:** None (verification only)
**Depends on:** Tasks 01, 02, 03

**Specification:**

Run `cargo build` and verify the project compiles without errors. If there are any compilation issues introduced by Tasks 01–03, fix them.

Check specifically:
1. The `SyncDelta` struct usage in the bookmark deltas (ensure all fields are present)
2. The `warn!` macro import is available (it's already imported at line 22)
3. The `LOG.with(...)` call for `changelog_len` in Task 02 works in the async context (it does — `LOG` is thread-local and the `.with()` call is synchronous)

Run: `cargo build 2>&1 | head -50`

**Update after completion:**
- [x] Mark this task `[x]`

---

### Task 05: Commit the WatchChanges initial sync fix

**Files to modify:** None (git operations only)
**Depends on:** Task 04

**Specification:**

Stage and commit all changes with:

```
git add -A
git commit -m "fix: WatchChanges sends bookmark delta after snapshot for correct lastSeq tracking

- Send a bookmark SyncDelta (table=0, seq=cursor) after the full
  snapshot completes so the client can persist a non-zero lastSeq.
  Previously all snapshot deltas had seq=0, causing the client to
  request a full snapshot on every reconnect.

- Validate lastSeq on stream open: misaligned or ahead-of-head
  cursors gracefully fall back to a full snapshot instead of
  crashing the stream with an internal error.

- Send a bookmark delta when incremental batches produce no visible
  deltas, so the client advances its stored cursor past filtered
  changelog records.

Fixes: Issue 001 (WatchChanges initial sync)"
```

**Update after completion:**
- [ ] Mark this task `[x]`
