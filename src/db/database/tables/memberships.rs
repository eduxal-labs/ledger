use crate::db::database::traits::Load;
use crate::db::schema::{guardians, owners, staff, students, teachers};
use crate::types::error::Result;
use crate::types::id::Id;
use crate::types::user::User;
use diesel::SqliteConnection as Conn;
use diesel::{ExpressionMethods, NullableExpressionMethods, QueryDsl, RunQueryDsl};
use std::collections::HashSet;

/// Load all distinct school IDs where a user is a member.
///
/// A user is considered a member of a school if they appear in any of
/// the five member tables: `owners`, `teachers`, `staff`, `students`,
/// or `guardians`.
///
/// Note: `students` links via an optional `user` column (nullable text),
/// while the other four tables have a non-nullable `user` column.
impl Load<&User, Id> for Conn {
    fn load(&mut self, user: &User) -> Result<Vec<Id>> {
        let owner_schools: Vec<Id> = owners::table
            .filter(owners::user.eq(user.id))
            .select(owners::school)
            .load(self)?;

        let teacher_schools: Vec<Id> = teachers::table
            .filter(teachers::user.eq(user.id))
            .select(teachers::school)
            .load(self)?;

        let staff_schools: Vec<Id> = staff::table
            .filter(staff::user.eq(user.id))
            .select(staff::school)
            .load(self)?;

        let student_schools: Vec<Id> = students::table
            .filter(students::user.eq(user.id))
            .select(students::school)
            .load(self)?;

        let guardian_schools: Vec<Id> = guardians::table
            .filter(guardians::user.eq(user.id))
            .select(guardians::school)
            .load(self)?;

        // Merge and deduplicate via HashSet (Id implements Hash + Eq)
        let mut set = HashSet::with_capacity(
            owner_schools.len()
                + teacher_schools.len()
                + staff_schools.len()
                + student_schools.len()
                + guardian_schools.len(),
        );
        set.extend(owner_schools);
        set.extend(teacher_schools);
        set.extend(staff_schools);
        set.extend(student_schools);
        set.extend(guardian_schools);

        Ok(set.into_iter().collect())
    }
}

/// Load all distinct user IDs that are members of the given schools.
///
/// Queries the five member tables (`owners`, `teachers`, `staff`,
/// `students`, `guardians`) for rows whose `school` is in the
/// provided set, then merges and deduplicates the user IDs.
///
/// Used by the sync engine to build the co-member visibility set so
/// that Normal (and System) users can see user rows for people who
/// share a school with them.
impl Load<&HashSet<Id>, Id> for Conn {
    fn load(&mut self, school_ids: &HashSet<Id>) -> Result<Vec<Id>> {
        if school_ids.is_empty() {
            return Ok(vec![]);
        }

        let ids: Vec<Id> = school_ids.iter().copied().collect();

        let owner_users: Vec<Id> = owners::table
            .filter(owners::school.eq_any(&ids))
            .select(owners::user)
            .load(self)?;

        let teacher_users: Vec<Id> = teachers::table
            .filter(teachers::school.eq_any(&ids))
            .select(teachers::user)
            .load(self)?;

        let staff_users: Vec<Id> = staff::table
            .filter(staff::school.eq_any(&ids))
            .select(staff::user)
            .load(self)?;

        // students.user is nullable — filter out NULLs and unwrap
        let student_users: Vec<Id> = students::table
            .filter(students::school.eq_any(&ids))
            .filter(students::user.is_not_null())
            .select(students::user.assume_not_null())
            .load(self)?;

        let guardian_users: Vec<Id> = guardians::table
            .filter(guardians::school.eq_any(&ids))
            .select(guardians::user)
            .load(self)?;

        let mut set = HashSet::with_capacity(
            owner_users.len()
                + teacher_users.len()
                + staff_users.len()
                + student_users.len()
                + guardian_users.len(),
        );
        set.extend(owner_users);
        set.extend(teacher_users);
        set.extend(staff_users);
        set.extend(student_users);
        set.extend(guardian_users);

        Ok(set.into_iter().collect())
    }
}
