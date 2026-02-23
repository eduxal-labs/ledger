// @generated automatically by Diesel CLI.

diesel::table! {
    aiusage (school, student, year, term) {
        school -> Text,
        student -> Integer,
        year -> Integer,
        term -> SmallInt,
        allocated -> Integer,
        used -> Integer,
        created -> BigInt,
        updated -> BigInt,
    }
}

diesel::table! {
    announcements (id) {
        id -> Text,
        school -> Text,
        title -> Text,
        content -> Text,
        grade -> Nullable<SmallInt>,
        stream -> Nullable<SmallInt>,
        audience -> Integer,
        author -> Nullable<Text>,
        created -> BigInt,
        updated -> BigInt,
    }
}

diesel::table! {
    attendance (school, year, term, grade, stream, student, date) {
        school -> Text,
        year -> Integer,
        term -> SmallInt,
        grade -> SmallInt,
        stream -> SmallInt,
        student -> Integer,
        date -> Integer,
        status -> SmallInt,
        created -> BigInt,
        updated -> BigInt,
    }
}

diesel::table! {
    class_teachers (school, year, term, grade, stream, teacher) {
        school -> Text,
        year -> Integer,
        term -> SmallInt,
        grade -> SmallInt,
        stream -> SmallInt,
        teacher -> Text,
        start -> Integer,
        end -> Nullable<Integer>,
        created -> BigInt,
    }
}

diesel::table! {
    departments (school, name) {
        school -> Text,
        name -> Text,
        description -> Nullable<Text>,
        created -> BigInt,
        updated -> BigInt,
    }
}

diesel::table! {
    discounts (school, plan, year, term, grade) {
        school -> Text,
        plan -> Text,
        year -> Integer,
        term -> SmallInt,
        grade -> SmallInt,
        amount -> Float,
        unit -> SmallInt,
        created -> BigInt,
        updated -> BigInt,
    }
}

diesel::table! {
    enrollments (school, year, term, grade, stream, student) {
        school -> Text,
        year -> Integer,
        term -> SmallInt,
        grade -> SmallInt,
        stream -> SmallInt,
        student -> Integer,
        created -> BigInt,
    }
}

diesel::table! {
    exams (id) {
        id -> Text,
        school -> Text,
        year -> Integer,
        term -> SmallInt,
        grade -> SmallInt,
        stream -> Nullable<SmallInt>,
        personalized -> Bool,
        #[sql_name = "type"]
        type_ -> SmallInt,
        start -> Integer,
        end -> Integer,
        teacher -> Text,
        created -> BigInt,
        updated -> BigInt,
    }
}

diesel::table! {
    fees (id) {
        id -> Text,
        school -> Text,
        year -> Integer,
        term -> SmallInt,
        grade -> SmallInt,
        title -> Text,
        description -> Text,
        amount -> Float,
        mandatory -> Bool,
        due -> BigInt,
        created -> BigInt,
        updated -> BigInt,
    }
}

diesel::table! {
    grades (school, exam, student, subject, paper) {
        school -> Text,
        exam -> Text,
        student -> Integer,
        subject -> SmallInt,
        paper -> Nullable<SmallInt>,
        score -> Float,
        total -> Integer,
        created -> BigInt,
        updated -> BigInt,
    }
}

diesel::table! {
    guardians (school, user, student) {
        school -> Text,
        user -> Text,
        student -> Integer,
        relationship -> SmallInt,
        role -> SmallInt,
        created -> BigInt,
        updated -> BigInt,
    }
}

diesel::table! {
    invoices (id) {
        id -> Text,
        school -> Text,
        year -> Integer,
        term -> SmallInt,
        fee -> Nullable<Text>,
        description -> Nullable<Text>,
        student -> Integer,
        amount -> Float,
        status -> SmallInt,
        due -> Nullable<BigInt>,
        created -> BigInt,
        updated -> BigInt,
    }
}

diesel::table! {
    lessons (school, year, term, grade, stream, date, subject, teacher) {
        school -> Text,
        year -> Integer,
        term -> SmallInt,
        grade -> SmallInt,
        stream -> SmallInt,
        date -> Integer,
        subject -> SmallInt,
        teacher -> Text,
        created -> BigInt,
        updated -> BigInt,
    }
}

diesel::table! {
    mastery (school, student, grade, subject, topic) {
        school -> Text,
        student -> Integer,
        grade -> SmallInt,
        subject -> SmallInt,
        topic -> SmallInt,
        score -> Float,
        created -> BigInt,
        updated -> BigInt,
    }
}

diesel::table! {
    owners (school, user) {
        school -> Text,
        user -> Text,
        created -> BigInt,
    }
}

diesel::table! {
    papers (school, exam, subject, paper) {
        school -> Text,
        exam -> Text,
        subject -> SmallInt,
        paper -> Nullable<SmallInt>,
        invigilator -> Text,
        start -> BigInt,
        end -> BigInt,
        status -> SmallInt,
        created -> BigInt,
        updated -> BigInt,
    }
}

diesel::table! {
    payments (id) {
        id -> Text,
        invoice -> Nullable<Text>,
        school -> Nullable<Text>,
        student -> Nullable<Integer>,
        amount -> Float,
        method -> SmallInt,
        reference -> Nullable<Text>,
        recorder -> Nullable<Text>,
        date -> Nullable<Integer>,
        created -> BigInt,
        updated -> BigInt,
    }
}

diesel::table! {
    plans (id) {
        id -> Text,
        name -> Text,
        description -> Nullable<Text>,
        amount -> Float,
        levels -> Integer,
        status -> SmallInt,
        features -> Nullable<Text>,
        created -> BigInt,
        updated -> BigInt,
    }
}

diesel::table! {
    roles (id) {
        id -> Text,
        school -> Nullable<Text>,
        name -> Text,
        description -> Nullable<Text>,
        permissions -> Text,
        created -> BigInt,
        updated -> BigInt,
    }
}

diesel::table! {
    schools (id) {
        id -> Text,
        name -> Text,
        motto -> Nullable<Text>,
        phone -> Nullable<Text>,
        email -> Nullable<Text>,
        county -> Integer,
        domain -> Nullable<Text>,
        established -> Nullable<Integer>,
        status -> SmallInt,
        created -> BigInt,
        updated -> BigInt,
    }
}

diesel::table! {
    scopes (school, user, role) {
        school -> Nullable<Text>,
        user -> Text,
        role -> Text,
        created -> BigInt,
    }
}

diesel::table! {
    settings (school) {
        school -> Text,
        data -> Text,
        mpesa -> Nullable<Text>,
        created -> BigInt,
        updated -> BigInt,
    }
}

diesel::table! {
    staff (school, user) {
        school -> Text,
        user -> Text,
        idnumber -> Nullable<Text>,
        role -> Nullable<Text>,
        department -> Nullable<Text>,
        status -> SmallInt,
        created -> BigInt,
        updated -> BigInt,
    }
}

diesel::table! {
    students (school, adm) {
        school -> Text,
        adm -> Integer,
        user -> Nullable<Text>,
        name -> Text,
        dob -> Nullable<Integer>,
        gender -> Nullable<SmallInt>,
        documents -> Nullable<Text>,
        admitted -> Nullable<Integer>,
        status -> SmallInt,
        created -> BigInt,
        updated -> BigInt,
    }
}

diesel::table! {
    subjects (school, year, term, grade, stream, subject) {
        school -> Text,
        year -> Integer,
        term -> SmallInt,
        grade -> SmallInt,
        stream -> SmallInt,
        subject -> SmallInt,
        teacher -> Text,
        created -> BigInt,
    }
}

diesel::table! {
    subscriptions (school, plan, year, term, student) {
        school -> Text,
        plan -> Text,
        year -> Integer,
        term -> SmallInt,
        student -> Integer,
        invoice -> Nullable<Text>,
        discount -> Float,
        status -> SmallInt,
        created -> BigInt,
        updated -> BigInt,
    }
}

diesel::table! {
    teachers (school, user) {
        school -> Text,
        user -> Text,
        hired -> Nullable<Integer>,
        role -> Nullable<Text>,
        department -> Nullable<Text>,
        status -> SmallInt,
        created -> BigInt,
        updated -> BigInt,
    }
}

diesel::table! {
    terms (school, year, term) {
        school -> Text,
        year -> Integer,
        term -> SmallInt,
        start -> BigInt,
        end -> BigInt,
        created -> BigInt,
        updated -> BigInt,
    }
}

diesel::table! {
    timetable (school, year, term, grade, stream, subject, day, start) {
        school -> Text,
        year -> Integer,
        term -> SmallInt,
        grade -> SmallInt,
        stream -> SmallInt,
        subject -> SmallInt,
        teacher -> Text,
        day -> SmallInt,
        start -> Integer,
        end -> Integer,
        created -> BigInt,
        updated -> BigInt,
    }
}

diesel::table! {
    users (id) {
        id -> Text,
        phone -> Text,
        email -> Nullable<Text>,
        name -> Text,
        level -> SmallInt,
        status -> SmallInt,
        created -> BigInt,
        updated -> BigInt,
    }
}

diesel::joinable!(aiusage -> schools (school));
diesel::joinable!(announcements -> schools (school));
diesel::joinable!(announcements -> users (author));
diesel::joinable!(attendance -> schools (school));
diesel::joinable!(class_teachers -> schools (school));
diesel::joinable!(departments -> schools (school));
diesel::joinable!(discounts -> plans (plan));
diesel::joinable!(discounts -> schools (school));
diesel::joinable!(enrollments -> schools (school));
diesel::joinable!(exams -> schools (school));
diesel::joinable!(fees -> schools (school));
diesel::joinable!(grades -> exams (exam));
diesel::joinable!(grades -> schools (school));
diesel::joinable!(guardians -> schools (school));
diesel::joinable!(guardians -> users (user));
diesel::joinable!(invoices -> fees (fee));
diesel::joinable!(invoices -> schools (school));
diesel::joinable!(lessons -> schools (school));
diesel::joinable!(mastery -> schools (school));
diesel::joinable!(owners -> schools (school));
diesel::joinable!(owners -> users (user));
diesel::joinable!(papers -> exams (exam));
diesel::joinable!(papers -> schools (school));
diesel::joinable!(payments -> invoices (invoice));
diesel::joinable!(payments -> schools (school));
diesel::joinable!(payments -> users (recorder));
diesel::joinable!(roles -> schools (school));
diesel::joinable!(scopes -> roles (role));
diesel::joinable!(scopes -> schools (school));
diesel::joinable!(scopes -> users (user));
diesel::joinable!(settings -> schools (school));
diesel::joinable!(staff -> schools (school));
diesel::joinable!(staff -> users (user));
diesel::joinable!(students -> schools (school));
diesel::joinable!(students -> users (user));
diesel::joinable!(subjects -> schools (school));
diesel::joinable!(subscriptions -> invoices (invoice));
diesel::joinable!(subscriptions -> plans (plan));
diesel::joinable!(subscriptions -> schools (school));
diesel::joinable!(teachers -> schools (school));
diesel::joinable!(teachers -> users (user));
diesel::joinable!(terms -> schools (school));
diesel::joinable!(timetable -> schools (school));

diesel::allow_tables_to_appear_in_same_query!(
    aiusage,
    announcements,
    attendance,
    class_teachers,
    departments,
    discounts,
    enrollments,
    exams,
    fees,
    grades,
    guardians,
    invoices,
    lessons,
    mastery,
    owners,
    papers,
    payments,
    plans,
    roles,
    schools,
    scopes,
    settings,
    staff,
    students,
    subjects,
    subscriptions,
    teachers,
    terms,
    timetable,
    users,
);
