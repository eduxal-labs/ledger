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
    ai_token_log (paper, student) {
        paper -> Text,
        student -> Integer,
        input_tokens -> Integer,
        output_tokens -> Integer,
        thinking_tokens -> Integer,
        cached_tokens -> Integer,
        total_tokens -> Integer,
        created -> BigInt,
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
    answer_pages (paper, student, page) {
        paper -> Text,
        student -> Integer,
        page -> SmallInt,
        key -> Text,
        created -> BigInt,
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
    events (id) {
        id -> Text,
        school -> Text,
        name -> Text,
        type_ -> SmallInt,
        term -> SmallInt,
        year -> Integer,
        start_date -> Integer,
        end_date -> Integer,
        status -> SmallInt,
        created -> BigInt,
        updated -> BigInt,
    }
}

diesel::table! {
    exam_coverage (schedule, topic) {
        schedule -> Text,
        topic -> Integer,
        confirmed_by -> Text,
        confirmed_at -> BigInt,
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
    grades (paper, student) {
        paper -> Text,
        student -> Integer,
        score -> Float,
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
        subject -> Integer,
        teacher -> Text,
        created -> BigInt,
        updated -> BigInt,
    }
}

diesel::table! {
    marking_queue (id) {
        id -> Nullable<Integer>,
        paper -> Text,
        phase -> SmallInt,
        progress -> Text,
        error -> Nullable<Text>,
        total_students -> Integer,
        marked_students -> Integer,
        created -> BigInt,
        updated -> BigInt,
    }
}

diesel::table! {
    mastery (school, student, subject, topic) {
        school -> Text,
        student -> Integer,
        subject -> Integer,
        topic -> Integer,
        score -> Float,
        created -> BigInt,
        updated -> BigInt,
    }
}

diesel::table! {
    mpesa (school) {
        school -> Text,
        consumer_key -> Text,
        consumer_secret -> Text,
        passkey -> Text,
        shortcode -> Text,
        env -> SmallInt,
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
    paper_questions (paper, student, question) {
        paper -> Text,
        student -> Nullable<Integer>,
        question -> Integer,
        position -> SmallInt,
        section -> Nullable<Text>,
    }
}

diesel::table! {
    paper_schedules (id) {
        id -> Text,
        event -> Text,
        subject -> Integer,
        grade -> SmallInt,
        stream -> Nullable<SmallInt>,
        date -> Integer,
        start_time -> Integer,
        end_time -> Integer,
        duration_minutes -> SmallInt,
        invigilator -> Nullable<Text>,
        paper -> Nullable<Text>,
        generation_status -> SmallInt,
        reveal_at -> BigInt,
        generate_at -> BigInt,
        created -> BigInt,
    }
}

diesel::table! {
    paper_topics (paper, topic) {
        paper -> Text,
        topic -> Integer,
        weight -> Float,
    }
}

diesel::table! {
    papers (id) {
        id -> Text,
        school -> Text,
        event -> Nullable<Text>,
        subject -> Integer,
        grade -> SmallInt,
        stream -> Nullable<SmallInt>,
        type_ -> SmallInt,
        teacher -> Text,
        name -> Text,
        total_marks -> SmallInt,
        duration_minutes -> SmallInt,
        date -> Integer,
        status -> SmallInt,
        pdf_key -> Nullable<Text>,
        ms_key -> Nullable<Text>,
        generation_mode -> SmallInt,
        instructions -> Nullable<Text>,
        created -> BigInt,
        updated -> BigInt,
    }
}

diesel::table! {
    part_rubric_criteria (question, part, position) {
        question -> Integer,
        part -> SmallInt,
        position -> SmallInt,
        criterion -> Text,
        marks -> SmallInt,
        max_marks -> Nullable<SmallInt>,
        required -> Bool,
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
    question_grades (paper, student, question) {
        paper -> Text,
        student -> Integer,
        question -> Integer,
        score -> Float,
        feedback -> Nullable<Text>,
        awarded_criteria -> Nullable<Text>,
        created -> BigInt,
        updated -> BigInt,
    }
}

diesel::table! {
    question_images (id) {
        id -> Nullable<Integer>,
        question -> Integer,
        position -> SmallInt,
        context -> SmallInt,
        key -> Text,
        caption -> Nullable<Text>,
    }
}

diesel::table! {
    question_parts (question, position) {
        question -> Integer,
        position -> SmallInt,
        label -> Text,
        body -> Text,
        body_format -> SmallInt,
        marks -> SmallInt,
        max_marks -> Nullable<SmallInt>,
        answer_space_type -> SmallInt,
        answer_lines -> Nullable<SmallInt>,
        answer_box_height_mm -> Nullable<SmallInt>,
        example_answer -> Nullable<Text>,
        stimulus -> Nullable<Text>,
    }
}

diesel::table! {
    questions (id) {
        id -> Nullable<Integer>,
        topic -> Integer,
        body -> Text,
        body_format -> SmallInt,
        stimulus -> Nullable<Text>,
        type_ -> SmallInt,
        difficulty -> SmallInt,
        cognitive_level -> SmallInt,
        marks -> SmallInt,
        max_marks -> Nullable<SmallInt>,
        answer_space_type -> SmallInt,
        answer_lines -> Nullable<SmallInt>,
        answer_box_height_mm -> Nullable<SmallInt>,
        example_answer -> Nullable<Text>,
        created -> BigInt,
        updated -> BigInt,
        created_by -> Text,
    }
}

diesel::table! {
    roles (id) {
        id -> Text,
        school -> Nullable<Text>,
        name -> Text,
        description -> Nullable<Text>,
        permissions -> Binary,
        created -> BigInt,
        updated -> BigInt,
    }
}

diesel::table! {
    rubric_criteria (question, position) {
        question -> Integer,
        position -> SmallInt,
        criterion -> Text,
        marks -> SmallInt,
        max_marks -> Nullable<SmallInt>,
        required -> Bool,
    }
}

diesel::table! {
    scheme_pages (paper, page) {
        paper -> Text,
        page -> SmallInt,
        key -> Text,
        created -> BigInt,
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
    streams (school, grade, stream) {
        school -> Text,
        grade -> SmallInt,
        stream -> SmallInt,
        name -> Text,
        created -> BigInt,
        updated -> BigInt,
    }
}

diesel::table! {
    student_pdf_keys (paper, student) {
        paper -> Text,
        student -> Integer,
        pdf_key -> Text,
        generated_at -> BigInt,
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
    subject_teachers (school, year, term, grade, stream, subject) {
        school -> Text,
        year -> Integer,
        term -> SmallInt,
        grade -> SmallInt,
        stream -> SmallInt,
        subject -> Integer,
        teacher -> Text,
        created -> BigInt,
    }
}

diesel::table! {
    subjects (id) {
        id -> Nullable<Integer>,
        name -> Text,
        curriculum -> SmallInt,
        created -> BigInt,
        updated -> BigInt,
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
    taught_topics (school, subject, grade, stream, topic) {
        school -> Text,
        subject -> Integer,
        grade -> SmallInt,
        stream -> Nullable<SmallInt>,
        topic -> Integer,
        taught_by -> Text,
        status -> SmallInt,
        taught_date -> Nullable<Integer>,
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
        subject -> Integer,
        teacher -> Text,
        day -> SmallInt,
        start -> Integer,
        end -> Integer,
        created -> BigInt,
        updated -> BigInt,
    }
}

diesel::table! {
    topics (id) {
        id -> Nullable<Integer>,
        subject -> Integer,
        grade -> SmallInt,
        name -> Text,
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

diesel::joinable!(ai_token_log -> papers (paper));
diesel::joinable!(aiusage -> schools (school));
diesel::joinable!(announcements -> schools (school));
diesel::joinable!(announcements -> users (author));
diesel::joinable!(answer_pages -> papers (paper));
diesel::joinable!(attendance -> schools (school));
diesel::joinable!(class_teachers -> schools (school));
diesel::joinable!(departments -> schools (school));
diesel::joinable!(discounts -> plans (plan));
diesel::joinable!(discounts -> schools (school));
diesel::joinable!(enrollments -> schools (school));
diesel::joinable!(events -> schools (school));
diesel::joinable!(exam_coverage -> paper_schedules (schedule));
diesel::joinable!(exam_coverage -> topics (topic));
diesel::joinable!(exam_coverage -> users (confirmed_by));
diesel::joinable!(fees -> schools (school));
diesel::joinable!(grades -> papers (paper));
diesel::joinable!(guardians -> schools (school));
diesel::joinable!(guardians -> users (user));
diesel::joinable!(invoices -> fees (fee));
diesel::joinable!(invoices -> schools (school));
diesel::joinable!(lessons -> schools (school));
diesel::joinable!(marking_queue -> papers (paper));
diesel::joinable!(mastery -> schools (school));
diesel::joinable!(mastery -> subjects (subject));
diesel::joinable!(mastery -> topics (topic));
diesel::joinable!(mpesa -> schools (school));
diesel::joinable!(owners -> schools (school));
diesel::joinable!(owners -> users (user));
diesel::joinable!(paper_questions -> papers (paper));
diesel::joinable!(paper_questions -> questions (question));
diesel::joinable!(paper_schedules -> events (event));
diesel::joinable!(paper_schedules -> papers (paper));
diesel::joinable!(paper_schedules -> subjects (subject));
diesel::joinable!(paper_schedules -> users (invigilator));
diesel::joinable!(paper_topics -> papers (paper));
diesel::joinable!(paper_topics -> topics (topic));
diesel::joinable!(papers -> events (event));
diesel::joinable!(papers -> schools (school));
diesel::joinable!(papers -> subjects (subject));
diesel::joinable!(payments -> invoices (invoice));
diesel::joinable!(payments -> schools (school));
diesel::joinable!(payments -> users (recorder));
diesel::joinable!(question_grades -> papers (paper));
diesel::joinable!(question_grades -> questions (question));
diesel::joinable!(question_images -> questions (question));
diesel::joinable!(question_parts -> questions (question));
diesel::joinable!(questions -> topics (topic));
diesel::joinable!(questions -> users (created_by));
diesel::joinable!(roles -> schools (school));
diesel::joinable!(rubric_criteria -> questions (question));
diesel::joinable!(scheme_pages -> papers (paper));
diesel::joinable!(scopes -> roles (role));
diesel::joinable!(scopes -> schools (school));
diesel::joinable!(scopes -> users (user));
diesel::joinable!(staff -> schools (school));
diesel::joinable!(staff -> users (user));
diesel::joinable!(streams -> schools (school));
diesel::joinable!(student_pdf_keys -> papers (paper));
diesel::joinable!(students -> schools (school));
diesel::joinable!(students -> users (user));
diesel::joinable!(subject_teachers -> schools (school));
diesel::joinable!(subject_teachers -> subjects (subject));
diesel::joinable!(subscriptions -> invoices (invoice));
diesel::joinable!(subscriptions -> plans (plan));
diesel::joinable!(subscriptions -> schools (school));
diesel::joinable!(taught_topics -> schools (school));
diesel::joinable!(taught_topics -> subjects (subject));
diesel::joinable!(taught_topics -> topics (topic));
diesel::joinable!(teachers -> schools (school));
diesel::joinable!(teachers -> users (user));
diesel::joinable!(terms -> schools (school));
diesel::joinable!(timetable -> schools (school));
diesel::joinable!(topics -> subjects (subject));

diesel::allow_tables_to_appear_in_same_query!(
    ai_token_log,
    aiusage,
    announcements,
    answer_pages,
    attendance,
    class_teachers,
    departments,
    discounts,
    enrollments,
    events,
    exam_coverage,
    fees,
    grades,
    guardians,
    invoices,
    lessons,
    marking_queue,
    mastery,
    mpesa,
    owners,
    paper_questions,
    paper_schedules,
    paper_topics,
    papers,
    part_rubric_criteria,
    payments,
    plans,
    question_grades,
    question_images,
    question_parts,
    questions,
    roles,
    rubric_criteria,
    scheme_pages,
    schools,
    scopes,
    staff,
    streams,
    student_pdf_keys,
    students,
    subject_teachers,
    subjects,
    subscriptions,
    taught_topics,
    teachers,
    terms,
    timetable,
    topics,
    users,
);
