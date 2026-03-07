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