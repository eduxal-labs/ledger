# TASKS.md

## Overview

**Feature:** Support the client-side multi-file bulk import + image upload pipeline.

**Context:** The Flutter client (at ../eduxal) is building a feature to bulk-import ~413 JSON question files with ~797 SVG image references. The backend already supports BulkImportQuestions (returns question_ids) and RequestImageUploadUrls. However:

1. The client Dart proto stubs are **out of date** and wire-incompatible with the current backend proto.
2. The S3 key format in request_image_upload_urls hardcodes .webp but import files reference .svg images.
3. The ImageUploadSpec proto needs a filename field so the backend can use the correct file extension.

These tasks fix the backend and regenerate client stubs so the end-to-end flow works.

---

## Task B1: Add filename field to ImageUploadSpec and fix S3 key extension

**Files to modify:**
- protos/services/question_bank.proto
- src/services/question_bank.rs

**Depends on:** None

**Specification:**

### Step 1: Update proto

In protos/services/question_bank.proto, add string filename = 5 to ImageUploadSpec:

    message ImageUploadSpec {
      int32 question_id = 1;
      int32 position = 2;
      int32 context = 3;
      optional string caption = 4;
      string filename = 5;
    }

### Step 2: Update S3 key generation

In src/services/question_bank.rs, find the request_image_upload_urls handler (around line 430). Change the key format from:

    let key = format!("questions/{}/{}.webp", spec.question_id, spec.position);

To:

    let ext = spec.filename.rsplit('.').next().unwrap_or("webp");
    let key = format!("questions/{}/{}.{}", spec.question_id, spec.position, ext);

This extracts the file extension from the provided filename (e.g. "diagram.svg" -> "svg") and uses it in the S3 key. Falls back to "webp" if no extension is found.

### Step 3: Build and verify

Run cargo build to verify the proto compiles and the service code works with the new field.

**Update after completion:**
- [x] Mark this task [x]

---

## Task B2: Regenerate Dart proto stubs for the client

**Files to create/modify:**
- ../eduxal/lib/proto/services/question_bank.pb.dart (overwrite)
- ../eduxal/lib/proto/services/question_bank.pbgrpc.dart (overwrite)
- ../eduxal/lib/proto/services/question_bank.pbenum.dart (overwrite)
- ../eduxal/lib/proto/services/question_bank.pbjson.dart (overwrite)

**Depends on:** Task B1

**Specification:**

Regenerate the Dart protobuf stubs from the updated protos/services/question_bank.proto and copy them to the client project.

### Step 1: Generate Dart stubs

Run protoc with the Dart plugin to generate stubs:

    protoc --dart_out=grpc:./dart_out --proto_path=./protos services/question_bank.proto

(Adjust the command based on your local protoc + Dart plugin setup.)

### Step 2: Copy generated files to client

Copy the generated question_bank.pb.dart, question_bank.pbgrpc.dart, question_bank.pbenum.dart, and question_bank.pbjson.dart to ../eduxal/lib/proto/services/.

### Step 3: Verify the new stubs contain

After regeneration, verify these fields exist in the Dart stubs:

1. BulkImportResponse has a questionIds field (repeated int32, field 3)
2. ImageUploadUrlsRequest has an images field (repeated ImageUploadSpec, field 1)
3. ImageUploadSpec has fields: questionId (1), position (2), context (3), caption (4), filename (5)
4. ImageUploadUrl has fields: questionId (1), position (2), key (3), putUrl (4)

**Update after completion:**
- [ ] Mark this task [x]

---

## Dependency Graph

    Task B1 (proto + key fix) -- no deps
      +-- Task B2 (regenerate Dart stubs) -- depends on B1

B1 must complete first. Then B2.
