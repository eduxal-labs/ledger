# Eduxal Ledger — Session Context

This document brings a new AI session fully up to speed on the Eduxal Ledger project. Read it completely before making any changes or suggestions.

---

## What Is This Project

Eduxal is a school management platform for Kenyan schools. The backend is called **Ledger** — a Rust gRPC server (Tonic + Diesel + SQLite) serving a Flutter mobile client. It handles authentication, data sync, exam/question management, and AI-assisted marking.

The server is **fully functional and already running**. Two pilot schools are planned for near-term launch. The codebase is clean, hand-crafted Rust following strict architectural patterns described in `AGENT.md`. Read `AGENT.md` in full before touching any code.

---

## Repository Layout

```
eduxal-labs/
├── ledger/          ← This repo. The active Rust gRPC server.
│   ├── AGENT.md     ← MUST READ before any code changes
│   ├── TASKS.md     ← Current task list for auth improvement
│   ├── context.md   ← This file
│   ├── client.md    ← Client-side auth enforcement spec
│   └── src/
└── eduxal/          ← Flutter client (Dart). NOT in this repo root.
```

---

## Current Technical Stack

| Layer | Technology |
|---|---|
| Language | Rust, edition 2024, nightly |
| gRPC | Tonic |
| ORM | Diesel + SQLite |
| Tokens | PASETO v4 local (symmetric, compile-time key via `macros::key!`) |
| OTP delivery | WhatsApp Business API |
| File storage | Cloudflare R2 (presigned URLs) |
| AI marking | External AI API via async queue |
| Client | Flutter (Dart), local-first with SQLite |
| Server address | `0.0.0.0:50051` |

---

## What Has Already Been Built and Works

### Authentication Service (`/protos/services/authentication.proto`)
Passwordless phone-based OTP flow via WhatsApp. Fully working. Three token types: Access (2 days), Refresh (30 days), Setup (15 min). All PASETO v4 local.

**This service has already been SEPARATELY re-implemented as a serverless service:**
- Runtime: AWS Lambda (arm64)
- Database: DynamoDB
- API: HTTP REST via AWS API Gateway
- Base URL: `https://auth.eduxal.com`
- Endpoints: `POST /auth/login`, `POST /auth/verify`, `POST /auth/setup`, `GET /auth/refresh`, `GET /user`, `PATCH /user/rename`, `POST /user/change-phone`, `PATCH /user/confirm-change-phone`, `GET /sessions`, `DELETE /sessions/{id}`
- Status: **Complete and tested**

### Sync Service (`/protos/services/sync.proto`)
Real-time bidirectional sync using gRPC streaming. Clients push mutations (`PushActions`) and receive live deltas (`WatchChanges`). The server maintains a binary changelog (`changelog.bin` + `changelog.bin.deletes`) for incremental sync. Per-user filtering based on school membership and role permissions.

### Question Bank Service (`/protos/services/question_bank.proto`)
Global (system-scoped) question catalog. Questions have: plain text stem, marks, optional example answer, rubric criteria (criterion + marks), and images (S3-backed). Paper assembly: randomly selects questions per topic allocation, generates PDFs.

### AI Marking Service (`/protos/services/ai_marking.proto`)
Async marking pipeline. Students upload answer sheets (images). Server downloads, runs AI marking against rubric, writes scores to grades table.

---

## Three-Tier Permission Model

The permission system has three user levels:

### Super (level = 2)
- Bypasses all checks. Can do everything. No filtering.

### System (level = 1)
- Globally scoped but role-gated.
- System-scoped roles (where `scopes.school IS NULL`) determine what they can read/write globally.
- Can also have school memberships — when acting in a school context, school roles + system roles are merged.

### Normal (level = 0)
- School membership determines access. Must be an owner, teacher, staff, student, or guardian of a school to interact with it.
- School owners get a full bypass within their own school (no role check needed).
- Within a school: role-based permissions determine what operations are allowed.

### Permission Storage
Roles have a `permissions` column stored as a binary blob:
- 3 bytes per non-empty resource: `[resource_id: u8, actions_lo: u8, actions_hi: u8]` (little-endian u16)
- Sparse encoding — empty resources are omitted
- Max 19 resources × 3 bytes = 57 bytes

### Resources (19 total)
```
1=Users, 2=Schools, 3=Owners, 4=Teachers, 5=Staff, 6=Students,
7=Departments, 8=Classes, 9=Attendance, 10=Lessons, 11=Exams,
12=Grades, 13=Fees, 14=Payments, 15=Announcements, 16=Roles,
17=Plans, 18=AI, 19=Subjects
```

### Actions (bitmask u16)
```
Create=1, Read=2, Update=4, Delete=8, Purge=16,
Assign=32, Unassign=64, Mark=128, Approve=256
```

---

## Active Work: Authorization Improvement

### The Problem
In `src/services/sync.rs`, the `check_action_permission` function has a critical bug:

```rust
Level::Normal => {
    // BUG: all Normal users pass auth unconditionally
    Ok(())
}
```

This means any Normal user (teacher, staff, parent) can perform ANY operation on ANY school if they know the record IDs. This is a launch blocker.

### The Fix (described in `TASKS.md`)
Three tasks, fully specified in `TASKS.md`:

**Task 01** — Add `action_organisation(conn, action_id, user_id, payload) -> Result<Organisation>` to `src/db/database/tables/actions.rs`. This maps every action + its decoded payload to an `Organisation` context (`System`, `Account`, or `School(id)`). Uses prost helper structs for decoding and SQL lookups for records where school is not in the payload (UpdateExam, UpdateFee, UpdateInvoice, UpdatePayment, UpdateAnnouncement).

**Task 02** — Add `authorize_user(conn, user, organisation, permissions) -> Result<()>` to `src/db/database/authorize.rs`. This mirrors the existing `Authorize::authorize` impl but takes an already-loaded `&User` instead of a `Token`, avoiding a redundant DB fetch. The existing `Authorize` impl in `authorize.rs` is complete and correct — we just need to expose its logic for the push path.

**Task 03** — Replace `check_action_permission` in `src/services/sync.rs` with a three-step call: build `Permissions` from `action_permission()`, get `Organisation` from `action_organisation()`, call `authorize_user()`. Delete the old function.

**Zero proto changes. Zero frontend changes.**

### Key Infrastructure Already in Place
- `src/db/database/authorize.rs` — full `Authorize` impl with Super bypass, owner bypass, school-active check, school-scoped role loading, system role merging
- `src/db/database/tables/memberships.rs` — `Load<&User, Id>` loads all schools a user belongs to
- `src/db/database/authorize.rs` — `Load<(Id, &User), Role>` loads school-scoped roles
- `src/db/database/authorize.rs` — `Load<&User, Role>` loads system-scoped roles
- `src/types/role/permissions.rs` — `Permissions` with `Index<Resource>`, `+`/`-` operators
- `src/types/role/actions.rs` — `Actions::from(action)`, bitmask math

---

## Serverless Migration Plan

### Why Migrate
The current SQLite + single-process + gRPC streaming architecture cannot scale horizontally. The file-based changelog, thread-local connections, and streaming connections all require a single persistent process.

### What Has Already Been Done
Authentication is fully migrated to AWS Lambda + DynamoDB (see above).

### The Target Architecture
**Core insight:** The server stores no actual school data. It is purely a message relay + authorization gatekeeper. The client's local SQLite database is the source of truth for school data.

**Components:**

| Component | Technology | Role |
|---|---|---|
| Message broker | AWS IoT Core (MQTT) | Real-time fan-out, persistent sessions |
| Message ingest | AWS IoT Basic Ingest (`$aws/rules/{name}`) | Cheap publish path → Rules Engine |
| Message log | DynamoDB `message_log` table + S3 archive | Durable, replay-capable event store |
| Auth projections | DynamoDB `roles` + `assignments` tables | Rebuilt from message log replay |
| Connection tracking | DynamoDB `connections` table | Tracks per-session last received message |
| Authorization | AWS Lambda (Custom Authorizer) | Validates PASETO token on connect |
| Message processor | AWS Lambda (Rules Engine trigger) | Validates action, routes to school topic |
| Offline delivery | Connections table + DynamoDB/S3 replay | Delivers missed messages on reconnect |
| Cold start | Peer device sync OR message log replay | New device gets full state |

**MQTT Topic Structure:**
```
Subscribe (client receives):
  schools/{school_id}/changes     ← school-scoped mutations
  global/changes                  ← users, plans, subject catalog
  users/{user_id}/replay          ← missed messages on reconnect

Publish (client sends):
  $aws/rules/ProcessMutation      ← Basic Ingest → Rules Engine
```

**Message Schema (every published message):**
```json
{
  "id":         "01J3K...",         // ULID — sortable, unique
  "type":       "CreateStudent",    // action type string
  "school_id":  "abc123",           // null for global actions
  "user_id":    "507f1f...",
  "session_id": "69ed05...",
  "timestamp":  1718000000000,      // ms epoch
  "payload":    { ... }
}
```

**MQTT Persistent Sessions:**
- Client connects with `clientId = session_id`, `cleanSession = false`
- IoT Core queues undelivered QoS 1 messages for offline clients
- **Retention limit: ~1 hour** — after this, IoT Core drops queued messages
- The `connections` table compensates: on disconnect, record `last_message_id`; on reconnect, replay missed messages from DynamoDB/S3

**The Two Lambdas:**
1. **Custom Authorizer Lambda** — called once on MQTT connect. Validates PASETO token, reads user's schools/level/roles from DynamoDB (from auth service tables), returns IoT policy scoping allowed topics.
2. **Message Processor Lambda** — triggered by Basic Ingest rule on every published message. Saves message to DynamoDB `message_log` + S3, updates Roles/Assignments projections if relevant, re-publishes to `schools/{school_id}/changes` for fan-out.

**DynamoDB Tables:**
```
connections:    pk=session_id, attrs: user_id, last_message_id (ULID), schools (list)
message_log:    pk=school_id|"GLOBAL", sk=ULID, attrs: type, payload, user_id, timestamp
                TTL=30 days → DynamoDB Streams → Lambda → S3 archive
roles:          pk=school_id|"GLOBAL", sk=role_id, attrs: name, permissions
assignments:    pk=user_id, sk=school_id#role_id, GSI: pk=school_id
```

**Event Sourcing Pattern:**
The message log is the event store. Roles/Assignments DynamoDB tables are projections rebuilt by replaying messages. If a projection is corrupt or a new query pattern is needed, replay the log to rebuild.

**School-Sharded Changelog (key scalability insight):**
- DynamoDB `pk = "SCHOOL#{school_id}"` for school-scoped messages, `pk = "GLOBAL"` for system messages
- Client stores a map of cursors: `{ "GLOBAL": "lastULID", "school_A": "lastULID" }`
- Poll requests only query relevant partitions — a Normal user in one school makes 2 DynamoDB queries, not a global scan
- Cost scales with per-user data, not total platform write volume

**Migration Sequencing:**
1. Keep the current Rust server running (containerize on Railway/Fly.io for ~$5/month)
2. Build IoT Core backend in parallel (no deadline pressure)
3. Build web frontend (Svelte/Solid) targeting the IoT backend
4. Cut over when ready; deprecate the Rust server

---

## Web Frontend Plan

The planned web frontend is **Svelte or SolidJS** (NOT React). Tauri v2 wraps it for:
- **Desktop**: native binaries for Windows/macOS/Linux (school admin PCs)
- **Mobile**: iOS and Android via Tauri v2 mobile (replacing the Flutter app long-term)

The tech stack:
- **Svelte 5** (runes) or **SolidJS** for the UI framework
- **shadcn-svelte** or **shadcn-solid** for the component library
- **mqtt.js** for IoT Core sync over WebSocket
- **tauri-plugin-sql** (SQLite) for the local-first database in Tauri
- **TipTap** (Svelte/Solid extension) for the rich text question editor

---

## Question Bank Improvement — THE MAIN TOPIC FOR THIS SESSION

This is the primary focus. The current question bank has poor UX because questions are plain text strings with no formatting. Real exam questions need:
- **Reading passages** (comprehension questions have text students read before answering)
- **Rich text formatting** (bold, italic, lists, tables, math equations)
- **Marks placement** (show marks to the right, below, or inline with the question)
- **Structured rubric** that can also be rich text
- **PDF export** that faithfully reproduces the rich formatting
- **A human-authored editor** (not AI-programmatic generation)

### Current Database Schema

```sql
-- Global question catalog (not school-scoped)
CREATE TABLE questions (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    topic INTEGER NOT NULL REFERENCES topic_catalog(id),
    text TEXT NOT NULL,              -- plain string stem
    marks SMALLINT NOT NULL,
    example_answer TEXT,             -- plain string
    created BIGINT NOT NULL DEFAULT (unixepoch('now')),
    updated BIGINT NOT NULL DEFAULT (unixepoch('now'))
);

CREATE TABLE rubric_criteria (
    question INTEGER NOT NULL REFERENCES questions(id) ON DELETE CASCADE,
    position SMALLINT NOT NULL,
    criterion TEXT NOT NULL,         -- plain string
    marks SMALLINT NOT NULL,
    PRIMARY KEY (question, position)
);

CREATE TABLE question_images (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    question INTEGER NOT NULL REFERENCES questions(id) ON DELETE CASCADE,
    position SMALLINT NOT NULL,
    context SMALLINT NOT NULL,       -- 0=passage, 1=stem, 2=option
    key TEXT NOT NULL,               -- R2 object key
    caption TEXT
);
```

### Current Proto Message
```protobuf
message Question {
    int32 id = 1;
    int32 topic_id = 2;
    string text = 3;                 // plain text
    int32 marks = 4;
    optional string example_answer = 5;
    repeated RubricCriterion rubric = 6;
    repeated QuestionImage images = 7;
    int64 created = 8;
    int64 updated = 9;
}
```

### Required Schema Changes

**Migration — add to `questions` table:**
```sql
ALTER TABLE questions ADD COLUMN passage TEXT;
-- Rich text JSON for reading material shown before the question.
-- Null for questions with no passage.

ALTER TABLE questions ADD COLUMN body TEXT;
-- Rich text JSON for the question stem.
-- When present, replaces `text` for rendering.
-- `text` remains for backward compat and plain-text fallback.

ALTER TABLE questions ADD COLUMN marks_label TEXT NOT NULL DEFAULT 'right';
-- Where to display the marks value: 'right' | 'below' | 'inline' | 'none'

ALTER TABLE questions ADD COLUMN question_type SMALLINT NOT NULL DEFAULT 0;
-- 0 = Structured (open-ended with rubric)
-- 1 = MCQ (multiple choice — requires question_options table)
-- 2 = Essay (long-form, single rubric criterion)
-- 3 = Reading (passage is mandatory)
```

**Migration — add to `rubric_criteria` table:**
```sql
ALTER TABLE rubric_criteria ADD COLUMN body TEXT;
-- Rich text JSON for the criterion description.
-- When present, replaces `criterion` for rendering.
```

**New table for MCQ options (phase 2, not immediate):**
```sql
CREATE TABLE question_options (
    question INTEGER NOT NULL REFERENCES questions(id) ON DELETE CASCADE,
    key TEXT NOT NULL,              -- 'A', 'B', 'C', 'D'
    body TEXT NOT NULL,             -- rich text JSON for option text
    correct INTEGER NOT NULL DEFAULT 0,  -- 0=false, 1=true
    position SMALLINT NOT NULL,
    PRIMARY KEY (question, key)
);
```

### The Rich Text Format: TipTap/ProseMirror JSON

Use **TipTap's JSON document format** as the canonical storage format. This is the right choice because:
- TipTap has official Svelte and solid-js integrations
- The same JSON renders correctly in the web editor and the PDF generator
- Flutter can render it with a custom widget tree (simple block traversal)
- It is human-readable, versionable, and easy for an AI agent to generate

**Document structure:**
```json
{
  "type": "doc",
  "content": [
    {
      "type": "heading",
      "attrs": { "level": 2 },
      "content": [{ "type": "text", "text": "Read the following passage." }]
    },
    {
      "type": "paragraph",
      "content": [
        { "type": "text", "text": "The water cycle, also known as the " },
        { "type": "text", "text": "hydrological cycle", "marks": [{ "type": "bold" }] },
        { "type": "text", "text": ", describes the continuous movement..." }
      ]
    },
    {
      "type": "bulletList",
      "content": [
        {
          "type": "listItem",
          "content": [{ "type": "paragraph", "content": [{ "type": "text", "text": "Evaporation" }] }]
        }
      ]
    }
  ]
}
```

**Supported node types for exam questions:**
- `paragraph` — body text
- `heading` (level 1–3) — section titles
- `bold`, `italic`, `underline` marks — inline formatting
- `bulletList`, `orderedList` — lists
- `hardBreak` — line break
- `image` — embedded image (references question_images by position)
- `mathBlock` (via TipTap Math extension) — LaTeX equations
- `table` — data tables

**What the `passage` field stores:** A full TipTap doc JSON string for the reading material shown above the question. Null if the question has no passage.

**What the `body` field stores:** A full TipTap doc JSON string for the question stem. When present, the UI ignores `text` and renders `body` instead.

### Updated Proto Message

```protobuf
message Question {
    int32 id = 1;
    int32 topic_id = 2;
    string text = 3;                    // legacy plain text — keep for backward compat
    int32 marks = 4;
    optional string example_answer = 5;
    repeated RubricCriterion rubric = 6;
    repeated QuestionImage images = 7;
    int64 created = 8;
    int64 updated = 9;
    optional string passage = 10;       // NEW: TipTap JSON for reading passage
    optional string body = 11;          // NEW: TipTap JSON for question stem
    optional string marks_label = 12;   // NEW: 'right' | 'below' | 'inline' | 'none'
    optional int32 question_type = 13;  // NEW: 0=structured, 1=mcq, 2=essay, 3=reading
}

message RubricCriterion {
    int32 position = 1;
    string criterion = 2;               // legacy plain text
    int32 marks = 3;
    optional string body = 4;           // NEW: TipTap JSON for rich criterion
}
```

### Updated `CreateQuestionRequest` and `BulkImportQuestion`

```protobuf
message CreateQuestionRequest {
    int32 topic_id = 1;
    string text = 2;                    // still accepted (plain text fallback)
    int32 marks = 3;
    optional string example_answer = 4;
    repeated RubricCriterionInput rubric = 5;
    optional string passage = 6;        // NEW
    optional string body = 7;           // NEW
    optional string marks_label = 8;    // NEW
    optional int32 question_type = 9;   // NEW
}
```

For `BulkImportQuestion` (the JSON import struct in `question_bank.rs`), add the same optional fields to the serde struct.

### Rust Service Changes Required

**`src/db/database/tables/rows.rs`** — Add `passage`, `body`, `marks_label`, `question_type` to `QuestionRow`. Add `body` to `RubricCriterionRow`.

**`src/db/database/tables/question_bank.rs`** — Update all INSERT/UPDATE SQL to include the new columns. Update SELECT queries to return them.

**`src/services/question_bank.rs`**:
- `build_question_proto` — populate new proto fields from row data
- `create_question` — persist new fields
- `update_question` — update new fields
- `bulk_import_questions` — parse new JSON fields in `BulkImportQuestion`
- `finalize_paper` — extend PDF generation to render TipTap JSON (see below)

**`src/db/schema/schema.rs`** — After running migration, regenerate with `diesel print-schema > src/db/schema/schema.rs`

### PDF Generation with Rich Text

The current `finalize_paper` generates PDFs. It needs to render TipTap JSON blocks to HTML and then pass to the PDF library.

**Recommended approach:**
1. In `finalize_paper`, build an HTML string from each question's block JSON
2. Use a simple block-to-HTML converter (write in Rust — each node type maps to an HTML tag)
3. Pass the HTML to the existing PDF generation library

**Block → HTML mapping:**
```
paragraph      → <p>...</p>
heading(1)     → <h1>...</h1>
heading(2)     → <h2>...</h2>
heading(3)     → <h3>...</h3>
bold mark      → <strong>...</strong>
italic mark    → <em>...</em>
underline mark → <u>...</u>
bulletList     → <ul>...</ul>
orderedList    → <ol>...</ol>
listItem       → <li>...</li>
hardBreak      → <br/>
mathBlock      → render via MathJax/KaTeX (if PDF library supports it)
image          → <img src="[signed R2 URL]" />
```

**Marks placement in PDF:**
- `marks_label = 'right'` → marks shown in a right-aligned column `[4 marks]`
- `marks_label = 'below'` → marks shown on the line below the question
- `marks_label = 'inline'` → marks appended at end of question text `(4 marks)`
- `marks_label = 'none'` → marks not shown on student copy

### Migration of Existing Questions

The user has all existing questions stored in JSON files. The plan:
1. An AI agent reads each question's plain `text` field
2. It generates TipTap JSON for `body` (preserving formatting intent)
3. It generates TipTap JSON for `passage` if the question has reading material
4. It populates `marks_label` and `question_type` based on question structure
5. Output: updated JSON files in the new `BulkImportRequest` format with `body`, `passage` fields

The server's `bulk_import_questions` endpoint accepts this JSON and stores it. The `text` field is still populated as a plain-text fallback.

### Editor UI (Web — Svelte/Solid)

When the web frontend is built, the question editor needs:
- **TipTap editor** for the `body` field (question stem)
- **TipTap editor** for the `passage` field (collapsible/optional)
- **TipTap editors** for each rubric criterion's `body` field
- **Marks input** (number) + **marks_label selector** (dropdown: right/below/inline/none)
- **Question type selector**
- **Image upload** (connects to `RequestImageUploadUrls` endpoint)
- **Preview mode** — renders exactly as it would in the PDF

The editor calls `CreateQuestion` or `UpdateQuestion` gRPC endpoints (or REST equivalents in the new architecture) with the TipTap JSON stringified.

### Flutter Client Rendering

Until the web frontend exists, the Flutter client renders questions. Implement a `QuestionRenderer` widget:
- If `body != null`: walk the TipTap JSON tree and render as Flutter widgets (Text with TextSpan for inline marks, Column for blocks)
- If `body == null`: render `text` as plain `Text` widget
- If `passage != null`: render passage above the question in a styled container
- Show marks according to `marks_label`

---

## What To Work On Next (Suggested Agenda for New Session)

After reading `AGENT.md` and this file:

1. **If auth tasks are incomplete:** Complete Tasks 01–04 in `TASKS.md` first. This is a launch blocker.

2. **Question bank richness:** Create the Diesel migration adding `passage`, `body`, `marks_label`, `question_type` columns. Update `rows.rs`, `question_bank.rs` (DB layer), `question_bank.rs` (service layer), and `question_bank.proto`. This is the main UX improvement before pilot launch.

3. **PDF generation enhancement:** Extend `finalize_paper` to render TipTap JSON blocks to HTML for PDF output.

4. **Bulk import format update:** Update `BulkImportQuestion` struct to accept new rich text fields so the AI-processed question JSON can be imported.

---

## Important Constraints

- **No frontend code in this repo.** The Flutter client is at `../eduxal/`. The web frontend does not exist yet.
- **No hand-editing `src/db/schema/schema.rs`.** Always run `diesel print-schema` after migrations.
- **All env vars are compile-time.** After `.env` changes, `cargo clean` is required.
- **Single `Error` enum.** Never add new error types. Add variants to `src/types/error.rs` and add a `From<Error> for Status` mapping.
- **Proto files for services must be registered in `build.rs`.** Type-only protos are discovered via imports.
- **Svelte or SolidJS for web frontend** — NOT React. Component library: shadcn-svelte or shadcn-solid.

---

## Files To Read Before Making Changes

Always read in this order:
1. `AGENT.md` — architectural rules, patterns, conventions
2. `TASKS.md` — current task list and status
3. The specific source files affected by the task (listed in each task spec)