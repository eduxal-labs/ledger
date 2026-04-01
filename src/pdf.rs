use printpdf::*;

/// Generate an exam paper PDF from question data.
///
/// Returns the PDF as a byte vector.
///
/// # Arguments
/// * `school_name` — Name of the school (bold header)
/// * `school_motto` — Optional school motto (italic, below name)
/// * `exam_name` — Name of the exam
/// * `subject_name` — Subject name
/// * `paper_number` — Optional paper number
/// * `grade` — Grade/form level
/// * `questions` — Slice of (question_text, marks, rubric_criteria) tuples
///   where rubric_criteria is a vec of (criterion_text, marks)
pub fn generate_paper_pdf(
    school_name: &str,
    school_motto: Option<&str>,
    exam_name: &str,
    subject_name: &str,
    paper_number: Option<i16>,
    grade: i16,
    questions: &[(String, i16, Vec<(String, i16)>)],
) -> Result<Vec<u8>, String> {
    // A4 dimensions
    let page_width = Mm(210.0);
    let page_height = Mm(297.0);

    // Margins and layout constants (in mm, converted to Pt for ops)
    let left_margin_mm: f32 = 20.0;
    let right_margin_mm: f32 = 190.0;
    let top_start_mm: f32 = 277.0; // start y from bottom (A4 = 297mm, 20mm top margin)
    let bottom_margin_mm: f32 = 25.0;
    let line_height_mm: f32 = 5.5;
    let paragraph_spacing_mm: f32 = 3.0;

    // Approximate characters per line at 10pt Helvetica on A4 with 20mm margins
    let max_chars_per_line: usize = 90;

    // Font handles
    let font_regular = PdfFontHandle::Builtin(BuiltinFont::Helvetica);
    let font_bold = PdfFontHandle::Builtin(BuiltinFont::HelveticaBold);
    let font_italic = PdfFontHandle::Builtin(BuiltinFont::HelveticaOblique);

    // We'll collect all pages; build ops for each page
    let mut pages: Vec<PdfPage> = Vec::new();
    let mut ops: Vec<Op> = Vec::new();
    let mut y = top_start_mm;

    // Helper: flush current ops into a page and start fresh
    let flush_page = |ops: &mut Vec<Op>, pages: &mut Vec<PdfPage>| {
        // End any open text section
        ops.push(Op::EndTextSection);
        let page = PdfPage::new(page_width, page_height, std::mem::take(ops));
        pages.push(page);
    };

    // Helper: create a LinePoint from mm coordinates
    let lp = |x_mm: f32, y_mm: f32| -> LinePoint {
        LinePoint {
            p: Point::new(Mm(x_mm), Mm(y_mm)),
            bezier: false,
        }
    };

    // Start first page text section
    ops.push(Op::StartTextSection);

    // --- HEADER: School name (bold, centered, 16pt) ---
    let school_name_size = Pt(16.0);
    ops.push(Op::SetFont {
        font: font_bold.clone(),
        size: school_name_size,
    });
    // Approximate centering: A4 is 210mm, rough char width at 16pt ~4mm
    let school_x = center_x(school_name, 16.0, left_margin_mm, right_margin_mm);
    ops.push(Op::SetTextCursor {
        pos: Point::new(Mm(school_x), Mm(y)),
    });
    ops.push(Op::ShowText {
        items: vec![TextItem::Text(school_name.to_string())],
    });
    y -= 8.0;

    // --- MOTTO (italic, centered, 10pt) if present ---
    if let Some(motto) = school_motto {
        if !motto.is_empty() {
            ops.push(Op::EndTextSection);
            ops.push(Op::StartTextSection);
            ops.push(Op::SetFont {
                font: font_italic.clone(),
                size: Pt(10.0),
            });
            let motto_x = center_x(motto, 10.0, left_margin_mm, right_margin_mm);
            ops.push(Op::SetTextCursor {
                pos: Point::new(Mm(motto_x), Mm(y)),
            });
            ops.push(Op::ShowText {
                items: vec![TextItem::Text(motto.to_string())],
            });
            y -= 7.0;
        }
    }

    // --- Divider line (drawn as a thin line) ---
    ops.push(Op::EndTextSection);
    ops.push(Op::SetOutlineColor {
        col: Color::Rgb(Rgb {
            r: 0.3,
            g: 0.3,
            b: 0.3,
            icc_profile: None,
        }),
    });
    ops.push(Op::SetOutlineThickness { pt: Pt(0.5) });
    ops.push(Op::DrawLine {
        line: Line {
            points: vec![lp(left_margin_mm, y), lp(right_margin_mm, y)],
            is_closed: false,
        },
    });
    y -= 6.0;

    // --- Sub-header: Exam | Subject | Paper N | Form ---
    ops.push(Op::StartTextSection);
    ops.push(Op::SetFont {
        font: font_bold.clone(),
        size: Pt(11.0),
    });
    let paper_str = paper_number
        .map(|p| format!("  |  Paper {}", p))
        .unwrap_or_default();
    let sub_header = format!(
        "{}  |  {}{}  |  Form {}",
        exam_name, subject_name, paper_str, grade
    );
    ops.push(Op::SetTextCursor {
        pos: Point::new(Mm(left_margin_mm), Mm(y)),
    });
    ops.push(Op::ShowText {
        items: vec![TextItem::Text(sub_header)],
    });
    y -= 8.0;

    // --- Instructions ---
    ops.push(Op::EndTextSection);
    ops.push(Op::StartTextSection);
    ops.push(Op::SetFont {
        font: font_italic.clone(),
        size: Pt(10.0),
    });
    ops.push(Op::SetTextCursor {
        pos: Point::new(Mm(left_margin_mm), Mm(y)),
    });
    ops.push(Op::ShowText {
        items: vec![TextItem::Text("Answer ALL questions.".to_string())],
    });
    y -= 10.0;

    // --- Horizontal rule below instructions ---
    ops.push(Op::EndTextSection);
    ops.push(Op::DrawLine {
        line: Line {
            points: vec![lp(left_margin_mm, y + 3.0), lp(right_margin_mm, y + 3.0)],
            is_closed: false,
        },
    });
    ops.push(Op::StartTextSection);

    // --- QUESTIONS ---
    let total_marks: i16 = questions.iter().map(|(_, m, _)| m).sum();

    for (i, (text, marks, _rubric)) in questions.iter().enumerate() {
        // Check if we need a new page
        if y < bottom_margin_mm {
            flush_page(&mut ops, &mut pages);
            ops.push(Op::StartTextSection);
            y = top_start_mm;
        }

        // Question header: "1. (X marks)"
        let q_header = format!("{}.", i + 1);
        let q_marks = format!("({} mark{})", marks, if *marks == 1 { "" } else { "s" });

        // Set bold for question number
        ops.push(Op::EndTextSection);
        ops.push(Op::StartTextSection);
        ops.push(Op::SetFont {
            font: font_bold.clone(),
            size: Pt(10.0),
        });
        ops.push(Op::SetTextCursor {
            pos: Point::new(Mm(left_margin_mm), Mm(y)),
        });
        ops.push(Op::ShowText {
            items: vec![TextItem::Text(q_header)],
        });

        // Question text in regular font, indented
        let indent_mm = left_margin_mm + 10.0;
        let text_with_marks = format!("{} {}", text, q_marks);
        let available_chars = ((right_margin_mm - indent_mm) / (right_margin_mm - left_margin_mm)
            * max_chars_per_line as f32) as usize;
        let lines = word_wrap(&text_with_marks, available_chars.max(40));

        for (li, line) in lines.iter().enumerate() {
            if y < bottom_margin_mm {
                flush_page(&mut ops, &mut pages);
                ops.push(Op::StartTextSection);
                y = top_start_mm;
            }

            if li == 0 {
                // First line: position next to question number
                ops.push(Op::EndTextSection);
                ops.push(Op::StartTextSection);
                ops.push(Op::SetFont {
                    font: font_regular.clone(),
                    size: Pt(10.0),
                });
                ops.push(Op::SetTextCursor {
                    pos: Point::new(Mm(indent_mm), Mm(y)),
                });
            } else {
                // Continuation lines
                ops.push(Op::EndTextSection);
                ops.push(Op::StartTextSection);
                ops.push(Op::SetFont {
                    font: font_regular.clone(),
                    size: Pt(10.0),
                });
                ops.push(Op::SetTextCursor {
                    pos: Point::new(Mm(indent_mm), Mm(y)),
                });
            }

            ops.push(Op::ShowText {
                items: vec![TextItem::Text(line.clone())],
            });
            y -= line_height_mm;
        }

        // Extra spacing between questions
        y -= paragraph_spacing_mm;
    }

    // --- Footer: Total marks ---
    if y < bottom_margin_mm + 10.0 {
        flush_page(&mut ops, &mut pages);
        ops.push(Op::StartTextSection);
        y = top_start_mm;
    }

    y -= 2.0;
    ops.push(Op::EndTextSection);
    ops.push(Op::DrawLine {
        line: Line {
            points: vec![lp(left_margin_mm, y + 2.0), lp(right_margin_mm, y + 2.0)],
            is_closed: false,
        },
    });
    y -= 6.0;

    ops.push(Op::StartTextSection);
    ops.push(Op::SetFont {
        font: font_bold.clone(),
        size: Pt(11.0),
    });
    let total_str = format!("Total: {} marks", total_marks);
    let total_x = right_margin_mm - (total_str.len() as f32 * 2.5);
    ops.push(Op::SetTextCursor {
        pos: Point::new(Mm(total_x.max(left_margin_mm)), Mm(y)),
    });
    ops.push(Op::ShowText {
        items: vec![TextItem::Text(total_str)],
    });

    // Flush the last page
    flush_page(&mut ops, &mut pages);

    // Build the document
    let mut doc = PdfDocument::new("Exam Paper");
    doc.metadata.info.document_title = format!("{} - {} Paper", exam_name, subject_name);
    doc.metadata.info.creator = "EduxaL Ledger".to_string();
    doc.metadata.info.producer = "printpdf".to_string();
    doc.pages = pages;

    // Serialize to bytes
    let mut warnings = Vec::new();
    let opts = PdfSaveOptions::default();
    let bytes = doc.save(&opts, &mut warnings);

    if !warnings.is_empty() {
        tracing::warn!("PDF generation warnings: {:?}", warnings);
    }

    Ok(bytes)
}

/// Approximate horizontal centering for a text string.
///
/// Estimates the text width using an average character width ratio
/// and returns the x position (in mm) to start the text so it appears centered.
fn center_x(text: &str, font_size_pt: f32, left_mm: f32, right_mm: f32) -> f32 {
    let page_center = (left_mm + right_mm) / 2.0;
    // Approximate character width: ~0.5 * font_size in pt, converted to mm (1pt = 0.3528mm)
    let avg_char_width_mm = 0.5 * font_size_pt * 0.3528;
    let text_width = text.len() as f32 * avg_char_width_mm;
    let x = page_center - (text_width / 2.0);
    x.max(left_mm)
}

/// Simple word wrap: split text into lines of at most `max` characters,
/// breaking at whitespace boundaries.
fn word_wrap(text: &str, max: usize) -> Vec<String> {
    let mut lines = Vec::new();
    let mut current = String::new();

    for word in text.split_whitespace() {
        if current.is_empty() {
            current = word.to_string();
        } else if current.len() + 1 + word.len() > max {
            lines.push(current);
            current = word.to_string();
        } else {
            current.push(' ');
            current.push_str(word);
        }
    }
    if !current.is_empty() {
        lines.push(current);
    }

    if lines.is_empty() {
        lines.push(String::new());
    }

    lines
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_word_wrap_basic() {
        let lines = word_wrap("hello world foo bar", 10);
        assert_eq!(lines, vec!["hello", "world foo", "bar"]);
    }

    #[test]
    fn test_word_wrap_long_word() {
        let lines = word_wrap("superlongword short", 10);
        assert_eq!(lines, vec!["superlongword", "short"]);
    }

    #[test]
    fn test_word_wrap_empty() {
        let lines = word_wrap("", 80);
        assert_eq!(lines, vec![""]);
    }

    #[test]
    fn test_word_wrap_single_line() {
        let lines = word_wrap("hello world", 80);
        assert_eq!(lines, vec!["hello world"]);
    }

    #[test]
    fn test_center_x_returns_valid_position() {
        let x = center_x("Test Title", 16.0, 20.0, 190.0);
        assert!(x >= 20.0);
        assert!(x <= 190.0);
    }

    #[test]
    fn test_generate_paper_pdf_basic() {
        let questions = vec![
            (
                "What is 2 + 2?".to_string(),
                2_i16,
                vec![("Correct answer: 4".to_string(), 2_i16)],
            ),
            (
                "Explain the water cycle.".to_string(),
                5_i16,
                vec![
                    ("Evaporation".to_string(), 2_i16),
                    ("Condensation and precipitation".to_string(), 3_i16),
                ],
            ),
        ];

        let result = generate_paper_pdf(
            "Moi High School",
            Some("Excellence in Education"),
            "End of Term 1 Exam 2025",
            "Mathematics",
            Some(1),
            10,
            &questions,
        );

        assert!(result.is_ok());
        let bytes = result.unwrap();
        // A valid PDF starts with %PDF
        assert!(bytes.len() > 100);
        assert_eq!(&bytes[0..5], b"%PDF-");
    }

    #[test]
    fn test_generate_paper_pdf_no_motto() {
        let questions = vec![("Define osmosis.".to_string(), 3_i16, vec![])];

        let result =
            generate_paper_pdf("Test School", None, "CAT 1", "Biology", None, 9, &questions);

        assert!(result.is_ok());
        let bytes = result.unwrap();
        assert_eq!(&bytes[0..5], b"%PDF-");
    }

    #[test]
    fn test_generate_paper_pdf_many_questions_multi_page() {
        // Generate enough questions to force multiple pages
        let questions: Vec<(String, i16, Vec<(String, i16)>)> = (0..60)
            .map(|i| {
                (
                    format!(
                        "This is question number {} which has a reasonably long text to take up space on the page and help trigger page breaks during PDF generation.",
                        i + 1
                    ),
                    3_i16,
                    vec![("criterion".to_string(), 3_i16)],
                )
            })
            .collect();

        let result = generate_paper_pdf(
            "Multi-Page School",
            Some("Testing pagination"),
            "Final Exam",
            "History",
            Some(2),
            12,
            &questions,
        );

        assert!(result.is_ok());
        let bytes = result.unwrap();
        assert_eq!(&bytes[0..5], b"%PDF-");
        // Multi-page PDF should be larger
        assert!(bytes.len() > 1000);
    }
}
