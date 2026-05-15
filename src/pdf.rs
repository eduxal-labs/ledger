//! PDF generation using the Typst typesetting engine.

use crate::types::error::Result;
use crate::types::question::{ExampleAnswer, ExampleAnswerFormat, Stimulus, StimulusType};

// ── Public types ─────────────────────────────────────────────────────────────

pub struct PaperPdfInput<'a> {
    pub school_name: &'a str,
    pub school_motto: Option<&'a str>,
    pub paper_name: &'a str,
    pub subject_name: &'a str,
    pub paper_number: Option<i16>,
    pub grade: i16,
    pub duration_minutes: Option<i16>,
    pub instructions: Option<&'a str>,
    pub questions: &'a [PaperQuestion],
}

#[derive(Clone)]
pub struct PaperQuestion {
    pub body: String,
    pub body_format: u8,
    pub marks: i16,
    pub max_marks: Option<i16>,
    pub answer_space_type: u8,
    pub answer_lines: Option<i16>,
    pub answer_box_height_mm: Option<i16>,
    pub stimulus: Option<String>,
    pub example_answer: Option<String>,
    pub rubric: Vec<(String, i16, bool)>, // (criterion, marks, required)
    pub parts: Vec<PaperPart>,
    pub section: Option<String>,
}

#[derive(Clone)]
pub struct PaperPart {
    pub label: String,
    pub body: String,
    pub body_format: u8,
    pub marks: i16,
    pub answer_space_type: u8,
    pub answer_lines: Option<i16>,
    pub answer_box_height_mm: Option<i16>,
    pub stimulus: Option<String>,
    pub rubric: Vec<(String, i16, bool)>,
}

// ── Typst World implementation ───────────────────────────────────────────────

use typst::Library;
use typst::diag::FileError;
use typst::foundations::{Bytes, Datetime};
use typst::syntax::{FileId, Source, VirtualPath};
use typst::text::{Font, FontBook};
use typst::utils::LazyHash;

struct SingleDocWorld {
    source: String,
    library: LazyHash<Library>,
    book: LazyHash<FontBook>,
    fonts: Vec<Font>,
    main_id: FileId,
}

impl SingleDocWorld {
    fn new(source: String) -> Self {
        let fonts: Vec<Font> = typst_assets::fonts()
            .flat_map(|data| Font::iter(Bytes::new(data)))
            .collect();
        let book = FontBook::from_fonts(fonts.iter());
        Self {
            library: LazyHash::new(Library::default()),
            book: LazyHash::new(book),
            fonts,
            main_id: FileId::new(None, VirtualPath::new("main.typ")),
            source,
        }
    }
}

impl typst::World for SingleDocWorld {
    fn library(&self) -> &LazyHash<Library> {
        &self.library
    }
    fn book(&self) -> &LazyHash<FontBook> {
        &self.book
    }
    fn main(&self) -> FileId {
        self.main_id
    }

    fn source(&self, id: FileId) -> std::result::Result<Source, FileError> {
        if id == self.main_id {
            Ok(Source::new(id, self.source.clone()))
        } else {
            Err(FileError::NotFound(std::path::PathBuf::from("<virtual>")))
        }
    }

    fn file(&self, _id: FileId) -> std::result::Result<Bytes, FileError> {
        Err(FileError::NotFound(std::path::PathBuf::from("<virtual>")))
    }

    fn font(&self, index: usize) -> Option<Font> {
        self.fonts.get(index).cloned()
    }

    fn today(&self, _offset: Option<i64>) -> Option<Datetime> {
        None
    }
}

fn compile_typst(source: String) -> Result<Vec<u8>> {
    let world = SingleDocWorld::new(source.clone());
    let document: typst::layout::PagedDocument = typst::compile(&world).output.map_err(|errs| {
        let msgs: Vec<String> = errs.iter().map(|e| e.message.to_string()).collect();
        let msg = msgs.join("; ");
        tracing::error!("Typst compilation failed: {msg}\n--- Typst source ---\n{source}\n--- end source ---");
        crate::types::error::Error::internal(msg)
    })?;
    let options = typst_pdf::PdfOptions::default();
    typst_pdf::pdf(&document, &options).map_err(|errs| {
        let msgs: Vec<String> = errs.iter().map(|e| e.message.to_string()).collect();
        crate::types::error::Error::internal(msgs.join("; "))
    })
}

// ── Template builders ─────────────────────────────────────────────────────────

/// Build the Typst source for an exam paper.
pub fn build_exam_paper_typst(input: &PaperPdfInput) -> String {
    let mut doc = String::new();

    // Page setup
    doc.push_str("#set page(paper: \"a4\", margin: (top: 20mm, bottom: 25mm, left: 20mm, right: 20mm), numbering: \"1\")\n");
    doc.push_str("#set text(font: \"New Computer Modern\", size: 9pt)\n");
    doc.push_str("#set par(justify: true)\n\n");

    // Header — clean centred layout with horizontal rules
    doc.push_str(&format!(
        "#align(center, text(weight: \"bold\", size: 14pt)[{}])\n",
        escape_typst(input.school_name)
    ));
    if let Some(motto) = input.school_motto
        .filter(|m| !m.is_empty() && *m != "unknown" && *m != "null" && *m != "none")
    {
        doc.push_str(&format!(
            "#align(center, text(style: \"italic\", size: 10pt)[{}])\n",
            escape_typst(motto)
        ));
    }
    doc.push_str("#v(2mm)\n");
    doc.push_str("#align(center, line(length: 40%, stroke: 0.5pt))\n");
    doc.push_str("#v(2mm)\n");
    doc.push_str(&format!(
        "#align(center, text(weight: \"bold\", size: 13pt)[{}])\n",
        escape_typst(input.paper_name)
    ));
    let paper_num_str = input
        .paper_number
        .map(|n| format!("Paper {}", n))
        .unwrap_or_default();
    let grade_label = format_grade_display(input.grade);
    let subject_line = if paper_num_str.is_empty() {
        format!("{} — {}", escape_typst(input.subject_name), grade_label)
    } else {
        format!(
            "{} — {} — {}",
            escape_typst(input.subject_name),
            grade_label,
            paper_num_str
        )
    };
    doc.push_str(&format!("#align(center)[{}]\n", subject_line));
    if let Some(dur) = input.duration_minutes {
        doc.push_str(&format!(
            "#align(center, text(size: 10pt)[Time Allowed: {} minutes])\n",
            dur
        ));
    }
    doc.push_str("#v(2mm)\n");
    doc.push_str("#line(length: 100%, stroke: 1.5pt)\n");

    // Instructions
    if let Some(instrs) = input.instructions {
        doc.push_str("#block(stroke: 1pt, inset: 8pt, width: 100%)[\n");
        doc.push_str("  *Instructions:* \\\n");
        doc.push_str(&format!("  {}\n", escape_typst(instrs)));
        doc.push_str("]\n");
    }
    doc.push_str("#v(5mm)\n\n");

    // Questions
    let mut current_section: Option<String> = None;
    for (i, q) in input.questions.iter().enumerate() {
        // Section header — always update current_section so None resets correctly
        if q.section != current_section {
            if let Some(ref s) = q.section {
                doc.push_str(&format!(
                    "\n#align(center)[*SECTION {}*]\n\n",
                    escape_typst(s)
                ));
            }
            current_section = q.section.clone();
        }

        doc.push_str(&format!(
            "*{}. * {} #h(1fr) [*{} mark{}*]\n\n",
            i + 1,
            escape_typst(&render_body(&q.body, q.body_format)),
            q.marks,
            if q.marks != 1 { "s" } else { "" }
        ));

        // Stimulus
        if let Some(ref stim) = q.stimulus {
            render_stimulus(&mut doc, stim, "");
        }

        // Parts
        for part in &q.parts {
            // Part stimulus
            if let Some(ref stim) = part.stimulus {
                render_stimulus(&mut doc, stim, "  ");
            }
            doc.push_str(&format!(
                "  *({})* {} #h(1fr) [*{} mark{}*]\n\n",
                part.label,
                escape_typst(&render_body(&part.body, part.body_format)),
                part.marks,
                if part.marks != 1 { "s" } else { "" }
            ));
            render_answer_space(
                &mut doc,
                part.answer_space_type,
                part.answer_lines,
                part.answer_box_height_mm,
            );
        }

        // Answer space (if no parts)
        if q.parts.is_empty() {
            render_answer_space(
                &mut doc,
                q.answer_space_type,
                q.answer_lines,
                q.answer_box_height_mm,
            );
        }
    }

    doc
}

fn render_answer_space(
    doc: &mut String,
    space_type: u8,
    lines: Option<i16>,
    height_mm: Option<i16>,
) {
    match space_type {
        0 => {
            // Lines — add space before the first line so it doesn't sit on
            // the question text, and use tighter inter-line spacing.
            let n = lines.unwrap_or(4);
            doc.push_str("#v(3mm)\n");
            for _ in 0..n {
                doc.push_str("#line(length: 100%)\n#v(4mm)\n");
            }
        }
        1 | 2 | 3 | 4 => {
            // Boxes
            let h = height_mm.unwrap_or(40);
            let label = match space_type {
                3 => " [Construction space]",
                4 => " [Grid]",
                _ => "",
            };
            doc.push_str(&format!("#rect(width: 100%, height: {}mm)[{}]\n", h, label));
        }
        _ => {}
    }
    doc.push_str("#v(3mm)\n");
}

/// Build the Typst source for a marking scheme.
pub fn build_marking_scheme_typst(input: &PaperPdfInput) -> String {
    let mut doc = String::new();

    doc.push_str("#set page(paper: \"a4\", margin: (top: 20mm, bottom: 25mm, left: 20mm, right: 20mm), numbering: \"1\")\n");
    doc.push_str("#set text(font: \"New Computer Modern\", size: 9pt)\n");
    doc.push_str("#set par(justify: true)\n\n");

    // Header with MARKING SCHEME title
    doc.push_str(&format!(
        "#align(center, text(weight: \"bold\", size: 14pt)[{}])\n",
        escape_typst(input.school_name)
    ));
    doc.push_str("#v(2mm)\n");
    doc.push_str("#align(center, line(length: 40%, stroke: 0.5pt))\n");
    doc.push_str("#v(2mm)\n");
    doc.push_str(&format!(
        "#align(center, text(weight: \"bold\", size: 13pt)[MARKING SCHEME — {}])\n",
        escape_typst(input.paper_name)
    ));
    doc.push_str(&format!(
        "#align(center)[{} — {}]\n",
        escape_typst(input.subject_name),
        format_grade_display(input.grade)
    ));
    doc.push_str("#v(2mm)\n");
    doc.push_str("#line(length: 100%, stroke: 1.5pt)\n#v(5mm)\n\n");

    // Questions with rubric
    for (i, q) in input.questions.iter().enumerate() {
        doc.push_str(&format!(
            "*{}. * {} #h(1fr) [*{} mark{}*]\n\n",
            i + 1,
            escape_typst(&render_body(&q.body, q.body_format)),
            q.marks,
            if q.marks != 1 { "s" } else { "" }
        ));

        // Stimulus
        if let Some(ref stim) = q.stimulus {
            render_stimulus(&mut doc, stim, "");
        }

        if let Some(cap) = q.max_marks {
            doc.push_str(&format!("_(Award any {} of the following)_\n\n", cap));
        }

        for (j, (criterion, marks, required)) in q.rubric.iter().enumerate() {
            let req_mark = if *required { "\\* " } else { "" };
            doc.push_str(&format!(
                "{}{}. {} ... [{} mark{}]\n",
                req_mark,
                j + 1,
                escape_typst(criterion),
                marks,
                if *marks != 1 { "s" } else { "" }
            ));
        }

        if !q.rubric.is_empty() {
            doc.push_str("\n");
        }

        // Example answer
        if let Some(ref ea) = q.example_answer {
            render_example_answer(&mut doc, ea, "");
        }

        // Parts
        for part in &q.parts {
            // Part stimulus
            if let Some(ref stim) = part.stimulus {
                render_stimulus(&mut doc, stim, "  ");
            }
            doc.push_str(&format!(
                "  *({})* {}\n",
                part.label,
                escape_typst(&render_body(&part.body, part.body_format))
            ));
            for (j, (criterion, marks, required)) in part.rubric.iter().enumerate() {
                let req_mark = if *required { "\\* " } else { "" };
                doc.push_str(&format!(
                    "  {}{}. {} ... [{} mark{}]\n",
                    req_mark,
                    j + 1,
                    escape_typst(criterion),
                    marks,
                    if *marks != 1 { "s" } else { "" }
                ));
            }
            doc.push_str("\n");
        }
    }

    doc
}

/// Build per-student named exam paper.
pub fn build_student_exam_paper_typst(
    input: &PaperPdfInput,
    student_name: &str,
    student_adm: i32,
) -> String {
    // Build the base exam paper
    let mut base = build_exam_paper_typst(input);

    // Insert student header block just before the first question (#v(5mm)\n\n section)
    let student_block = format!(
        "#block(stroke: 0.5pt, inset: 6pt)[Name: #underline[{}] #h(1fr) Adm No: #underline[{}]]\n#v(3mm)\n",
        escape_typst(student_name),
        student_adm
    );

    // Insert after the instructions block (before the first question)
    let marker = "#v(5mm)\n\n";
    if let Some(pos) = base.find(marker) {
        let insert_at = pos + marker.len();
        base.insert_str(insert_at, &student_block);
    } else {
        // Fallback: append before any content after the header area.
        // This should not happen, but if it does the student info is
        // still included rather than silently dropped.
        tracing::warn!(
            "build_student_exam_paper_typst: marker not found, appending student block at end"
        );
        base.push_str(&student_block);
    }

    base
}

// ── Public API ───────────────────────────────────────────────────────────────

/// Generate an exam paper PDF.
pub fn generate_paper_pdf_typst(input: &PaperPdfInput) -> std::result::Result<Vec<u8>, String> {
    let source = build_exam_paper_typst(input);
    compile_typst(source).map_err(|e| e.to_string())
}

/// Generate a marking scheme PDF.
pub fn generate_marking_scheme_pdf_typst(
    input: &PaperPdfInput,
) -> std::result::Result<Vec<u8>, String> {
    let source = build_marking_scheme_typst(input);
    compile_typst(source).map_err(|e| e.to_string())
}

/// Generate a named student paper PDF (with student name/adm pre-filled).
pub fn generate_student_paper_pdf(
    input: &PaperPdfInput,
    student_name: &str,
    student_adm: i32,
) -> std::result::Result<Vec<u8>, String> {
    let source = build_student_exam_paper_typst(input, student_name, student_adm);
    compile_typst(source).map_err(|e| e.to_string())
}

// ── Helpers ──────────────────────────────────────────────────────────────────

/// Render a stimulus JSON string as a Typst block, with type label, caption, and image info.
fn render_stimulus(doc: &mut String, stimulus_json: &str, indent: &str) {
    match serde_json::from_str::<Stimulus>(stimulus_json) {
        Ok(stim) => {
            let body_text = escape_typst(&stim.body);
            let label = match stim.type_ {
                StimulusType::Passage => "",
                StimulusType::Table => "[Table] ",
                StimulusType::Graph => "[Graph] ",
                StimulusType::Diagram => "[Diagram] ",
            };
            doc.push_str(&format!(
                "{indent}#block(fill: luma(240), stroke: 0.5pt, inset: 6pt, width: 100%)[{label}{body_text}",
            ));
            if !stim.caption.is_empty() {
                doc.push_str(&format!(
                    " \\\\ #text(size: 9pt, style: \"italic\")[{}]",
                    escape_typst(&stim.caption)
                ));
            }
            if let Some(img) = &stim.image {
                doc.push_str(&format!(
                    " \\\\ #text(size: 9pt)[Image: {}]",
                    escape_typst(&img.filename)
                ));
            }
            doc.push_str("]\n\n");
        }
        Err(_) => {
            // Legacy: plain string fallback
            doc.push_str(&format!(
                "{indent}#block(fill: luma(240), stroke: 0.5pt, inset: 6pt, width: 100%)[{}]\n\n",
                escape_typst(stimulus_json)
            ));
        }
    }
}

/// Render an example_answer JSON string as a Typst indented block.
fn render_example_answer(doc: &mut String, example_answer_json: &str, indent: &str) {
    match serde_json::from_str::<ExampleAnswer>(example_answer_json) {
        Ok(ea) => match ea.format {
            ExampleAnswerFormat::Plain | ExampleAnswerFormat::Tiptap => {
                if let Some(content) = &ea.content {
                    let text = strip_html(content);
                    doc.push_str(&format!(
                        "{indent}#block(inset: (left: 8pt))[_Example answer:_ {}]\n\n",
                        escape_typst(&text)
                    ));
                }
            }
            ExampleAnswerFormat::Svg => {
                if let Some(content) = &ea.content {
                    doc.push_str(&format!(
                        "{indent}#block(inset: (left: 8pt))[#image.decode(\"{}\", format: \"svg\")]\n\n",
                        escape_typst(content)
                    ));
                }
            }
            ExampleAnswerFormat::Image => {
                let filename = ea
                    .image
                    .as_ref()
                    .map(|i| i.filename.as_str())
                    .unwrap_or("image");
                doc.push_str(&format!(
                    "{indent}#block(inset: (left: 8pt))[_Example answer:_ [See image: {}]]\n\n",
                    escape_typst(filename)
                ));
            }
        },
        Err(_) => {
            // Legacy: plain string
            doc.push_str(&format!(
                "{indent}#block(inset: (left: 8pt))[_Example answer:_ {}]\n\n",
                escape_typst(example_answer_json)
            ));
        }
    }
}

/// Strip basic HTML tags from a string.
/// Return the body text, stripping HTML tags when body_format == 1 (Tiptap).
fn render_body(body: &str, body_format: u8) -> String {
    if body_format == 1 {
        strip_html(body)
    } else {
        body.to_owned()
    }
}

fn strip_html(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    let mut in_tag = false;
    for c in s.chars() {
        match c {
            '<' => in_tag = true,
            '>' => in_tag = false,
            _ if !in_tag => result.push(c),
            _ => {}
        }
    }
    result
}

/// Convert a raw grade integer to a human-readable display label.
///
/// Follows the same mapping as the Flutter client:
/// - CBC:  1=PP1, 2=PP2, 3–14=Grade 1–12
/// - 8-4-4: 1–8=Standard 1–8, 41–44=Form 1–4
fn format_grade_display(grade: i16) -> String {
    match grade {
        // CBC labels
        1 => "PP1".into(),
        2 => "PP2".into(),
        3 => "Grade 1".into(),
        4 => "Grade 2".into(),
        5 => "Grade 3".into(),
        6 => "Grade 4".into(),
        7 => "Grade 5".into(),
        8 => "Grade 6".into(),
        9 => "Grade 7".into(),
        10 => "Grade 8".into(),
        11 => "Grade 9".into(),
        12 => "Grade 10".into(),
        13 => "Grade 11".into(),
        14 => "Grade 12".into(),
        // 8-4-4 labels
        41 => "Form 1".into(),
        42 => "Form 2".into(),
        43 => "Form 3".into(),
        44 => "Form 4".into(),
        // Standard 1–8 (8-4-4 primary) — same numbers as CBC Grade 1–8,
        // but the curriculum context disambiguates. If we reach the match
        // default, we return the raw form as a last resort.
        other => format!("Grade {}", other),
    }
}

/// Escape text for safe inclusion in Typst source.
fn escape_typst(s: &str) -> String {
    s.replace('\\', "\\\\")
        .replace('[', "\\[")
        .replace(']', "\\]")
        .replace('#', "\\#")
        .replace('@', "\\@")
        .replace('*', "\\*")
        .replace('_', "\\_")
        .replace('`', "\\`")
        .replace('<', "\\<")
        .replace('>', "\\>")
}
