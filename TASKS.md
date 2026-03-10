# Ledger — Task Board

> Tasks are ordered by dependency and priority. Execute top-to-bottom.
> Detailed task specifications are in `FULL.md`.
> That file is the source of truth — this file is a quick reference.

---

See `FULL.md` for full self-sufficient task definitions.

- [x] **L0:** Commit current server state
- [x] **L1:** Write the new `sync.proto` (action-based messages)
- [x] **L2:** Update proto adapter (`src/proto/services/sync.rs`)
- [x] **L3:** Create action dispatcher (`src/db/database/tables/actions.rs`)
- [x] **L4:** Implement handlers — Schools, Users, Settings, Plans
- [x] **L5:** Implement handlers — Teachers, Staff, Owners, Guardians (invitation pattern)
- [x] **L6:** Implement handlers — Students, Enrollments, Departments, Terms
- [x] **L7:** Implement handlers — Classes, Attendance, Lessons, Exams, Grades
- [x] **L8:** Implement handlers — Finance, Announcements, Roles, AI, Subscriptions, Discounts
- [x] **L9:** Rewrite `services/sync.rs` push flow
- [x] **L10:** Delete old `apply.rs` and clean up
- [x] **L11:** Full build + smoke test
- [ ] **L12:** Commit the sync redesign
