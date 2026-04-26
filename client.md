# Client-Side Authorization Enforcement

## Why This Matters

The server enforces authorization on every pushed action. If a client queues an action
the user is not permitted to perform, the server will reject it with code 1
(Permission denied) when the client comes online. From the user's perspective:

1. They performed an operation (e.g., deleted an exam)
2. The UI accepted it and updated locally
3. They go offline and come back
4. The sync fails silently or shows a confusing error
5. Their local state diverges from the server

Client-side pre-authorization solves this by evaluating the permission check
**before** the action is queued locally. If the check fails, show an immediate,
clear error message: "You do not have permission to delete exams." The action is
never written to the sync queue.

This document is the complete specification for implementing that check in both
the current Flutter client and the future Svelte/Solid web client.

---

## Data Available to the Client

The client syncs and stores these tables locally. All the data needed for
permission evaluation is already present in the local database — no server
round-trip is required.

| Table | What it provides |
|---|---|
| `users` | Current user's `id`, `level` (0=Normal, 1=System, 2=Super), `status` |
| `owners` | `(school, user)` — whether a user is an owner of a school |
| `teachers` | `(school, user)` — whether a user is a teacher at a school |
| `staff` | `(school, user)` — whether a user is staff at a school |
| `students` | `(school, adm, user)` — student membership (user may be null) |
| `guardians` | `(school, user, student_adm)` — guardian membership |
| `schools` | `id`, `status` — school must be active (status = 1) |
| `roles` | `id`, `name`, `permissions` (binary blob) — role definitions |
| `scopes` | `(role, user, school)` — role assignments; school=NULL for system scope |

---

## The Permission Binary Format

The `roles.permissions` column is a binary blob. It encodes a sparse array of
(resource, actions) pairs:

```
3 bytes per non-empty resource:
  byte 0: resource_id (u8, 1–19)
  byte 1: actions_lo  (u8, low byte of u16 bitmask)
  byte 2: actions_hi  (u8, high byte of u16 bitmask)

Actions bitmask is little-endian u16:
  actions = actions_lo | (actions_hi << 8)

Empty resources are omitted entirely.
Max size: 19 resources × 3 bytes = 57 bytes.
```

### Action Bitmask Values

```
Create   = 1    (0x0001)
Read     = 2    (0x0002)
Update   = 4    (0x0004)
Delete   = 8    (0x0008)
Purge    = 16   (0x0010)
Assign   = 32   (0x0020)
Unassign = 64   (0x0040)
Mark     = 128  (0x0080)
Approve  = 256  (0x0100)
```

### Resource ID Values

```
Users         = 1
Schools       = 2
Owners        = 3
Teachers      = 4
Staff         = 5
Students      = 6
Departments   = 7
Classes       = 8
Attendance    = 9
Lessons       = 10
Exams         = 11
Grades        = 12
Fees          = 13
Payments      = 14
Announcements = 15
Roles         = 16
Plans         = 17
AI            = 18
Subjects      = 19
```

---

## Action → (Resource, Action) Mapping

Every sync action maps to exactly one (Resource, required Action). This is the
same table the server uses in `action_permission()`. The client must replicate it.

| Action constant | Resource | Required Action |
|---|---|---|
| CREATE_SCHOOL | Schools | Create |
| UPDATE_SCHOOL | Schools | Update |
| DELETE_SCHOOL | Schools | Delete |
| CREATE_TEACHER | Teachers | Create |
| UPDATE_TEACHER | Teachers | Update |
| DELETE_TEACHER | Teachers | Delete |
| CREATE_STAFF | Staff | Create |
| UPDATE_STAFF | Staff | Update |
| DELETE_STAFF | Staff | Delete |
| CREATE_OWNER | Owners | Create |
| DELETE_OWNER | Owners | Delete |
| CREATE_STUDENT | Students | Create |
| UPDATE_STUDENT | Students | Update |
| DELETE_STUDENT | Students | Delete |
| ENROLL_STUDENT | Students | Assign |
| UNENROLL_STUDENT | Students | Unassign |
| CREATE_GUARDIAN | Students | Create |
| UPDATE_GUARDIAN | Students | Update |
| DELETE_GUARDIAN | Students | Delete |
| CREATE_DEPARTMENT | Departments | Create |
| UPDATE_DEPARTMENT | Departments | Update |
| DELETE_DEPARTMENT | Departments | Delete |
| CREATE_TERM | Schools | Create |
| UPDATE_TERM | Schools | Update |
| DELETE_TERM | Schools | Delete |
| ASSIGN_CLASS_TEACHER | Classes | Assign |
| UNASSIGN_CLASS_TEACHER | Classes | Unassign |
| ASSIGN_SUBJECT | Classes | Assign |
| UNASSIGN_SUBJECT | Classes | Unassign |
| CREATE_TIMETABLE_ENTRY | Classes | Create |
| UPDATE_TIMETABLE_ENTRY | Classes | Update |
| DELETE_TIMETABLE_ENTRY | Classes | Delete |
| MARK_ATTENDANCE | Attendance | Mark |
| DELETE_ATTENDANCE | Attendance | Delete |
| CREATE_LESSON | Lessons | Create |
| DELETE_LESSON | Lessons | Delete |
| CREATE_EXAM | Exams | Create |
| UPDATE_EXAM | Exams | Update |
| DELETE_EXAM | Exams | Delete |
| CREATE_PAPER | Exams | Create |
| UPDATE_PAPER | Exams | Update |
| DELETE_PAPER | Exams | Delete |
| MARK_GRADES | Grades | Mark |
| UPDATE_GRADE | Grades | Update |
| DELETE_GRADE | Grades | Delete |
| UPDATE_MASTERY | Grades | Mark |
| CREATE_FEE | Fees | Create |
| UPDATE_FEE | Fees | Update |
| DELETE_FEE | Fees | Delete |
| CREATE_INVOICE | Fees | Create |
| UPDATE_INVOICE | Fees | Update |
| DELETE_INVOICE | Fees | Delete |
| CREATE_PAYMENT | Payments | Create |
| UPDATE_PAYMENT | Payments | Update |
| DELETE_PAYMENT | Payments | Delete |
| APPROVE_PAYMENT | Payments | Approve |
| CREATE_ANNOUNCEMENT | Announcements | Create |
| UPDATE_ANNOUNCEMENT | Announcements | Update |
| DELETE_ANNOUNCEMENT | Announcements | Delete |
| CREATE_ROLE | Roles | Create |
| UPDATE_ROLE | Roles | Update |
| DELETE_ROLE | Roles | Delete |
| ASSIGN_ROLE | Roles | Assign |
| UNASSIGN_ROLE | Roles | Unassign |
| UPDATE_USER | Users | Update |
| DELETE_USER | Users | Delete |
| UPDATE_SETTINGS | Schools | Update |
| CREATE_PLAN | Plans | Create |
| UPDATE_PLAN | Plans | Update |
| DELETE_PLAN | Plans | Delete |
| UPDATE_AI_USAGE | AI | Update |
| CREATE_SUBSCRIPTION | Plans | Create |
| UPDATE_SUBSCRIPTION | Plans | Update |
| DELETE_SUBSCRIPTION | Plans | Delete |
| CREATE_DISCOUNT | Plans | Create |
| UPDATE_DISCOUNT | Plans | Update |
| DELETE_DISCOUNT | Plans | Delete |
| CREATE_SUBJECT | Subjects | Create |
| UPDATE_SUBJECT | Subjects | Update |
| DELETE_SUBJECT | Subjects | Delete |
| CREATE_TOPIC | Subjects | Create |
| UPDATE_TOPIC | Subjects | Update |
| DELETE_TOPIC | Subjects | Delete |
| CREATE_STREAM | Schools | Create |
| UPDATE_STREAM | Schools | Update |
| DELETE_STREAM | Schools | Delete |
| CREATE_MPESA | Schools | Create |
| UPDATE_MPESA | Schools | Update |
| DELETE_MPESA | Schools | Delete |
| UPLOAD_SCHEME | Exams | Update |
| DELETE_SCHEME | Exams | Delete |
| UPLOAD_ANSWER_SHEET | Grades | Mark |
| DELETE_ANSWER_SHEET | Grades | Delete |

---

## Organisation Context Per Action

Every action falls into one of three organisation scopes. The scope determines
which roles are loaded for the permission check.

### System-level (only System or Super users can perform)
No school context. Roles loaded: system-scoped (scopes WHERE school IS NULL).

```
CREATE_SCHOOL
CREATE_PLAN, UPDATE_PLAN, DELETE_PLAN
CREATE_SUBJECT, UPDATE_SUBJECT, DELETE_SUBJECT
CREATE_TOPIC, UPDATE_TOPIC, DELETE_TOPIC
UPDATE_ROLE, DELETE_ROLE
DELETE_USER
```

### Account-level (any authenticated user on their own record)
No role check. The action handler validates record ownership server-side.

```
UPDATE_USER  — only when the target user id == current user id
              (if target != current user → treat as System-level)
```

### School-scoped (requires school membership + role permissions)
Most actions. The school_id is determined from the action payload (see below).

```
All other actions not listed above.
```

---

## Determining School ID From Action Payload

Before evaluating permissions, the client must determine which school the action
targets. The rules mirror `action_organisation()` on the server:

| Pattern | Actions | How to get school_id |
|---|---|---|
| School is the record being operated on | UPDATE_SCHOOL, DELETE_SCHOOL | The `id` field in the payload IS the school id |
| School is payload field 1 (most common) | CREATE_TEACHER, UPDATE_TEACHER, DELETE_TEACHER, CREATE_STAFF, UPDATE_STAFF, DELETE_STAFF, CREATE_OWNER, DELETE_OWNER, CREATE_STUDENT, UPDATE_STUDENT, DELETE_STUDENT, ENROLL_STUDENT, UNENROLL_STUDENT, CREATE_GUARDIAN, UPDATE_GUARDIAN, DELETE_GUARDIAN, CREATE_DEPARTMENT, UPDATE_DEPARTMENT, DELETE_DEPARTMENT, CREATE_TERM, UPDATE_TERM, DELETE_TERM, ASSIGN_CLASS_TEACHER, UNASSIGN_CLASS_TEACHER, ASSIGN_SUBJECT, UNASSIGN_SUBJECT, CREATE_TIMETABLE_ENTRY, UPDATE_TIMETABLE_ENTRY, DELETE_TIMETABLE_ENTRY, MARK_ATTENDANCE, DELETE_ATTENDANCE, CREATE_LESSON, DELETE_LESSON, CREATE_PAPER, UPDATE_PAPER, DELETE_PAPER, MARK_GRADES, UPDATE_GRADE, DELETE_GRADE, UPDATE_MASTERY, CREATE_STREAM, UPDATE_STREAM, DELETE_STREAM, CREATE_MPESA, UPDATE_MPESA, DELETE_MPESA, CREATE_SUBSCRIPTION, UPDATE_SUBSCRIPTION, DELETE_SUBSCRIPTION, CREATE_DISCOUNT, UPDATE_DISCOUNT, DELETE_DISCOUNT, UPDATE_AI_USAGE, UPDATE_SETTINGS, UPLOAD_SCHEME, DELETE_SCHEME, UPLOAD_ANSWER_SHEET, DELETE_ANSWER_SHEET | payload.school |
| School is payload field 2 (id is field 1) | CREATE_EXAM, CREATE_FEE, CREATE_INVOICE, CREATE_ANNOUNCEMENT | payload.school |
| School is optional in payload | CREATE_ROLE, ASSIGN_ROLE, UNASSIGN_ROLE | payload.school (if null/absent → System-level) |
| School must be looked up from local DB | UPDATE_EXAM, DELETE_EXAM | SELECT school FROM exams WHERE id = payload.id |
| School must be looked up from local DB | UPDATE_FEE, DELETE_FEE | SELECT school FROM fees WHERE id = payload.id |
| School must be looked up from local DB | UPDATE_INVOICE, DELETE_INVOICE | SELECT school FROM invoices WHERE id = payload.id |
| School must be looked up from local DB | UPDATE_PAYMENT, DELETE_PAYMENT, APPROVE_PAYMENT | SELECT school FROM payments WHERE id = payload.id |
| School must be looked up from local DB | UPDATE_ANNOUNCEMENT, DELETE_ANNOUNCEMENT | SELECT school FROM announcements WHERE id = payload.id |
| Optional school in field 3 | CREATE_PAYMENT | payload.school (if null → System-level) |

---

## The Authorization Algorithm

```
function canPerformAction(actionId, payload, currentUser, db):

  // Step 1: Super users bypass everything
  if currentUser.level == 2:
    return { allowed: true }

  // Step 2: Determine organisation context
  organisation = determineOrganisation(actionId, payload, currentUser.id, db)

  // Step 3: Get required (resource, action)
  (resource, requiredAction) = actionPermission(actionId)

  // Step 4: Evaluate by organisation type

  if organisation == SYSTEM:
    if currentUser.level < 1:
      return { allowed: false, reason: "This operation requires system access." }
    roles = loadSystemRoles(currentUser.id, db)
    granted = aggregatePermissions(roles)
    if granted[resource] & requiredAction != 0:
      return { allowed: true }
    else:
      return { allowed: false, reason: permissionDeniedMessage(resource, requiredAction) }

  if organisation == ACCOUNT:
    // Any active user can update their own profile
    return { allowed: true }

  if organisation == SCHOOL(schoolId):
    // Check school exists and is active
    school = db.query("SELECT status FROM schools WHERE id = ?", schoolId)
    if school == null:
      return { allowed: false, reason: "School not found." }
    if school.status != 1:
      return { allowed: false, reason: "This school is not currently active." }

    // School owners bypass all role checks
    isOwner = db.query("SELECT 1 FROM owners WHERE school = ? AND user = ?", schoolId, currentUser.id)
    if isOwner:
      return { allowed: true }

    // Load school-scoped roles
    roles = loadSchoolRoles(currentUser.id, schoolId, db)

    // System users also get system-scoped roles merged in
    if currentUser.level == 1:
      systemRoles = loadSystemRoles(currentUser.id, db)
      roles = roles + systemRoles

    granted = aggregatePermissions(roles)
    if granted[resource] & requiredAction != 0:
      return { allowed: true }
    else:
      return { allowed: false, reason: permissionDeniedMessage(resource, requiredAction) }


function loadSchoolRoles(userId, schoolId, db):
  // Returns all roles assigned to userId within schoolId
  return db.query("""
    SELECT r.id, r.permissions
    FROM roles r
    INNER JOIN scopes s ON s.role = r.id
    WHERE s.user = ? AND s.school = ?
  """, userId, schoolId)

function loadSystemRoles(userId, db):
  // Returns all system-scoped roles (school IS NULL) for userId
  return db.query("""
    SELECT r.id, r.permissions
    FROM roles r
    INNER JOIN scopes s ON s.role = r.id
    WHERE s.user = ? AND s.school IS NULL
  """, userId)

function aggregatePermissions(roles):
  // Returns a map: resource_id -> u16 bitmask
  result = Map<int, int>()
  for role in roles:
    parsed = parsePermissionsBlob(role.permissions)
    for (resourceId, actionBits) in parsed:
      result[resourceId] = (result[resourceId] ?? 0) | actionBits
  return result

function parsePermissionsBlob(bytes):
  // Parse the 3-byte-per-resource binary format
  result = []
  i = 0
  while i + 2 < bytes.length:
    resourceId = bytes[i]
    actionBits = bytes[i+1] | (bytes[i+2] << 8)   // little-endian u16
    result.append((resourceId, actionBits))
    i += 3
  return result
```

---

## Dart / Flutter Implementation

### Permissions Parser

```dart
class Permissions {
  final Map<int, int> _data; // resource_id -> u16 actions bitmask

  Permissions._(this._data);

  factory Permissions.fromBytes(Uint8List bytes) {
    final data = <int, int>{};
    for (var i = 0; i + 2 < bytes.length; i += 3) {
      final resourceId = bytes[i];
      final actions = bytes[i + 1] | (bytes[i + 2] << 8);
      data[resourceId] = actions;
    }
    return Permissions._(data);
  }

  factory Permissions.empty() => Permissions._({});

  bool contains(int resource, int action) {
    return ((_data[resource] ?? 0) & action) != 0;
  }

  Permissions merge(Permissions other) {
    final merged = Map<int, int>.from(_data);
    for (final entry in other._data.entries) {
      merged[entry.key] = (merged[entry.key] ?? 0) | entry.value;
    }
    return Permissions._(merged);
  }
}
```

### Resource and Action Constants

```dart
abstract class Resource {
  static const int users         = 1;
  static const int schools       = 2;
  static const int owners        = 3;
  static const int teachers      = 4;
  static const int staff         = 5;
  static const int students      = 6;
  static const int departments   = 7;
  static const int classes       = 8;
  static const int attendance    = 9;
  static const int lessons       = 10;
  static const int exams         = 11;
  static const int grades        = 12;
  static const int fees          = 13;
  static const int payments      = 14;
  static const int announcements = 15;
  static const int roles         = 16;
  static const int plans         = 17;
  static const int ai            = 18;
  static const int subjects      = 19;
}

abstract class ActionBit {
  static const int create   = 1;
  static const int read     = 2;
  static const int update   = 4;
  static const int delete   = 8;
  static const int purge    = 16;
  static const int assign   = 32;
  static const int unassign = 64;
  static const int mark     = 128;
  static const int approve  = 256;
}
```

### AuthorizationService

```dart
enum Organisation {
  system,
  account,
  school,
}

class OrgContext {
  final Organisation type;
  final String? schoolId; // non-null when type == school

  const OrgContext.system() : type = Organisation.system, schoolId = null;
  const OrgContext.account() : type = Organisation.account, schoolId = null;
  const OrgContext.school(String id) : type = Organisation.school, schoolId = id;
}

class PermissionResult {
  final bool allowed;
  final String? reason; // human-readable denial reason, null when allowed

  const PermissionResult.allow() : allowed = true, reason = null;
  const PermissionResult.deny(this.reason) : allowed = false;
}

class AuthorizationService {
  final Database _db; // your local SQLite wrapper

  AuthorizationService(this._db);

  Future<PermissionResult> check({
    required int actionId,
    required String? schoolId, // provide when known from payload
    required String? recordId, // provide for update/delete lookups
  }) async {
    final user = await _db.currentUser();
    if (user == null) return const PermissionResult.deny("Not authenticated.");
    if (user.level == 2) return const PermissionResult.allow(); // Super

    final org = await _resolveOrganisation(actionId, schoolId, recordId, user.id);
    final (resource, action) = _actionPermission(actionId);

    switch (org.type) {
      case Organisation.system:
        if (user.level < 1) {
          return const PermissionResult.deny(
            "This operation requires system-level access.",
          );
        }
        final roles = await _loadSystemRoles(user.id);
        final granted = _aggregate(roles);
        return granted.contains(resource, action)
            ? const PermissionResult.allow()
            : PermissionResult.deny(_denialMessage(resource, action));

      case Organisation.account:
        return const PermissionResult.allow();

      case Organisation.school:
        final sid = org.schoolId!;
        final schoolStatus = await _db.queryScalar<int>(
          'SELECT status FROM schools WHERE id = ?', [sid],
        );
        if (schoolStatus == null) {
          return const PermissionResult.deny("School not found.");
        }
        if (schoolStatus != 1) {
          return const PermissionResult.deny("This school is not currently active.");
        }

        final isOwner = await _db.exists(
          'SELECT 1 FROM owners WHERE school = ? AND user = ?', [sid, user.id],
        );
        if (isOwner) return const PermissionResult.allow();

        var roles = await _loadSchoolRoles(user.id, sid);
        if (user.level == 1) {
          final systemRoles = await _loadSystemRoles(user.id);
          roles = [...roles, ...systemRoles];
        }

        final granted = _aggregate(roles);
        return granted.contains(resource, action)
            ? const PermissionResult.allow()
            : PermissionResult.deny(_denialMessage(resource, action));
    }
  }

  Future<OrgContext> _resolveOrganisation(
    int actionId,
    String? schoolId,
    String? recordId,
    String userId,
  ) async {
    // System-level actions
    const systemActions = {
      SyncAction.createSchool,
      SyncAction.createPlan, SyncAction.updatePlan, SyncAction.deletePlan,
      SyncAction.createSubject, SyncAction.updateSubject, SyncAction.deleteSubject,
      SyncAction.createTopic, SyncAction.updateTopic, SyncAction.deleteTopic,
      SyncAction.updateRole, SyncAction.deleteRole,
      SyncAction.deleteUser,
    };
    if (systemActions.contains(actionId)) return const OrgContext.system();

    // Account-level (own user only)
    if (actionId == SyncAction.updateUser) {
      return recordId == userId
          ? const OrgContext.account()
          : const OrgContext.system();
    }

    // DB lookup cases: school not in payload
    if (recordId != null) {
      String? lookedUpSchool;
      switch (actionId) {
        case SyncAction.updateExam:
        case SyncAction.deleteExam:
          lookedUpSchool = await _db.queryScalar<String>(
            'SELECT school FROM exams WHERE id = ?', [recordId]);
        case SyncAction.updateFee:
        case SyncAction.deleteFee:
          lookedUpSchool = await _db.queryScalar<String>(
            'SELECT school FROM fees WHERE id = ?', [recordId]);
        case SyncAction.updateInvoice:
        case SyncAction.deleteInvoice:
          lookedUpSchool = await _db.queryScalar<String>(
            'SELECT school FROM invoices WHERE id = ?', [recordId]);
        case SyncAction.updatePayment:
        case SyncAction.deletePayment:
        case SyncAction.approvePayment:
          lookedUpSchool = await _db.queryScalar<String>(
            'SELECT school FROM payments WHERE id = ?', [recordId]);
        case SyncAction.updateAnnouncement:
        case SyncAction.deleteAnnouncement:
          lookedUpSchool = await _db.queryScalar<String>(
            'SELECT school FROM announcements WHERE id = ?', [recordId]);
      }
      if (lookedUpSchool != null) {
        return OrgContext.school(lookedUpSchool);
      }
    }

    // Role assignment: school may be absent → system
    if (actionId == SyncAction.createRole ||
        actionId == SyncAction.assignRole ||
        actionId == SyncAction.unassignRole) {
      return schoolId != null && schoolId.isNotEmpty
          ? OrgContext.school(schoolId)
          : const OrgContext.system();
    }

    // All other actions: school from payload
    if (schoolId != null && schoolId.isNotEmpty) {
      return OrgContext.school(schoolId);
    }

    // Fallback: treat as system if school cannot be determined
    return const OrgContext.system();
  }

  Future<List<({Uint8List permissions})>> _loadSchoolRoles(
    String userId, String schoolId) async {
    return _db.query(
      '''SELECT r.permissions FROM roles r
         INNER JOIN scopes s ON s.role = r.id
         WHERE s.user = ? AND s.school = ?''',
      [userId, schoolId],
    );
  }

  Future<List<({Uint8List permissions})>> _loadSystemRoles(String userId) async {
    return _db.query(
      '''SELECT r.permissions FROM roles r
         INNER JOIN scopes s ON s.role = r.id
         WHERE s.user = ? AND s.school IS NULL''',
      [userId],
    );
  }

  Permissions _aggregate(List<({Uint8List permissions})> roles) {
    var result = Permissions.empty();
    for (final role in roles) {
      result = result.merge(Permissions.fromBytes(role.permissions));
    }
    return result;
  }

  static (int resource, int action) _actionPermission(int actionId) {
    return switch (actionId) {
      SyncAction.createSchool     => (Resource.schools, ActionBit.create),
      SyncAction.updateSchool     => (Resource.schools, ActionBit.update),
      SyncAction.deleteSchool     => (Resource.schools, ActionBit.delete),
      SyncAction.createTeacher    => (Resource.teachers, ActionBit.create),
      SyncAction.updateTeacher    => (Resource.teachers, ActionBit.update),
      SyncAction.deleteTeacher    => (Resource.teachers, ActionBit.delete),
      SyncAction.createStaff      => (Resource.staff, ActionBit.create),
      SyncAction.updateStaff      => (Resource.staff, ActionBit.update),
      SyncAction.deleteStaff      => (Resource.staff, ActionBit.delete),
      SyncAction.createOwner      => (Resource.owners, ActionBit.create),
      SyncAction.deleteOwner      => (Resource.owners, ActionBit.delete),
      SyncAction.createStudent    => (Resource.students, ActionBit.create),
      SyncAction.updateStudent    => (Resource.students, ActionBit.update),
      SyncAction.deleteStudent    => (Resource.students, ActionBit.delete),
      SyncAction.enrollStudent    => (Resource.students, ActionBit.assign),
      SyncAction.unenrollStudent  => (Resource.students, ActionBit.unassign),
      SyncAction.createGuardian   => (Resource.students, ActionBit.create),
      SyncAction.updateGuardian   => (Resource.students, ActionBit.update),
      SyncAction.deleteGuardian   => (Resource.students, ActionBit.delete),
      SyncAction.createDepartment => (Resource.departments, ActionBit.create),
      SyncAction.updateDepartment => (Resource.departments, ActionBit.update),
      SyncAction.deleteDepartment => (Resource.departments, ActionBit.delete),
      SyncAction.createTerm       => (Resource.schools, ActionBit.create),
      SyncAction.updateTerm       => (Resource.schools, ActionBit.update),
      SyncAction.deleteTerm       => (Resource.schools, ActionBit.delete),
      SyncAction.assignClassTeacher   => (Resource.classes, ActionBit.assign),
      SyncAction.unassignClassTeacher => (Resource.classes, ActionBit.unassign),
      SyncAction.assignSubject    => (Resource.classes, ActionBit.assign),
      SyncAction.unassignSubject  => (Resource.classes, ActionBit.unassign),
      SyncAction.createTimetableEntry => (Resource.classes, ActionBit.create),
      SyncAction.updateTimetableEntry => (Resource.classes, ActionBit.update),
      SyncAction.deleteTimetableEntry => (Resource.classes, ActionBit.delete),
      SyncAction.markAttendance   => (Resource.attendance, ActionBit.mark),
      SyncAction.deleteAttendance => (Resource.attendance, ActionBit.delete),
      SyncAction.createLesson     => (Resource.lessons, ActionBit.create),
      SyncAction.deleteLesson     => (Resource.lessons, ActionBit.delete),
      SyncAction.createExam       => (Resource.exams, ActionBit.create),
      SyncAction.updateExam       => (Resource.exams, ActionBit.update),
      SyncAction.deleteExam       => (Resource.exams, ActionBit.delete),
      SyncAction.createPaper      => (Resource.exams, ActionBit.create),
      SyncAction.updatePaper      => (Resource.exams, ActionBit.update),
      SyncAction.deletePaper      => (Resource.exams, ActionBit.delete),
      SyncAction.markGrades       => (Resource.grades, ActionBit.mark),
      SyncAction.updateGrade      => (Resource.grades, ActionBit.update),
      SyncAction.deleteGrade      => (Resource.grades, ActionBit.delete),
      SyncAction.updateMastery    => (Resource.grades, ActionBit.mark),
      SyncAction.createFee        => (Resource.fees, ActionBit.create),
      SyncAction.updateFee        => (Resource.fees, ActionBit.update),
      SyncAction.deleteFee        => (Resource.fees, ActionBit.delete),
      SyncAction.createInvoice    => (Resource.fees, ActionBit.create),
      SyncAction.updateInvoice    => (Resource.fees, ActionBit.update),
      SyncAction.deleteInvoice    => (Resource.fees, ActionBit.delete),
      SyncAction.createPayment    => (Resource.payments, ActionBit.create),
      SyncAction.updatePayment    => (Resource.payments, ActionBit.update),
      SyncAction.deletePayment    => (Resource.payments, ActionBit.delete),
      SyncAction.approvePayment   => (Resource.payments, ActionBit.approve),
      SyncAction.createAnnouncement => (Resource.announcements, ActionBit.create),
      SyncAction.updateAnnouncement => (Resource.announcements, ActionBit.update),
      SyncAction.deleteAnnouncement => (Resource.announcements, ActionBit.delete),
      SyncAction.createRole       => (Resource.roles, ActionBit.create),
      SyncAction.updateRole       => (Resource.roles, ActionBit.update),
      SyncAction.deleteRole       => (Resource.roles, ActionBit.delete),
      SyncAction.assignRole       => (Resource.roles, ActionBit.assign),
      SyncAction.unassignRole     => (Resource.roles, ActionBit.unassign),
      SyncAction.updateUser       => (Resource.users, ActionBit.update),
      SyncAction.deleteUser       => (Resource.users, ActionBit.delete),
      SyncAction.updateSettings   => (Resource.schools, ActionBit.update),
      SyncAction.createPlan       => (Resource.plans, ActionBit.create),
      SyncAction.updatePlan       => (Resource.plans, ActionBit.update),
      SyncAction.deletePlan       => (Resource.plans, ActionBit.delete),
      SyncAction.updateAiUsage    => (Resource.ai, ActionBit.update),
      SyncAction.createSubscription => (Resource.plans, ActionBit.create),
      SyncAction.updateSubscription => (Resource.plans, ActionBit.update),
      SyncAction.deleteSubscription => (Resource.plans, ActionBit.delete),
      SyncAction.createDiscount   => (Resource.plans, ActionBit.create),
      SyncAction.updateDiscount   => (Resource.plans, ActionBit.update),
      SyncAction.deleteDiscount   => (Resource.plans, ActionBit.delete),
      SyncAction.createSubject    => (Resource.subjects, ActionBit.create),
      SyncAction.updateSubject    => (Resource.subjects, ActionBit.update),
      SyncAction.deleteSubject    => (Resource.subjects, ActionBit.delete),
      SyncAction.createTopic      => (Resource.subjects, ActionBit.create),
      SyncAction.updateTopic      => (Resource.subjects, ActionBit.update),
      SyncAction.deleteTopic      => (Resource.subjects, ActionBit.delete),
      SyncAction.createStream     => (Resource.schools, ActionBit.create),
      SyncAction.updateStream     => (Resource.schools, ActionBit.update),
      SyncAction.deleteStream     => (Resource.schools, ActionBit.delete),
      SyncAction.createMpesa      => (Resource.schools, ActionBit.create),
      SyncAction.updateMpesa      => (Resource.schools, ActionBit.update),
      SyncAction.deleteMpesa      => (Resource.schools, ActionBit.delete),
      SyncAction.uploadScheme     => (Resource.exams, ActionBit.update),
      SyncAction.deleteScheme     => (Resource.exams, ActionBit.delete),
      SyncAction.uploadAnswerSheet => (Resource.grades, ActionBit.mark),
      SyncAction.deleteAnswerSheet => (Resource.grades, ActionBit.delete),
      _ => throw ArgumentError('Unknown action: $actionId'),
    };
  }

  static String _denialMessage(int resource, int action) {
    final resourceName = switch (resource) {
      Resource.users         => 'users',
      Resource.schools       => 'school settings',
      Resource.owners        => 'school owners',
      Resource.teachers      => 'teachers',
      Resource.staff         => 'staff',
      Resource.students      => 'students',
      Resource.departments   => 'departments',
      Resource.classes       => 'classes',
      Resource.attendance    => 'attendance',
      Resource.lessons       => 'lessons',
      Resource.exams         => 'exams',
      Resource.grades        => 'grades',
      Resource.fees          => 'fees',
      Resource.payments      => 'payments',
      Resource.announcements => 'announcements',
      Resource.roles         => 'roles',
      Resource.plans         => 'subscription plans',
      Resource.ai            => 'AI usage',
      Resource.subjects      => 'subjects',
      _ => 'this resource',
    };
    final actionName = switch (action) {
      ActionBit.create   => 'create',
      ActionBit.update   => 'update',
      ActionBit.delete   => 'delete',
      ActionBit.assign   => 'assign',
      ActionBit.unassign => 'remove',
      ActionBit.mark     => 'record',
      ActionBit.approve  => 'approve',
      _ => 'perform this action on',
    };
    return "You don't have permission to $actionName $resourceName.";
  }
}
```

---

## TypeScript / Svelte / SolidJS Implementation

This is the implementation for the future web frontend.

### Permissions Parser

```typescript
export class Permissions {
  private data: Map<number, number>; // resource -> u16 bitmask

  constructor(data: Map<number, number> = new Map()) {
    this.data = data;
  }

  static fromBytes(bytes: Uint8Array): Permissions {
    const data = new Map<number, number>();
    for (let i = 0; i + 2 < bytes.length; i += 3) {
      const resourceId = bytes[i];
      const actions = bytes[i + 1] | (bytes[i + 2] << 8); // little-endian u16
      data.set(resourceId, actions);
    }
    return new Permissions(data);
  }

  static empty(): Permissions {
    return new Permissions();
  }

  contains(resource: number, action: number): boolean {
    return ((this.data.get(resource) ?? 0) & action) !== 0;
  }

  merge(other: Permissions): Permissions {
    const merged = new Map(this.data);
    for (const [resource, actions] of other.data) {
      merged.set(resource, (merged.get(resource) ?? 0) | actions);
    }
    return new Permissions(merged);
  }
}
```

### Resource and Action Constants

```typescript
export const Resource = {
  Users:         1,
  Schools:       2,
  Owners:        3,
  Teachers:      4,
  Staff:         5,
  Students:      6,
  Departments:   7,
  Classes:       8,
  Attendance:    9,
  Lessons:       10,
  Exams:         11,
  Grades:        12,
  Fees:          13,
  Payments:      14,
  Announcements: 15,
  Roles:         16,
  Plans:         17,
  AI:            18,
  Subjects:      19,
} as const;

export const ActionBit = {
  Create:   1,
  Read:     2,
  Update:   4,
  Delete:   8,
  Purge:    16,
  Assign:   32,
  Unassign: 64,
  Mark:     128,
  Approve:  256,
} as const;
```

### Authorization Function

```typescript
type OrgContext =
  | { type: 'system' }
  | { type: 'account' }
  | { type: 'school'; schoolId: string };

export interface PermissionResult {
  allowed: boolean;
  reason?: string;
}

interface CurrentUser {
  id: string;
  level: number; // 0=Normal, 1=System, 2=Super
  status: number;
}

// db is your local SQLite wrapper (e.g., tauri-plugin-sql or sql.js)
export async function canPerformAction(
  actionId: number,
  schoolId: string | null | undefined,
  recordId: string | null | undefined,
  currentUser: CurrentUser,
  db: LocalDatabase,
): Promise<PermissionResult> {

  // Super bypass
  if (currentUser.level === 2) return { allowed: true };

  const org = await resolveOrganisation(actionId, schoolId, recordId, currentUser.id, db);
  const [resource, action] = actionPermission(actionId);

  if (org.type === 'system') {
    if (currentUser.level < 1) {
      return { allowed: false, reason: 'This operation requires system-level access.' };
    }
    const roles = await loadSystemRoles(currentUser.id, db);
    const granted = aggregatePermissions(roles);
    return granted.contains(resource, action)
      ? { allowed: true }
      : { allowed: false, reason: denialMessage(resource, action) };
  }

  if (org.type === 'account') {
    return { allowed: true };
  }

  // school context
  const { schoolId: sid } = org;
  const schoolStatus = await db.queryScalar<number>(
    'SELECT status FROM schools WHERE id = ?', [sid]
  );
  if (schoolStatus == null) return { allowed: false, reason: 'School not found.' };
  if (schoolStatus !== 1) return { allowed: false, reason: 'This school is not currently active.' };

  const isOwner = await db.exists(
    'SELECT 1 FROM owners WHERE school = ? AND user = ?', [sid, currentUser.id]
  );
  if (isOwner) return { allowed: true };

  let roles = await loadSchoolRoles(currentUser.id, sid, db);
  if (currentUser.level === 1) {
    const systemRoles = await loadSystemRoles(currentUser.id, db);
    roles = [...roles, ...systemRoles];
  }

  const granted = aggregatePermissions(roles);
  return granted.contains(resource, action)
    ? { allowed: true }
    : { allowed: false, reason: denialMessage(resource, action) };
}

async function resolveOrganisation(
  actionId: number,
  schoolId: string | null | undefined,
  recordId: string | null | undefined,
  userId: string,
  db: LocalDatabase,
): Promise<OrgContext> {

  const SYSTEM_ACTIONS = new Set([
    SyncAction.CreateSchool,
    SyncAction.CreatePlan, SyncAction.UpdatePlan, SyncAction.DeletePlan,
    SyncAction.CreateSubject, SyncAction.UpdateSubject, SyncAction.DeleteSubject,
    SyncAction.CreateTopic, SyncAction.UpdateTopic, SyncAction.DeleteTopic,
    SyncAction.UpdateRole, SyncAction.DeleteRole,
    SyncAction.DeleteUser,
  ]);

  if (SYSTEM_ACTIONS.has(actionId)) return { type: 'system' };

  if (actionId === SyncAction.UpdateUser) {
    return recordId === userId ? { type: 'account' } : { type: 'system' };
  }

  // DB lookup cases
  const lookupTable: Partial<Record<number, string>> = {
    [SyncAction.UpdateExam]:         'SELECT school FROM exams WHERE id = ?',
    [SyncAction.DeleteExam]:         'SELECT school FROM exams WHERE id = ?',
    [SyncAction.UpdateFee]:          'SELECT school FROM fees WHERE id = ?',
    [SyncAction.DeleteFee]:          'SELECT school FROM fees WHERE id = ?',
    [SyncAction.UpdateInvoice]:      'SELECT school FROM invoices WHERE id = ?',
    [SyncAction.DeleteInvoice]:      'SELECT school FROM invoices WHERE id = ?',
    [SyncAction.UpdatePayment]:      'SELECT school FROM payments WHERE id = ?',
    [SyncAction.DeletePayment]:      'SELECT school FROM payments WHERE id = ?',
    [SyncAction.ApprovePayment]:     'SELECT school FROM payments WHERE id = ?',
    [SyncAction.UpdateAnnouncement]: 'SELECT school FROM announcements WHERE id = ?',
    [SyncAction.DeleteAnnouncement]: 'SELECT school FROM announcements WHERE id = ?',
  };

  const lookupSql = lookupTable[actionId];
  if (lookupSql && recordId) {
    const sid = await db.queryScalar<string>(lookupSql, [recordId]);
    if (sid) return { type: 'school', schoolId: sid };
  }

  // Role actions: optional school
  if (
    actionId === SyncAction.CreateRole ||
    actionId === SyncAction.AssignRole ||
    actionId === SyncAction.UnassignRole
  ) {
    return schoolId ? { type: 'school', schoolId } : { type: 'system' };
  }

  // General school-scoped
  if (schoolId) return { type: 'school', schoolId };

  return { type: 'system' };
}

async function loadSchoolRoles(
  userId: string,
  schoolId: string,
  db: LocalDatabase,
): Promise<Array<{ permissions: Uint8Array }>> {
  return db.query(
    `SELECT r.permissions FROM roles r
     INNER JOIN scopes s ON s.role = r.id
     WHERE s.user = ? AND s.school = ?`,
    [userId, schoolId],
  );
}

async function loadSystemRoles(
  userId: string,
  db: LocalDatabase,
): Promise<Array<{ permissions: Uint8Array }>> {
  return db.query(
    `SELECT r.permissions FROM roles r
     INNER JOIN scopes s ON s.role = r.id
     WHERE s.user = ? AND s.school IS NULL`,
    [userId],
  );
}

function aggregatePermissions(roles: Array<{ permissions: Uint8Array }>): Permissions {
  return roles.reduce(
    (acc, role) => acc.merge(Permissions.fromBytes(role.permissions)),
    Permissions.empty(),
  );
}

function denialMessage(resource: number, action: number): string {
  const resourceNames: Record<number, string> = {
    1: 'users', 2: 'school settings', 3: 'school owners', 4: 'teachers',
    5: 'staff', 6: 'students', 7: 'departments', 8: 'classes',
    9: 'attendance', 10: 'lessons', 11: 'exams', 12: 'grades',
    13: 'fees', 14: 'payments', 15: 'announcements', 16: 'roles',
    17: 'subscription plans', 18: 'AI usage', 19: 'subjects',
  };
  const actionNames: Record<number, string> = {
    1: 'create', 4: 'update', 8: 'delete', 32: 'assign',
    64: 'remove', 128: 'record', 256: 'approve',
  };
  const r = resourceNames[resource] ?? 'this resource';
  const a = actionNames[action] ?? 'perform this action on';
  return `You don't have permission to ${a} ${r}.`;
}
```

---

## Where To Call the Check

The check should happen at the point of user intent — before the action is
written to the local sync queue or the local database is updated optimistically.

### In Flutter (Dart)

```dart
// In your repository or use-case layer, before queuing any action:

Future<void> deleteExam(String examId) async {
  final result = await _authorizationService.check(
    actionId: SyncAction.deleteExam,
    schoolId: null,     // not in payload — service does DB lookup
    recordId: examId,   // used for lookup
  );

  if (!result.allowed) {
    throw PermissionException(result.reason!);
    // or: show a snackbar / dialog to the user
  }

  // Only reaches here if allowed
  await _db.deleteExam(examId);
  _syncQueue.enqueue(SyncAction.deleteExam, payload: DeleteExamPayload(id: examId));
}
```

### In Svelte / SolidJS

```typescript
// In your store action or event handler:

async function handleDeleteExam(examId: string) {
  const result = await canPerformAction(
    SyncAction.DeleteExam,
    null,       // school resolved from local DB
    examId,     // record id for lookup
    currentUser,
    db,
  );

  if (!result.allowed) {
    showToast({ type: 'error', message: result.reason });
    return;
  }

  // Proceed with optimistic local update
  examsStore.delete(examId);
  syncQueue.enqueue(SyncAction.DeleteExam, { id: examId });
}
```

---

## UI Permission Gating (Hiding Controls)

Beyond blocking queued actions, also use the permission check to hide or disable
UI controls that the user cannot use. This avoids showing buttons that always
produce an error.

```typescript
// Svelte example — hiding delete button on exam card:
{#await canPerformAction(SyncAction.DeleteExam, null, exam.id, $currentUser, db)}
  <!-- loading -->
{:then result}
  {#if result.allowed}
    <button on:click={() => handleDeleteExam(exam.id)}>Delete</button>
  {/if}
{/await}
```

For performance in list views, pre-compute a permission map for the current user
and school at page load rather than calling the async check per row. Cache it
and invalidate when scopes/roles change.

```typescript
// Pre-compute a PermissionMap for a school context
interface PermissionMap {
  can(resource: number, action: number): boolean;
}

async function buildPermissionMap(
  userId: string,
  schoolId: string,
  userLevel: number,
  db: LocalDatabase,
): Promise<PermissionMap> {
  if (userLevel === 2) {
    // Super: everything allowed
    return { can: () => true };
  }

  const isOwner = await db.exists(
    'SELECT 1 FROM owners WHERE school = ? AND user = ?', [schoolId, userId]
  );
  if (isOwner) {
    return { can: () => true };
  }

  let roles = await loadSchoolRoles(userId, schoolId, db);
  if (userLevel === 1) {
    roles = [...roles, ...await loadSystemRoles(userId, db)];
  }

  const granted = aggregatePermissions(roles);
  return { can: (resource, action) => granted.contains(resource, action) };
}
```

---

## Keeping Client and Server in Sync

The client permission check is a convenience layer, not a security boundary.
The server is always the authority. These two rules prevent drift:

1. **The action→(resource, action) mapping is the single source of truth in
   `src/db/database/tables/actions.rs` (`action_permission` function).** Any
   time a new action is added to the server, the same mapping must be added to
   both `client.md` (the table above) and the client implementation.

2. **The permissions binary format is defined in
   `src/types/role/permissions.rs`.** If the format changes (it will not during
   normal development — only if resources are added), the client parsers must
   be updated simultaneously.

When adding a new sync action:
- Add it to `action_permission()` in `actions.rs` (server)
- Add it to the action→(resource, action) table in this document
- Add it to `_actionPermission()` / `actionPermission()` in the client
- Add it to `SyncAction` constants in the client
- Add it to `execute_action()` and `action_organisation()` in `actions.rs`
