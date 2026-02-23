-- This file should undo anything in `up.sql`

-- Drop triggers first

DROP TRIGGER IF EXISTS lessons_within_term_update;
DROP TRIGGER IF EXISTS lessons_within_term;
DROP TRIGGER IF EXISTS attendance_within_term_update;
DROP TRIGGER IF EXISTS attendance_within_term;
DROP TRIGGER IF EXISTS papers_within_exam_range_update;
DROP TRIGGER IF EXISTS papers_within_exam_range;
DROP TRIGGER IF EXISTS terms_no_overlap_update;
DROP TRIGGER IF EXISTS terms_no_overlap;
DROP TRIGGER IF EXISTS subscriptions_invoice_check_update;
DROP TRIGGER IF EXISTS subscriptions_invoice_check;
DROP TRIGGER IF EXISTS grades_enrollment_check_update;
DROP TRIGGER IF EXISTS grades_paper_mix_check_update;
DROP TRIGGER IF EXISTS exams_stream_consistency_check;
DROP TRIGGER IF EXISTS grades_enrollment_check;
DROP TRIGGER IF EXISTS grades_paper_mix_check;
DROP TRIGGER IF EXISTS dept_delete_clear_staff;
DROP TRIGGER IF EXISTS dept_delete_clear_teachers;

-- Drop unique indexes
DROP INDEX IF EXISTS uq_exams_stream_type;
DROP INDEX IF EXISTS uq_exams_allstream_type;
DROP INDEX IF EXISTS uq_timetable_class_slot;
DROP INDEX IF EXISTS uq_timetable_teacher_slot;
DROP INDEX IF EXISTS uq_guardians_primary;

DROP INDEX IF EXISTS papers_subject_null_idx;
DROP INDEX IF EXISTS subjects_class_teacher_idx;
DROP INDEX IF EXISTS uq_enrollments_student_term;
DROP INDEX IF EXISTS uq_class_teachers_active;
DROP INDEX IF EXISTS scopes_system_idx;
DROP INDEX IF EXISTS roles_system_name_idx;
DROP INDEX IF EXISTS roles_school_name_idx;
DROP INDEX IF EXISTS students_school_user_idx;

-- Drop performance indexes
DROP INDEX IF EXISTS idx_discounts_school_term_grade;

DROP INDEX IF EXISTS idx_subscriptions_school_term;
DROP INDEX IF EXISTS idx_subscriptions_school_student;

DROP INDEX IF EXISTS idx_plans_status;

DROP INDEX IF EXISTS idx_scopes_role;
DROP INDEX IF EXISTS idx_scopes_school_role;

DROP INDEX IF EXISTS idx_announcements_school_grade;
DROP INDEX IF EXISTS idx_announcements_school;

DROP INDEX IF EXISTS idx_payments_direct_date;
DROP INDEX IF EXISTS idx_payments_school_student;
DROP INDEX IF EXISTS idx_payments_invoice;

DROP INDEX IF EXISTS idx_invoices_school_status;
DROP INDEX IF EXISTS idx_invoices_school_term;
DROP INDEX IF EXISTS idx_invoices_school_student;

DROP INDEX IF EXISTS idx_fees_school_term_grade;

DROP INDEX IF EXISTS idx_grades_school_student;

DROP INDEX IF EXISTS idx_papers_school_exam_status;

DROP INDEX IF EXISTS idx_exams_school_teacher;
DROP INDEX IF EXISTS idx_exams_school_term_class;

DROP INDEX IF EXISTS idx_lessons_school_term_date;
DROP INDEX IF EXISTS idx_lessons_school_teacher;

DROP INDEX IF EXISTS idx_timetable_school_teacher;

DROP INDEX IF EXISTS idx_attendance_school_term_student;

DROP INDEX IF EXISTS idx_subjects_school_teacher;

DROP INDEX IF EXISTS idx_enrollments_school_student;

DROP INDEX IF EXISTS idx_class_teachers_school_teacher;

DROP INDEX IF EXISTS idx_staff_school_status;
DROP INDEX IF EXISTS idx_staff_school_department;
DROP INDEX IF EXISTS idx_staff_school;

DROP INDEX IF EXISTS idx_teachers_school_status;
DROP INDEX IF EXISTS idx_teachers_school_department;

DROP INDEX IF EXISTS idx_guardians_school_student;

DROP INDEX IF EXISTS idx_students_school_name;
DROP INDEX IF EXISTS idx_students_school_status;

DROP INDEX IF EXISTS idx_owners_user;

DROP INDEX IF EXISTS idx_schools_domain;
DROP INDEX IF EXISTS idx_schools_county;
DROP INDEX IF EXISTS idx_schools_status;

DROP INDEX IF EXISTS idx_users_status;
DROP INDEX IF EXISTS idx_users_email;

-- Drop tables in reverse dependency order (most dependent first)
DROP TABLE IF EXISTS discounts;
DROP TABLE IF EXISTS subscriptions;
DROP TABLE IF EXISTS plans;

DROP TABLE IF EXISTS scopes;
DROP TABLE IF EXISTS roles;

DROP TABLE IF EXISTS settings;
DROP TABLE IF EXISTS aiusage;
DROP TABLE IF EXISTS mastery;
DROP TABLE IF EXISTS announcements;

DROP TABLE IF EXISTS payments;
DROP TABLE IF EXISTS invoices;
DROP TABLE IF EXISTS fees;

DROP TABLE IF EXISTS grades;
DROP TABLE IF EXISTS papers;
DROP TABLE IF EXISTS exams;
DROP TABLE IF EXISTS lessons;
DROP TABLE IF EXISTS timetable;
DROP TABLE IF EXISTS attendance;
DROP TABLE IF EXISTS subjects;
DROP TABLE IF EXISTS enrollments;
DROP TABLE IF EXISTS class_teachers;

DROP TABLE IF EXISTS terms;

DROP TABLE IF EXISTS staff;
DROP TABLE IF EXISTS teachers;
DROP TABLE IF EXISTS departments;

DROP TABLE IF EXISTS guardians;
DROP TABLE IF EXISTS students;
DROP TABLE IF EXISTS owners;

DROP TABLE IF EXISTS schools;
DROP TABLE IF EXISTS users;
