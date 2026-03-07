# Ledger

A high-performance gRPC authentication microservice built in Rust, designed as the identity and access management backbone for the EduxaL education platform. Ledger handles user authentication via phone-based OTP (delivered through WhatsApp), issues PASETO v4 encrypted tokens, and manages user lifecycle operations — all backed by SQLite with Diesel ORM.

## Table of Contents

- [Architecture](#architecture)
- [Features](#features)
- [Prerequisites](#prerequisites)
- [Environment Variables](#environment-variables)
- [Getting Started](#getting-started)
- [Project Structure](#project-structure)
- [gRPC API](#grpc-api)
- [Authentication Flow](#authentication-flow)
- [Database](#database)
- [Security](#security)
- [Configuration Subsystems](#configuration-subsystems)

## Architecture

```text
┌─────────────┐     gRPC (50051)     ┌──────────────────────────────────┐
│   Client    │ ──────────────────── │           Ledger Server          │
└─────────────┘                      │                                  │
                                     │  ┌────────────┐  ┌───────────┐  │
                                     │  │ Proto Layer │──│ Services  │  │
                                     │  └────────────┘  └─────┬─────┘  │
                                     │                        │        │
                                     │         ┌──────────────┼──────┐ │
                                     │         │              │      │ │
                                     │  ┌──────▼──┐  ┌───────▼──┐   │ │
                                     │  │ Config  │  │ Database │   │ │
                                     │  │         │  │ (SQLite) │   │ │
                                     │  ├─────────┤  └──────────┘   │ │
                                     │  │Verifyer │                 │ │
                                     │  │WhatsApp │                 │ │
                                     │  │R2 Signer│                 │ │
                                     │  └─────────┘                 │ │
                                     │                              │ │
                                     └──────────────────────────────┘ │
                                                                      │
                                     ┌──────────────┐  ┌─────────────┐
                                     │ Meta Graph   │  │Cloudflare R2│
                                     │ WhatsApp API │  │  (Storage)  │
                                     └──────────────┘  └─────────────┘
```

## Features

- **Phone-based OTP Authentication** — Login via 6-digit codes delivered through WhatsApp Business API
- **PASETO v4 Tokens** — Encrypted (local) tokens for access (3 days), refresh (30 days), and setup (1 hour)
- **Compile-time Key Derivation** — PASETO symmetric keys derived from passwords at build time using Argon2id via a proc-macro
- **In-memory Verification Engine** — Lock-free, channel-driven OTP store with rate limiting (90s cooldown) and automatic TTL-based expiry
- **SQLite with Diesel ORM** — Thread-local connections with WAL mode, foreign keys, and memory-mapped I/O for maximum throughput
- **Cloudflare R2 Presigned URLs** — AWS SigV4-compatible URL signing for profile image uploads/downloads (no runtime SDK dependency)
- **Comprehensive Phone Parsing** — Supports Kenyan phone formats: `+254...`, `254...`, `0...`, and raw 9-digit numbers
- **Clean Error Mapping** — Domain errors map directly to appropriate gRPC status codes

## Prerequisites

- **Rust** — Edition 2024 (nightly required for `let chains` and edition 2024)
- **Diesel CLI** — For running database migrations
- **Protobuf Compiler (`protoc`)** — For compiling `.proto` files at build time
- **SQLite 3.35+** — Bundled via `libsqlite3-sys` (no system install required)

```sh
# Install Diesel CLI (SQLite only)
cargo install diesel_cli --no-default-features --features sqlite

# Ensure protoc is available
# Ubuntu/Debian
sudo apt install -y protobuf-compiler

# macOS
brew install protobuf
```

## Environment Variables

Create a `.env` file in the project root. All variables are read at **compile time**.

| Variable | Description | Required |
|---|---|---|
| `PASETO_PASSWORD` | Password used to derive the 32-byte PASETO v4 symmetric key via Argon2id | Yes |
| `WHATSAPP_TOKEN` | Meta Graph API bearer token for WhatsApp Business messaging | Yes |
| `R2_ACCOUNT_ID` | Cloudflare account ID for R2 storage | Yes |
| `R2_BUCKET` | R2 bucket name | Yes |
| `R2_ACCESS_KEY_ID` | R2 API token access key | Yes |
| `R2_SECRET_ACCESS_KEY` | R2 API token secret key | Yes |
| `DATABASE_URL` | SQLite database path (defaults to `database.db`) | No |

Example `.env`:

```
PASETO_PASSWORD=your-strong-secret-password
WHATSAPP_TOKEN=EAAxxxxxxxxxxxxxxxxxxxxxxxx
R2_ACCOUNT_ID=abcdef1234567890
R2_BUCKET=eduxal-assets
R2_ACCESS_KEY_ID=your-r2-access-key
R2_SECRET_ACCESS_KEY=your-r2-secret-key
DATABASE_URL=database.db
```

> **Note:** Environment variables are injected at compile time via `build.rs` and the `env!()` macro. Changes require a rebuild.

## Getting Started

```sh
# Clone the repository
git clone <repo-url>
cd ledger

# Create your .env file (see above)
cp .env.example .env

# Run database migrations
diesel migration run

# Build and run
cargo run
```

The server starts on `127.0.0.1:50051`.

## Project Structure

```
ledger/
├── build.rs                    # Protobuf compilation & env loading
├── Cargo.toml                  # Workspace dependencies
├── diesel.toml                 # Diesel CLI configuration
├── macros/                     # Proc-macro crate
│   └── src/lib.rs              #   key! (Argon2id key derivation), Count derive
├── migrations/                 # Diesel SQL migrations
│   └── 2026-02-21-.../
│       ├── up.sql
│       └── down.sql
├── protos/                     # Protobuf definitions
│   ├── services/
│   │   └── authentication.proto
│   └── types/
│       ├── user.proto
│       └── verification.proto
└── src/
    ├── main.rs                 # Async entrypoint
    ├── server.rs               # Tonic server setup
    ├── config/                 # Configuration & external integrations
    │   ├── mod.rs              #   Configuration struct (composes traits)
    │   ├── messenger.rs        #   Messenger trait (async code delivery)
    │   ├── whatsapp.rs         #   WhatsApp Business API implementation
    │   ├── verifyer.rs         #   Verifyer trait (OTP lifecycle)
    │   ├── verifications.rs    #   In-memory OTP store & processor
    │   └── storage/
    │       ├── mod.rs
    │       └── sign.rs         #   R2 AWS SigV4 presigned URL generation
    ├── db/                     # Database layer
    │   ├── mod.rs
    │   ├── schema/
    │   │   ├── mod.rs
    │   │   └── schema.rs       #   Diesel-generated table definitions
    │   └── database/
    │       ├── mod.rs           #   Connection pool, migrations, pragmas
    │       ├── traits.rs        #   Generic Database, Create, Find, Update traits
    │       └── tables/
    │           └── users.rs     #   User CRUD implementations
    ├── proto/                   # Generated protobuf + service wrappers
    │   ├── mod.rs
    │   ├── types/mod.rs         #   Proto type includes (user, verification)
    │   └── services/
    │       ├── mod.rs
    │       └── authentication.rs #  Authentication trait & gRPC adapter
    ├── services/                # Business logic
    │   ├── mod.rs
    │   └── authentication.rs    #   Authenticator implementation
    └── types/                   # Domain types
        ├── mod.rs
        ├── id.rs                #   BSON ObjectId wrapper
        ├── phone.rs             #   Kenyan phone number parser
        ├── token.rs             #   PASETO v4 token (access/refresh/setup)
        ├── verification.rs      #   OTP verification & code type
        ├── command.rs           #   Channel command/response pattern
        ├── error.rs             #   Error enum with gRPC status mapping
        └── user/
            ├── mod.rs
            ├── user.rs          #   User struct (Diesel model)
            ├── update.rs        #   Update changeset
            ├── level.rs         #   Level enum (Normal/System/Super)
            └── status.rs        #   Status enum (Invited/Active/Suspended/Deleted)
```

## gRPC API

The service is defined in `protos/services/authentication.proto`:

| RPC | Request | Response | Description |
|---|---|---|---|
| `login` | `Login { phone }` | `Verification` | Sends OTP to the phone number via WhatsApp |
| `verify` | `Verify { id, code }` | `Verified` | Validates OTP; returns tokens or a setup token for new users |
| `setup` | `Setup { token, name }` | `Authenticated` | Completes first-time registration using the setup token |
| `refresh` | `Refresh { refresh_token }` | `Authenticated` | Reissues access + refresh tokens |
| `changePhone` | `ChangePhone { token, phone }` | `Verification` | Initiates phone number change with OTP |
| `confirmChangePhone` | `ConfirmChangePhone { token, id, code }` | `Authenticated` | Confirms phone change after OTP verification |

### Response Types

**`Verified`** — A oneof:
- `Registered { token }` — Setup token for new users (valid 1 hour)
- `Authenticated { access_token, refresh_token, user, profile }` — Full auth payload for existing users

**`Authenticated`**:
- `access_token` — PASETO v4 local token (3-day TTL)
- `refresh_token` — PASETO v4 local token (30-day TTL)
- `user` — Full user object
- `profile` — Presigned PUT URL for profile image upload (1-hour TTL)

## Authentication Flow

### New User Registration

```text
Client                          Server                     WhatsApp
  │                               │                           │
  │── login(phone) ──────────────▶│                           │
  │                               │── send OTP ──────────────▶│
  │                               │                           │── deliver ──▶ User
  │◀── Verification { id } ──────│                           │
  │                               │                           │
  │── verify(id, code) ─────────▶│                           │
  │◀── Registered { token } ─────│  (user not found)         │
  │                               │                           │
  │── setup(token, name) ───────▶│                           │
  │◀── Authenticated { ... } ────│  (user created)           │
```

### Returning User Login

```text
Client                          Server                     WhatsApp
  │                               │                           │
  │── login(phone) ──────────────▶│                           │
  │                               │── send OTP ──────────────▶│
  │◀── Verification { id } ──────│                           │
  │                               │                           │
  │── verify(id, code) ─────────▶│                           │
  │◀── Authenticated { ... } ────│  (user found & activated) │
```

### Token Refresh

```text
Client                          Server
  │                               │
  │── refresh(refresh_token) ───▶│
  │◀── Authenticated { ... } ────│
```

## Database

Ledger uses **SQLite** via Diesel ORM with the following optimizations applied at connection time:

| Pragma | Value | Purpose |
|---|---|---|
| `foreign_keys` | `ON` | Enforce referential integrity |
| `journal_mode` | `WAL` | Write-Ahead Logging for concurrent reads |
| `synchronous` | `NORMAL` | Balance durability and performance |
| `temp_store` | `MEMORY` | Temporary tables in RAM |
| `cache_size` | `-65536` | 64 MB page cache |
| `mmap_size` | `268435456` | 256 MB memory-mapped I/O |

The schema includes 30+ tables supporting the broader EduxaL platform (users, schools, students, enrollments, fees, exams, grades, attendance, timetables, and more). Currently, the authentication service operates on the `users` table.

Connections use a **thread-local `RefCell`** pattern — each thread gets its own dedicated SQLite connection, avoiding mutex contention while staying compatible with Tokio's multi-threaded runtime.

## Security

### Token Encryption

- Tokens use **PASETO v4 (local/symmetric)** encryption — not JWT signatures
- The 32-byte symmetric key is derived at **compile time** using **Argon2id** (19 MB memory, 8 iterations, 4 lanes) from the `PASETO_PASSWORD` environment variable
- Token payloads contain: `user`, `phone`, `purpose`, `created`, `expiry`
- Three token purposes with distinct TTLs: Access (3 days), Refresh (30 days), Setup (1 hour)

### OTP Verification

- 6-digit random numeric codes
- 15-minute TTL per verification
- 90-second rate limit between requests for the same phone number
- Bounded channel (capacity 200) with backpressure — returns `RESOURCE_EXHAUSTED` when full
- Codes are stored in-memory only (never persisted to disk)
- Verification entries are automatically cleaned via BTreeMap-based expiry sweeps

### Presigned URLs

- Profile image uploads/downloads use **AWS SigV4-compatible presigned URLs** for Cloudflare R2
- No credentials are ever sent to the client — only time-limited, cryptographically signed URLs
- GET URLs default to 3-day TTL; PUT URLs include `Content-Type: image/*` restriction

## Configuration Subsystems

### Messenger (WhatsApp)

Sends OTP codes via the Meta Graph API using WhatsApp Business template messages (`auth_code` template). The implementation is behind a `Messenger` trait, making it straightforward to swap in SMS, email, or other delivery mechanisms.

### Verifyer (In-Memory OTP Store)

A dedicated OS thread runs a command processor that receives `Command` variants over a bounded crossbeam channel:

- **Request** — Generate and store a new verification
- **Verify** — Validate a code and consume the verification
- **Delete** — Remove a verification (used for cleanup on send failure)

This design keeps the OTP state completely off the async runtime, avoiding any async mutex contention.

### Storage (R2 Signing)

Pure-computation AWS SigV4 URL signing (~1-5μs per URL) with no network calls or SDK dependencies. Supports generating presigned GET and PUT URLs for user profile images and school logos.

## License

This project is proprietary software developed by EduxaL Labs.