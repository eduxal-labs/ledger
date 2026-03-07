# AGENT.md — Guide for AI Agents Working on Ledger

This document describes the architecture, conventions, and patterns used in this Rust codebase. Follow these rules precisely when making changes or adding features.

## Project Overview

Ledger is a gRPC authentication and identity microservice for the EduxaL education platform. It is written in Rust (edition 2024, nightly) using Tonic for gRPC, Diesel for SQLite ORM, PASETO v4 for token encryption, and WhatsApp Business API for OTP delivery.

The server listens on `127.0.0.1:50051`.

## Build & Run

```sh
# All env vars are compile-time — changes require a full rebuild
cargo run

# Run database migrations (only if schema changed)
diesel migration run

# Regenerate schema.rs after migration changes
diesel print-schema > src/db/schema/schema.rs
```

Required `.env` file (read at compile time by `build.rs`):

```
PASETO_PASSWORD=...
WHATSAPP_TOKEN=...
R2_ACCOUNT_ID=...
R2_BUCKET=...
R2_ACCESS_KEY_ID=...
R2_SECRET_ACCESS_KEY=...
DATABASE_URL=database.db
```

> **Critical:** Environment variables are injected via `build.rs` using `cargo:rustc-env` and consumed via `env!()` macros. They are baked into the binary. A code change alone won't pick up new env values without `cargo clean` or touching the files that use them.

## Project Structure

```
ledger/
├── build.rs                        # Compiles .proto files, loads .env at build time
├── Cargo.toml                      # Edition 2024 (nightly), links local `macros` crate
├── diesel.toml                     # Points schema output to src/db/schema/schema.rs
├── macros/                         # Proc-macro crate (key!, Count)
├── migrations/                     # Diesel SQL migrations
├── protos/                         # Protobuf definitions (source of truth for gRPC API)
│   ├── services/*.proto            #   Service definitions (RPCs)
│   └── types/*.proto               #   Shared message/enum types
└── src/
    ├── main.rs                     # Entrypoint: declares modules, calls server::start()
    ├── server.rs                   # Tonic server wiring
    ├── config/                     # External integrations (WhatsApp, OTP engine, R2 signing)
    ├── db/                         # Database layer (Diesel, SQLite)
    │   ├── schema/                 #   Auto-generated Diesel schema (DO NOT HAND-EDIT)
    │   └── database/               #   Connection pool, traits, table impls
    ├── proto/                      # Generated proto wrappers + hand-written service traits
    ├── services/                   # Business logic implementations
    └── types/                      # Domain types (Id, Phone, Token, Error, User, etc.)
```

## Layered Architecture

The codebase follows a strict layered architecture. Understand these layers and their responsibilities:

```
    proto/services/       Thin gRPC adapter — parses requests, calls service methods, maps errors
         │
    services/             Business logic — orchestrates config + database operations
         │
    ┌────┴────┐
 config/    db/database/  Infrastructure — external APIs, OTP engine, SQLite CRUD
    │
    ├── messenger.rs      Trait for sending codes (impl: whatsapp.rs)
    ├── verifyer.rs       Trait for OTP lifecycle (impl: verifications.rs)
    └── storage/sign.rs   R2 presigned URL generation
```

### Data flow for a new feature

1. **Define the proto** in `protos/` (types and/or service RPCs)
2. **Register the proto** in `build.rs` if it's a new service file
3. **Add domain types** in `src/types/` with `FromStr`, `From`, Diesel derives
4. **Add database operations** in `src/db/database/tables/` implementing the generic traits
5. **Add config traits/impls** in `src/config/` if external integrations are needed
6. **Implement the service** in `src/services/` using config traits + database
7. **Wire the proto adapter** in `src/proto/services/` bridging gRPC to your service
8. **Register the service** in `src/server.rs`

## Key Patterns & Conventions

### 1. Trait-based abstraction for services

Services don't depend on concrete config types. They depend on trait bounds.

```rust
// src/config/mod.rs — Config is a trait alias composing sub-traits
pub trait Config: Messenger<Recipient = Phone> + Verifyer {}
impl Config for Configuration {}

// src/services/authentication.rs — generic over any Config impl
pub struct Authenticator<C> {
    config: Arc<C>,
}

impl<C: Config + Send + Sync + 'static> Authentication for Authenticator<C> { ... }
```

When adding a new service that needs new capabilities:
1. Define the capability as a trait in `src/config/`
2. Add the trait as a supertrait of `Config` (or create a new composite trait)
3. Implement it on `Configuration` by delegating to a concrete implementation field

### 2. The proto adapter pattern

Each gRPC service has a **hand-written trait** in `src/proto/services/` that defines the clean Rust interface, plus a `#[tonic::async_trait] impl` block that adapts tonic `Request<T>` → domain types → `Response<T>`.

```rust
// src/proto/services/authentication.rs

// 1. Clean trait with domain types (no tonic types)
pub trait Authentication: Sync + Send + 'static + Sized {
    type Config: Sync + Send + 'static;
    fn new(config: Self::Config) -> AuthenticationServer<Self>;
    fn login(&self, phone: Phone) -> impl Future<Output = Result<Verification>> + Send;
    // ...
}

// 2. Blanket adapter impl for tonic (parses strings → domain types via .parse())
#[tonic::async_trait]
impl<T: Authentication> authentication_server::Authentication for T {
    async fn login(&self, request: Request<Login>) -> Result<Response<...>, Status> {
        let phone = request.into_inner().phone.parse()?;  // FromStr → Error → Status
        let verification = self.login(phone).await?;
        Ok(Response::new(verification.into()))             // From<Domain> for Proto
    }
}
```

**Key rule:** The proto adapter does ALL parsing. The service layer receives only validated domain types. Never pass raw strings into service methods.

### 3. `FromStr` / `.parse()` everywhere

All domain types implement `FromStr` with `Err = Error`. This is how proto string fields become typed values:

| Type | Parses from | Example |
|------|-------------|---------|
| `Id` | 24-char hex (BSON ObjectId) | `"683d5a1b4f2e7c0019abcdef"` |
| `Phone` | Kenyan phone (`+254...`, `0...`, `254...`, 9-digit) | `"+254759762268"` → `Phone("0759762268")` |
| `Token` | PASETO v4 local encrypted string | `"v4.local.xxx..."` |
| `Code` | 6 ASCII digits | `"482910"` |

When adding new types, always implement `FromStr` with `type Err = Error` so `.parse()?` works in the proto adapter layer.

### 4. Error handling

There is one unified `Error` enum in `src/types/error.rs`. Every layer uses `Result<T, Error>`.

```rust
pub type Result<T, E = Error> = std::result::Result<T, E>;
```

Errors convert to gRPC status codes via `From<Error> for tonic::Status`:
- `Error::InvalidPhone` → `Status::invalid_argument`
- `Error::UserNotFound` → `Status::not_found`
- `Error::Forbidden` → `Status::permission_denied`
- `Error::Conflict` → `Status::already_exists`
- `Error::Internal` → `Status::internal`

**Rules:**
- Add new variants to the `Error` enum, not new error types
- Always add a corresponding `Status` mapping in the `From<Error> for Status` impl
- Use `Error::internal(err)` for logging + converting unexpected errors (it calls `tracing::error!`)
- Never expose internal details in error messages to the client

### 5. Database pattern — generic traits with thread-local connections

Database access uses three generic traits in `src/db/database/traits.rs`:

```rust
pub trait Create<I, O = I> { fn create(&mut self, record: I) -> Result<O>; }
pub trait Find<K, V>       { fn find(&mut self, key: K) -> Result<Option<V>>; }
pub trait Update<K, U, V>  { fn update(&mut self, key: K, record: U) -> Result<V>; }
```

These are implemented on `diesel::SqliteConnection` in `src/db/database/tables/*.rs`, one file per table.

The connection is accessed via a thread-local:

```rust
// In service code:
use crate::db::database::CONN as conn;
use crate::db::database::traits::Database;

let user: Option<User> = conn.find(phone)?;      // Find<Phone, User>
let user: User = conn.create(new_user)?;          // Create<User>
let user: User = conn.update(id, changeset)?;     // Update<Id, user::Update, User>
```

**When adding a new table:**
1. Write the migration SQL in `migrations/`
2. Run `diesel migration run` then `diesel print-schema > src/db/schema/schema.rs`
3. Create the Rust struct in `src/types/` with Diesel derives (`Queryable`, `Insertable`, `Selectable`, `QueryableByName`)
4. Create an `Update` struct with `AsChangeset` and `Default` (all fields `Option<T>`)
5. Create `src/db/database/tables/<name>.rs` implementing `Create`, `Find`, `Update` on `Conn`
6. Register the module in `src/db/database/tables/mod.rs`

### 6. User model pattern

The `User` type demonstrates the full pattern for a database-backed entity:

| File | Contains |
|------|----------|
| `src/types/user/user.rs` | `User` struct with Diesel derives + `From<User> for proto::User` |
| `src/types/user/update.rs` | `Update` changeset (all fields optional, `AsChangeset`, `Default`) |
| `src/types/user/level.rs` | `Level` enum with `TryFrom<i32>`, `From<Level> for i32`, Diesel `ToSql`/`FromSql` |
| `src/types/user/status.rs` | `Status` enum (same pattern as Level) |
| `src/types/user/mod.rs` | Re-exports everything via `pub use` |

Follow this exact pattern for new entities. Enums that map to `smallint` columns need all four impls: `TryFrom<i32>`, `From<Enum> for i32`, `ToSql<SmallInt>`, `FromSql<SmallInt>`.

### 7. Proto type conversions

Domain types convert to proto types via `From`:

```rust
impl From<User> for crate::proto::types::user::User { ... }
impl From<Verification> for crate::proto::types::verification::Verification { ... }
```

Proto types are accessed as `crate::proto::types::<package>::<Type>`. The package name comes from the `package` declaration in the `.proto` file.

### 8. Protobuf conventions

- **Type protos** go in `protos/types/<name>.proto` — one file per domain entity
- **Service protos** go in `protos/services/<name>.proto` — one file per service
- Service protos import types via `import "types/<name>.proto";`
- All proto files must be registered in `build.rs` `.compile_protos()` if they define services
- Type-only protos are discovered automatically via imports

### 9. Configuration / integration pattern

External integrations follow this pattern:

1. **Define a trait** (`src/config/messenger.rs`, `src/config/verifyer.rs`)
2. **Implement it on a concrete struct** (`whatsapp.rs`, `verifications.rs`)
3. **Implement the trait on `Configuration`** by delegating to the inner field
4. **Compose traits** into `Config` supertrait (`pub trait Config: Messenger + Verifyer {}`)

The `Configuration` struct in `src/config/mod.rs` holds all concrete implementations as fields and delegates each trait to the corresponding field.

### 10. Verification engine (in-memory, channel-based)

The OTP system runs on a **dedicated OS thread** (not the Tokio runtime). Communication uses:
- `crossbeam::channel` — bounded(200) for sending commands
- `tokio::sync::oneshot` — for receiving responses back into async code

Commands: `Request`, `Verify`, `Delete`

This design is intentional — it avoids async mutex contention on the hot OTP state. Do not refactor this to use async mutexes.

### 11. Token system

PASETO v4 local (symmetric encryption, not signing). Three token purposes:

| Purpose | TTL | Use |
|---------|-----|-----|
| `Access` | 3 days | API authentication |
| `Refresh` | 30 days | Token renewal |
| `Setup` | 1 hour | First-time registration |

The 32-byte key is derived at **compile time** via the `macros::key!` proc-macro using Argon2id. The key is a `const` embedded in the binary.

### 12. Module visibility

- `mod` (private) for internal implementation details (`messenger.rs`, `whatsapp.rs`, `verifyer.rs`, `verifications.rs`)
- `pub mod` for things other modules need to import (`storage`, `database`, `schema`, `services`, `types`)
- In type modules, use `pub use *;` re-exports in `mod.rs` to flatten the namespace

### 13. `OnConflict` / `Conflict` trait

`src/types/error.rs` defines a pattern for handling database unique constraint violations:

```rust
// Silently resolve conflicts (upsert-like behavior)
conn.create(user).resolve()?;       // Ok(()) if conflict, Err if other error

// Replace the conflict error with a domain-specific one
conn.create(user).on_conflict(Error::UserAlreadyExists)?;
```

Use `.resolve()` for idempotent seed operations. Use `.on_conflict(err)` for user-facing operations where conflicts should return specific errors.

## Naming Conventions

| What | Convention | Example |
|------|-----------|---------|
| Files | snake_case, singular | `authentication.rs`, `user.rs` |
| Modules | snake_case | `mod authentication;` |
| Structs | PascalCase, singular | `User`, `Authenticator`, `Verification` |
| Enums | PascalCase | `Status`, `Purpose`, `Level` |
| Enum variants | PascalCase | `Status::Active`, `Purpose::ChangePhone` |
| Traits | PascalCase, adjective/noun | `Messenger`, `Verifyer`, `Database`, `Config` |
| Database tables | snake_case, plural | `users`, `schools`, `enrollments` |
| Proto packages | lowercase, singular | `package user;`, `package authentication;` |
| Proto files | snake_case, singular | `authentication.proto`, `user.proto` |
| Constants | SCREAMING_SNAKE_CASE | `const TTL: i32 = 15 * 60;` |

## Database Conventions

### SQL schema rules

- `id` columns are `text` (BSON ObjectId hex strings)
- DateTime is `bigint` (seconds since Unix epoch)
- Date is `integer` (days since Unix epoch)
- Enums are `smallint` with comments documenting variants
- Composite primary keys are used extensively (no surrogate IDs for join/junction tables)
- Foreign keys always specify `ON DELETE` behavior
- `created` and `updated` default to `unixepoch('now')`
- Use `CHECK` constraints for data integrity

### Diesel model rules

- `Queryable`, `Selectable`, `QueryableByName`, `Insertable` on the main struct
- `AsChangeset` + `Default` on the `Update` struct (all `Option` fields)
- `#[diesel(table_name = <table>)]` attribute on both
- `AsExpression`, `FromSqlRow` on custom column types (Id, Phone, enums)
- Custom `ToSql` / `FromSql` impls for types that don't map directly to SQL primitives

## Adding a New gRPC Service (Step by Step)

1. **Proto definition:**
   - Create `protos/types/<entity>.proto` if new types are needed
   - Create `protos/services/<service>.proto` with the service RPCs
   - Register the service proto in `build.rs`: add it to the `.compile_protos()` call

2. **Domain types** (`src/types/`):
   - Create `src/types/<entity>/` directory with `mod.rs`, main struct, enums, update changeset
   - Implement `FromStr`, `From`/`TryFrom` conversions, Diesel derives, Serde derives
   - Add `pub mod <entity>;` to `src/types/mod.rs`

3. **Proto wrappers** (`src/proto/`):
   - Add `pub mod <package> { tonic::include_proto!("<package>"); }` to `src/proto/types/mod.rs`
   - Create `src/proto/services/<service>.rs` with:
     - The clean Rust trait (domain types only, no tonic types)
     - The `#[tonic::async_trait]` adapter impl
     - Any response constructor impls (`Authenticated::new`, etc.)
   - Register in `src/proto/services/mod.rs`

4. **Database** (`src/db/database/tables/`):
   - Create `src/db/database/tables/<entity>.rs`
   - Implement `Create`, `Find`, `Update` on `diesel::SqliteConnection`
   - Register in `src/db/database/tables/mod.rs`

5. **Config traits** (`src/config/`) — only if external integrations are needed:
   - Define the trait in a new file
   - Implement on a concrete struct
   - Add field to `Configuration`, delegate the trait
   - Add as supertrait to `Config` if the service needs it

6. **Service implementation** (`src/services/`):
   - Create `src/services/<service>.rs` with the service struct
   - Implement the proto trait, using `conn.find()`, `conn.create()`, etc.
   - Register in `src/services/mod.rs`

7. **Wire it up** (`src/server.rs`):
   - Import the service and its proto trait
   - Add `.add_service(...)` to the `Server::builder()` chain

## Common Pitfalls

1. **Don't edit `src/db/schema/schema.rs`** — It's auto-generated by Diesel CLI. Edit migrations and run `diesel print-schema`.

2. **Don't use `async` mutexes for shared mutable state** — The project deliberately uses OS threads + channels for concurrent mutable state (see `verifications.rs`). Follow this pattern.

3. **Don't forget the `From<Error> for Status` mapping** — Every new `Error` variant must have a corresponding gRPC status mapping or the code won't compile.

4. **All env vars are compile-time** — `env!()` reads at compile time. After changing `.env`, you need `cargo clean && cargo build` or at minimum touch the files that use the changed variables.

5. **`CONN` is thread-local** — Each thread gets its own SQLite connection. This is intentional for avoiding mutex contention. Don't try to share connections across threads.

6. **Phone numbers are Kenyan format** — The `Phone` type normalizes to `0XXXXXXXXX` (10 bytes). Don't assume international format.

7. **Don't add new error types** — Use the single `Error` enum. Add variants to it.

8. **Proto types vs Domain types** — Never use proto-generated types in service logic. Always convert at the proto adapter boundary via `From`/`Into`.

9. **`build.rs` only compiles service protos** — Type-only `.proto` files are picked up transitively via imports. Only `.proto` files that define `service` blocks need to be listed in `compile_protos()`.

10. **The `macros` crate is a local dependency** — It's at `./macros` and provides `key!` and `Count`. It runs at compile time with full access to environment variables.

## Testing

Tests are co-located in the same files using `#[cfg(test)] mod tests { ... }`. See `src/types/phone.rs` and `src/config/storage/sign.rs` for examples. When writing tests:

- Test `FromStr` / `.parse()` for valid and invalid inputs
- Test enum round-trips (`i32` → `Enum` → `i32`)
- Use `assert_eq!` / `assert_ne!` / `.unwrap()` / `.unwrap_err()`
- No external test framework — use standard `#[test]` functions

## Commit Conventions

Follow conventional commits:

- `feat:` — New feature
- `fix:` — Bug fix
- `chore:` — Dependencies, config, non-code changes
- `docs:` — Documentation only
- `refactor:` — Code restructuring without behavior change

Write detailed commit bodies explaining **what** was added and **why**, not just file lists. Group related changes into logical commits.
---

## Two-Agent Workflow

This project uses two distinct agent roles that share this `AGENT.md` as their common rule book.

### Orchestrator Agent

**Trigger:** The user describes a feature, change, bug fix, or any non-trivial request.

**Responsibilities:**
1. Read `AGENT.md` in full.
2. Read relevant source files in `src/` to understand current project state.
3. Read `migrations/` if the work touches the database layer.
4. Ask the user as many clarifying questions as needed — do not guess.
5. Produce a detailed, ordered task list and **insert it into `TASKS.md`** at the appropriate position (based on priority/dependency).
6. Remove completed tasks from `TASKS.md` when the user confirms they are done or when the executor marks them.
7. Each task MUST be **self-sufficient** for the executor — see task format below.

**The orchestrator never writes application code.** It only writes tasks.

### Executor Agent

**Trigger:** The user says simple words like "continue", "go ahead", "next", or similar.

**Responsibilities:**
1. Read `AGENT.md` in full.
2. Read `TASKS.md` — pick up the first unchecked `[ ]` task.
3. The task itself should contain everything needed. If clarification is needed, read the source file(s) referenced in the task. Avoid exploring the broader codebase.
4. Execute the task exactly as specified.
5. Mark the task as `[x]` in `TASKS.md`.
6. If the task list is now empty (all done), delete all content from `TASKS.md` except the header.

**The executor never invents new tasks or architectural decisions.** It executes what the orchestrator wrote.

### Self-Sufficient Task Format

Every task in `TASKS.md` must follow this structure so the executor can work without exploration:

```
### Task XX: <Title>
**Files to create/modify:** `src/path/to/file.rs`
**Reference files to read:** `../server/src/path/to/reference.rs` (if porting)
**Depends on:** Task YY (if any)

**Specification:**
Exact description with method signatures, struct definitions, imports.
If porting from ../server/, specify what to copy vs what to change.

**Update after completion:**
- [ ] Mark this task `[x]`
```

---

## Reference: `../server/` as Pattern Source

The old server at `../server/` (pre-local-first implementation) contains patterns and types that must be ported into this codebase. It is **NOT the active backend** — it is reference code only.

| File in `../server/` | What to port | Notes |
|---|---|---|
| `src/types/role/permissions.rs` | `Permissions` struct — array of `Actions` per `Resource`, binary serialization (`ToSql`/`FromSql` for `Binary`), operator overloading (+, -, +=, -=), proto conversion | Adapt from `u8` Actions to `u16` Actions. Binary encoding changes from 2-byte pairs to 3-byte pairs. |
| `src/types/role/action.rs` | `Action` enum (bitmask values) + proto conversions | Expand from 5 actions to 9. Change `repr(u8)` to `repr(u16)`. |
| `src/types/role/actions.rs` | `Actions` bitmask wrapper with `contains`/`iter`/operators | Change inner type from `u8` to `u16`. |
| `src/types/role/resource.rs` | `Resource` enum + `Count` derive + proto conversions + `TryFrom<u8>` | Expand from 5 resources to 18. |
| `src/types/role/organisation.rs` | `Organisation` enum (System, Account, School(Id)) + `FromStr` | Port as-is. |
| `src/types/role/role.rs` | `Role` struct (Diesel Queryable), `Reference`, `Assigner`, `Update`, `Assignment` | Adapt for new `Permissions` type (blob, not text). |
| `src/types/member/role.rs` | `Role`/`Roles` bitmask (Owner=1, Guardian=2, Student=4, Teacher=8, Staff=16) | Port as-is. |
| `src/types/member/membership.rs` | `Membership` struct (school id, name, roles bitmask) | Port as-is. |
| `src/db/database/authorize.rs` | `Authorize` trait impl — Super bypass, system role loading, school+system role merging | Port as-is, matching the three-tier permission model. |
| `src/db/database/traits.rs` | Extended traits: `Load`, `Authorize`, `List`, `Search`, `Delete`, `Purge` + `Database` blanket impl | Ledger currently only has `Create`, `Find`, `Update`, `Database`. Port the rest. |

**Preservation rule:** The owner's code is handcrafted and clean. Agents must match the established patterns: operator overloading for bitmask types, trait-based DB abstraction, proto adapter pattern, `FromStr` everywhere, single `Error` enum. Read the actual `../ledger/` source files to absorb the style before writing.

---

## Three-Tier Permission Model

### The Three User Levels

```
enum UserLevel { normal = 0, system = 1, super_ = 2 }
```

### Super Users (level = 2) — Unrestricted God Mode

- **See everything.** No filtering on sync. Receive all `server_logs` entries for all tables, all schools.
- **Write anything.** No permission checks on push. Can write to any table, any school, any system table.
- **Bypass all authorization.** The `Authorize` trait returns `Ok(())` immediately for Super users.
- **Can see deleted records** in the system dashboard (only level that can).

### System Users (level = 1) — Globally Scoped but Role-Gated

- **NOT full access.** System users have permissions determined by their assigned system-level roles (where `scopes.school IS NULL`).
- **Not restricted to any specific school** — if a system user has `Read` on `Schools`, they see ALL schools. If they have `Read` on `Students`, they see ALL students across all schools. The "system" part means their scope is global, not school-bound.
- **But only see resources their roles grant.** A system user without `Students.Read` does NOT receive student data during sync.
- **Can also be school members.** A system user who is also a teacher at School A gets their system-level roles PLUS their school-level roles for School A merged together:

```rust
// From ../server/src/db/database/authorize.rs — the pattern to follow
Organisation::School(id) => {
    let mut roles = self.load((id, &user))?;  // school-scoped roles
    if user.level == Level::System {
        let system = self.load(&user)?;         // system-scoped roles (school IS NULL)
        roles.extend(system);                   // merge both
    }
    // ... then check required permissions against merged roles
}
Organisation::System => self.load(&user),       // system roles only
```

- **Cannot see deleted records** in the system dashboard.

### Normal Users (level = 0) — Membership-Based

- **Only see schools where they are a member** (owner, teacher, staff, student, or guardian).
- **See their own user row** always.
- **See co-members** (other users who share a school with them).
- **See all plans** (global, needed for subscription UI).
- **Do NOT see system roles/scopes.**
- **Within a school:** currently, membership = full school visibility. Fine-grained per-table filtering deferred.

### Server Permission Checking Flow

```
1. Parse access_token from gRPC metadata → get user_id
2. Load user → check level:
   - Super → skip all checks, grant full access
   - System → load system-scoped roles, check resource+action against aggregated permissions
   - Normal → check school membership + (future) school-scoped role permissions
3. For school context: System users get school roles + system roles merged
4. Subtract required permissions from aggregated permissions
5. If remaining permissions are empty → authorized. Otherwise → Error::Forbidden
```

---

## Resource & Action Design

### Design Principles

1. **Resources are logical domain entities**, NOT a 1:1 mapping to the 30 database tables. Join/junction tables are absorbed as actions on their parent resource.
2. **Actions expand beyond CRUD.** Relationship operations like Assign, Revoke, Enroll become actions, not separate resources.
3. **Actions apply bidirectionally.** "Assign a teacher to a subject" = "assign a subject to a teacher" — same join operation. The resource is whichever entity makes sense from both perspectives.
4. **Not every action applies to every resource.** The UI shows only relevant actions per resource. The bitmask uses the same bit positions globally.
5. **Members are split** into separate resources (Owners, Teachers, Staff) — because a role that can add teachers should not automatically be able to add owners.

### Action Enum (u16 bitmask)

| Action | Bit | Description |
|---|---|---|
| Create | 0 | Create a new record |
| Read | 1 | View records |
| Update | 2 | Modify existing records |
| Delete | 3 | Soft-delete / deactivate |
| Purge | 4 | Permanent delete (Super-only) |
| Assign | 5 | Add a relationship (enroll student, assign teacher to subject, assign role to user) |
| Unassign | 6 | Remove a relationship (unenroll, unassign teacher, revoke role) |
| Mark | 7 | Record data (attendance, scores) |
| Approve | 8 | Approve/verify (payments, workflows) |

Bits 9-15 are reserved for future expansion. `repr(u16)` on `Action`, inner type of `Actions` is `u16`.

### Resource Enum (~18 logical resources)

| # | Resource | Covers tables | Notable non-CRUD actions |
|---|---|---|---|
| 1 | Users | `users` | Level/status changes |
| 2 | Schools | `schools`, `settings` | |
| 3 | Owners | `owners` | |
| 4 | Teachers | `teachers` | |
| 5 | Staff | `staff` | |
| 6 | Students | `students`, `guardians` | Assign (enroll → `enrollments`), Unassign (unenroll) |
| 7 | Departments | `departments` | |
| 8 | Classes | `class_teachers`, `subjects`, `timetable` | Assign (teacher→subject, teacher→class, timetable entry), Unassign |
| 9 | Attendance | `attendance` | Mark |
| 10 | Lessons | `lessons` | |
| 11 | Exams | `exams`, `papers` | |
| 12 | Grades | `grades`, `mastery` | Mark |
| 13 | Fees | `fees`, `invoices` | |
| 14 | Payments | `payments` | Approve |
| 15 | Announcements | `announcements` | |
| 16 | Roles | `roles`, `scopes` | Assign (role→user), Unassign (revoke) |
| 17 | Plans | `plans`, `subscriptions`, `discounts` | |
| 18 | AI | `aiusage` | |

### Permissions Storage Format

The `Permissions` struct is an array of `Actions` (u16 bitmask) indexed by `Resource`:

```rust
struct Permissions([Actions; Resource::COUNT]);  // ~18 resources × 2 bytes = 36 bytes max
```

**Database storage:** Binary blob — sparse encoding:
- 3 bytes per non-empty resource: `[resource_id: u8, actions_lo: u8, actions_hi: u8]` (little-endian u16)
- Empty resources are skipped
- Max size: 18 resources × 3 bytes = 54 bytes (but typically much smaller)
- `roles.permissions` column: `blob` type (NOT `text`)
- `ToSql`/`FromSql` impls follow the same pattern as `../server/src/types/role/permissions.rs` but with 3-byte tuples instead of 2-byte

**Proto wire format:** `repeated Permission { Resource resource = 1; repeated Action actions = 2; }`

### Action Context Per Resource (UI Display)

| Resource | Shown Actions |
|---|---|
| Users | Read, Update, Delete |
| Schools | Create, Read, Update, Delete |
| Owners | Create, Read, Delete |
| Teachers | Create, Read, Update, Delete |
| Staff | Create, Read, Update, Delete |
| Students | Create, Read, Update, Delete, Assign, Unassign |
| Departments | Create, Read, Update, Delete |
| Classes | Create, Read, Update, Delete, Assign, Unassign |
| Attendance | Read, Mark |
| Lessons | Create, Read, Update, Delete |
| Exams | Create, Read, Update, Delete |
| Grades | Read, Mark, Update, Delete |
| Fees | Create, Read, Update, Delete |
| Payments | Create, Read, Update, Delete, Approve |
| Announcements | Create, Read, Update, Delete |
| Roles | Create, Read, Update, Delete, Assign, Unassign |
| Plans | Create, Read, Update, Delete |
| AI | Read, Update |

Purge is never shown in the UI — implicitly granted to Super users only.

---

## Sync Engine Specification

### Two gRPC Streams

```protobuf
service Sync {
  rpc PushChanges (stream MutationBatch) returns (stream PushAck);
  rpc WatchChanges (WatchRequest) returns (stream SyncDelta);
}
```

- **PushChanges:** Client sends batches of local mutations. Server validates permissions, applies to DB, logs to `server_logs`, returns per-mutation results.
- **WatchChanges:** Client sends `WatchRequest{last_seq}`. Server streams all `server_logs` entries with `seq > last_seq`, filtered by the user's permissions. Keeps stream open for real-time push.

### Change Tracking

- **Server:** `server_logs` table with auto-incrementing `seq`, `user_id`, `tbl`, `op`, `row_key`, `row_data` (JSON), `school_id`.
- **Client:** `logs` table queues local mutations. `accounts.lastSeq` tracks server cursor.

### File Sync via S3

Files piggyback on data push/watch streams. Client logs a mutation on the parent record → server detects file-bearing record → returns presigned PUT URL in `PushAck` → client uploads via HTTP PUT → server notifies other clients via `SyncDelta` with GET URLs.

### Conflict Resolution

Last-write-wins by arrival order. Server always applies. Result propagates to all clients.

### Initial Sync (Cold Start)

Client sends `WatchRequest{last_seq: 0}`. Server sends ALL visible data as Insert deltas. Client writes in batched transactions (100 rows per batch).

### Proto Message Structure

```protobuf
// Client → Server
message MutationBatch {
  string batch_id = 1;            // UUID for idempotency
  repeated Mutation mutations = 2;
}
message Mutation {
  int32 table = 1;                // LogTable enum (0-29)
  int32 operation = 2;            // 0=Insert, 1=Update, 2=Delete
  string row_key = 3;             // "|"-delimited PK values
  optional int32 columns = 4;     // Bitmask of changed columns (Update only)
  RowData data = 5;               // The row data (Insert/Update only)
}

// Server → Client (push ack)
message PushAck {
  string batch_id = 1;
  bool success = 2;
  optional string error = 3;
  int64 server_seq = 4;           // Sequence number after applying batch
  repeated MutationResult results = 5;
}
message MutationResult {
  int32 index = 1;                // Index in the batch
  bool success = 2;
  optional string error = 3;
  int32 code = 4;                 // 0=ok, 1=permission_denied, 2=conflict, 3=validation, 4=not_found
  repeated FileUrl file_urls = 5; // Presigned URLs for file-bearing records
}

// Client → Server (watch request)
message WatchRequest { int64 last_seq = 1; }

// Server → Client (streaming deltas)
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

// RowData = oneof with 30 per-table *Row messages
// Each *Row has all columns as optional proto3 fields matching schema types.
```

### `server_logs` Table

```sql
CREATE TABLE server_logs (
    seq       INTEGER PRIMARY KEY AUTOINCREMENT,
    user_id   TEXT    NOT NULL,
    tbl       SMALLINT NOT NULL,       -- LogTable enum (0-29)
    op        SMALLINT NOT NULL,       -- 0=Insert, 1=Update, 2=Delete
    row_key   TEXT    NOT NULL,
    row_data  TEXT,                     -- Full JSON of the row (null for Delete)
    school_id TEXT,                     -- NULL for system tables (users, plans, roles where school IS NULL)
    created   BIGINT  NOT NULL DEFAULT (unixepoch('now'))
);
```

`school_id` enables permission filtering: Super users see all rows. System users see rows for resources they have Read access on (globally). Normal users see rows where `school_id IN (their_schools)` plus system-table rules (own user row, all plans).

### Sync Permission Filtering

**For WatchChanges (What Data to Send):**

| User Level | Filtering Rule |
|---|---|
| Super | `WHERE seq > last_seq` — no filter, send everything |
| System | For each resource the user has `Read` on: send ALL records globally for that resource's tables. If the user also has school memberships, additionally send school-scoped data for resources they DON'T have system-level Read on. |
| Normal | Send data only for schools where user is a member. Plus own user row. Plus all plans. No system roles/scopes. |

**For PushChanges (What Mutations to Accept):**

| User Level | Validation Rule |
|---|---|
| Super | Accept everything. No checks. |
| System | Check system-scoped roles for the required resource+action. If writing to a school-scoped table, also check school membership OR system-level permission for that resource. |
| Normal | Check school membership. Must be a member of the school referenced in the mutation. (Future: check school-scoped role permissions.) |

**Error Codes for Failed Push Mutations:**

| Code | Meaning | Client Action |
|---|---|---|
| 0 | Success | Delete log entry |
| 1 | Permission denied | Mark failed, show in notifications |
| 2 | Conflict | Apply server version, delete log |
| 3 | Validation error | Mark failed, user must fix |
| 4 | Not found | Mark failed (updates), delete (deletes) |

---

## Division of Labour

| Agent | Owns |
|---|---|
| **Server agent** | All Rust code in this repo: `src/`, `protos/`, `migrations/`, `build.rs` |
| **Client agent** | All Dart code in `../eduxal/`: `lib/`, `schema.sql`, `generate.sh` |

Proto definitions are created by the server agent. The client agent consumes them via `../eduxal/generate.sh` which runs `protoc` against this repo's `protos/` directory.

---

## User Creation Rules & Invitation Flow

### Two Types of User Creation

There are two distinct operations that result in a new `users` row:

#### 1. Invitation (Anyone → Normal Invited Users Only)

Any user — regardless of level or permissions — can **invite** a normal user. This happens implicitly when adding a member to a school. The user creation is a **side effect** of the member creation, not an independent operation.

**Server enforcement:**
- `status` MUST be `Invited` — reject anything else
- `level` MUST be `Normal` — reject anything else
- Only `phone` and `name` are accepted from the client — all other fields set to server defaults
- NOT gated by `Users.Create` permission — gated by the member table permission (`Teachers.Create`, `Staff.Create`, `Owners.Create`, etc.)

#### 2. Privileged User Creation (System/Super Users)

| Creator Level | Can Create Normal (Invited) | Can Create System (Invited) | Can Create Super (Invited) |
|---|---|---|---|
| Normal | ✅ Only as side effect of member creation | ❌ | ❌ |
| System (with `Users.Create`) | ✅ | ✅ | ❌ |
| System (without `Users.Create`) | ✅ Only as side effect of member creation | ❌ | ❌ |
| Super | ✅ | ✅ | ✅ |

**All created users start as `status = Invited`** regardless of who creates them.

### Server Validation for User Insert

```
// In PushChanges handler, when processing a users Insert:
1. if status != Invited → reject (code 3: validation_error)
2. match level:
   - Normal → allow (anyone can create normal invites)
   - System → require Users.Create permission (code 1 if missing)
   - Super  → require pusher.level == Super (code 1 if not super)
3. Server only accepts phone + name from client. Sets defaults for all other fields.
```

### Invitation-Aware Member Creation (Phone Conflict Resolution)

**The problem:** Two school admins (offline) both invite the same phone number. The first push creates the user. The second push hits a `UNIQUE(phone)` constraint conflict. But this is a resolvable conflict — both admins intended to add that phone number as a member of their school.

**Batch guarantee:** The client MUST always send the user invite + member creation in the **same `MutationBatch`**. The server uses this to detect the "invitation pattern."

**Server logic when processing a member table Insert** (`owners`, `teachers`, `staff`, `students`, `guardians`):

```
1. Check if this member mutation references a user that was also
   CREATED in the same batch (i.e., it's an invitation)

2. If yes → this is an invitation flow:
   a. Try to insert the user
   b. If phone conflict:
      - Look up existing user by phone number
      - If existing user status == Deleted → reject (code 4: not_found)
      - Rewrite the member's `user` field to the existing user's ID
      - Insert the member with the corrected user ID
      - For the user mutation: return code 2 (conflict)
      - For the member mutation: return code 0 (success) with corrected row data
      - Log to server_logs:
        * Delete on users table for the orphaned user ID (so all clients clean up)
        * Insert on the member table with the corrected user field

3. If no → normal member creation (user already exists), just validate and insert
```

**What streams to other clients via WatchChanges:**
- `SyncDelta { op: Delete, table: users, row_key: "orphaned_id" }` — all clients remove the orphan
- `SyncDelta { op: Insert, table: <member_table>, ... }` — with the corrected `user` field pointing to the real existing user

The pushing client receives:
- `PushAck` with code 2 on the user mutation (apply server version = delete orphan locally)
- `PushAck` with code 0 on the member mutation (success, but with corrected row data)

### Member Tables That Trigger Invitation Flow

All five member tables follow the same pattern:
- `owners` (school owner)
- `teachers` (teacher at school)
- `staff` (staff member at school)
- `students` (student — though students may be minors without phones, TBD)
- `guardians` (parent/guardian of a student)

