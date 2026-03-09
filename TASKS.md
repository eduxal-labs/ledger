# Ledger — Task Board

> Tasks are ordered by dependency and priority. Execute top-to-bottom.
> Detailed task specifications are in `../eduxal/LEDGER_TASKS.md`.
> That file is the source of truth — this file is a quick reference.

---

See `../eduxal/LEDGER_TASKS.md` for full self-sufficient task definitions.

- [ ] **L0:** Commit current server state
- [ ] **L1:** Write the new `sync.proto` (action-based messages)
- [ ] **L2:** Update proto adapter (`src/proto/services/sync.rs`)
- [ ] **L3:** Create action dispatcher (`src/db/database/tables/actions.rs`)
- [ ] **L4:** Implement handlers — Schools, Users, Settings, Plans
- [ ] **L5:** Implement handlers — Teachers, Staff, Owners, Guardians (invitation pattern)
- [ ] **L6:** Implement handlers — Students, Enrollments, Departments, Terms
- [ ] **L7:** Implement handlers — Classes, Attendance, Lessons, Exams, Grades
- [ ] **L8:** Implement handlers — Finance, Announcements, Roles, AI, Subscriptions, Discounts
- [ ] **L9:** Rewrite `services/sync.rs` push flow
- [ ] **L10:** Delete old `apply.rs` and clean up
- [ ] **L11:** Full build + smoke test
- [ ] **L12:** Commit the sync redesign
