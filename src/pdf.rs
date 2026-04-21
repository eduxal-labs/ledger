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
/// * `time_allowed_minutes` — Optional exam duration in minutes (rendered below sub-header)
/// * `custom_instructions` — Optional multi-line instructions string; falls back to 5 default lines
/// * `questions` — Slice of (question_text, marks, rubric_criteria, section) tuples
///   where rubric_criteria is a vec of (criterion_text, marks) and section is an optional
///   section letter string (e.g. "A", "B") used to render section headers
pub fn generate_paper_pdf(
    school_name: &str,
    school_motto: Option<&str>,
    exam_name: &str,
    subject_name: &str,
    paper_number: Option<i16>,
    grade: i16,
    time_allowed_minutes: Option<i16>,
    custom_instructions: Option<&str>,
    questions: &[(String, i16, Vec<(String, i16)>, Option<String>)],
) -> Result<Vec<u8>, String> {
    // A4 dimensions
    let page_width = Mm(210.0);
    let page_height = Mm(297.0);

    // Margins and layout constants (in mm)
    let left_margin_mm: f32 = 20.0;
    let right_margin_mm: f32 = 190.0;
    let top_start_mm: f32 = 277.0; // start y from bottom (A4 = 297mm, 20mm top margin)
    let bottom_margin_mm: f32 = 25.0;
    let line_height_mm: f32 = 5.5;

    // Approximate characters per line at 11pt Helvetica on A4 with 20mm margins
    let max_chars_per_line: usize = 90;

    // Font handles
    let font_regular = PdfFontHandle::Builtin(BuiltinFont::Helvetica);
    let font_bold = PdfFontHandle::Builtin(BuiltinFont::HelveticaBold);
    let font_italic = PdfFontHandle::Builtin(BuiltinFont::HelveticaOblique);

    // Collect all pages; build ops for each page
    let mut pages: Vec<PdfPage> = Vec::new();
    let mut ops: Vec<Op> = Vec::new();
    let mut y = top_start_mm;
    let mut page_num: usize = 0;

    // Helper: flush current ops into a page (adds page-number footer) and start fresh.
    // Call `page_num += 1` before calling this.
    let flush_page = |ops: &mut Vec<Op>, pages: &mut Vec<PdfPage>, page_num: usize| {
        // Close any open text section
        ops.push(Op::EndTextSection);

        // --- Page number footer: "- N -" centred below the bottom margin ---
        let page_num_str = format!("- {} -", page_num);
        let footer_x = center_x(&page_num_str, 9.0, left_margin_mm, right_margin_mm);
        ops.push(Op::StartTextSection);
        ops.push(Op::SetFont {
            font: font_regular.clone(),
            size: Pt(9.0),
        });
        ops.push(Op::SetTextCursor {
            pos: Point::new(Mm(footer_x), Mm(bottom_margin_mm - 5.0)),
        });
        ops.push(Op::ShowText {
            items: vec![TextItem::Text(page_num_str)],
        });
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

    // --- HEADER: School name (bold, centred, 16pt) ---
    let school_name_size = Pt(16.0);
    ops.push(Op::SetFont {
        font: font_bold.clone(),
        size: school_name_size,
    });
    let school_x = center_x(school_name, 16.0, left_margin_mm, right_margin_mm);
    ops.push(Op::SetTextCursor {
        pos: Point::new(Mm(school_x), Mm(y)),
    });
    ops.push(Op::ShowText {
        items: vec![TextItem::Text(school_name.to_string())],
    });
    y -= 8.0;

    // --- MOTTO (italic, centred, 10pt) if present ---
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

    // --- Divider line ---
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

    // --- Sub-header: Exam | Subject | Paper N | Form (Fix 1: 12pt) ---
    ops.push(Op::StartTextSection);
    ops.push(Op::SetFont {
        font: font_bold.clone(),
        size: Pt(12.0), // Fix 1: was Pt(11.0)
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

    // Compute total marks early so it can be shown in the header section
    let total_marks: i16 = questions.iter().map(|(_, m, _, _)| m).sum();

    // --- Time allowed (if set) ---
    if let Some(mins) = time_allowed_minutes {
        let time_str = if mins >= 60 {
            let h = mins / 60;
            let rem = mins % 60;
            if rem == 0 {
                if h == 1 {
                    format!("Time: 1 hour")
                } else {
                    format!("Time: {} hours", h)
                }
            } else {
                if h == 1 {
                    format!("Time: 1 hour {} minutes", rem)
                } else {
                    format!("Time: {} hours {} minutes", h, rem)
                }
            }
        } else {
            format!("Time: {} minutes", mins)
        };
        ops.push(Op::EndTextSection);
        ops.push(Op::StartTextSection);
        ops.push(Op::SetFont {
            font: font_regular.clone(),
            size: Pt(11.0),
        });
        let time_x = center_x(&time_str, 11.0, left_margin_mm, right_margin_mm);
        ops.push(Op::SetTextCursor {
            pos: Point::new(Mm(time_x), Mm(y)),
        });
        ops.push(Op::ShowText {
            items: vec![TextItem::Text(time_str)],
        });
        y -= 6.0;
    }

    // --- Total marks (always) ---
    let total_str = format!("Total Marks: {}", total_marks);
    ops.push(Op::EndTextSection);
    ops.push(Op::StartTextSection);
    ops.push(Op::SetFont {
        font: font_regular.clone(),
        size: Pt(11.0),
    });
    let total_hdr_x = center_x(&total_str, 11.0, left_margin_mm, right_margin_mm);
    ops.push(Op::SetTextCursor {
        pos: Point::new(Mm(total_hdr_x), Mm(y)),
    });
    ops.push(Op::ShowText {
        items: vec![TextItem::Text(total_str.clone())],
    });
    y -= 8.0;

    // --- Instructions block ---
    let effective_instructions: Vec<String> = if let Some(custom) = custom_instructions {
        custom.lines().map(|l| l.to_string()).collect()
    } else {
        vec![
            "Answer ALL questions in this paper.".to_string(),
            "Show all your working clearly in the spaces provided.".to_string(),
            "All answers must be written in the spaces provided.".to_string(),
            "Check that all pages are present before starting.".to_string(),
            "Candidates should check the paper for any missing pages.".to_string(),
        ]
    };
    for instr in &effective_instructions {
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
            items: vec![TextItem::Text(instr.clone())],
        });
        y -= 4.0;
    }
    y -= 3.0; // small extra gap before the rule

    // --- Horizontal rule below instructions ---
    ops.push(Op::EndTextSection);
    ops.push(Op::DrawLine {
        line: Line {
            points: vec![lp(left_margin_mm, y + 3.0), lp(right_margin_mm, y + 3.0)],
            is_closed: false,
        },
    });

    // --- Candidate information box ---
    let box_top = y;
    let num_rows = 5;
    let row_height = 9.0_f32;
    let inner_padding = 4.0_f32;
    let box_inner_height = num_rows as f32 * row_height;
    let box_height = box_inner_height + 2.0 * inner_padding;
    let box_bottom = box_top - box_height;

    // Draw border rectangle (0.5pt, RGB 0.3/0.3/0.3)
    ops.push(Op::SetOutlineColor {
        col: Color::Rgb(Rgb {
            r: 0.3,
            g: 0.3,
            b: 0.3,
            icc_profile: None,
        }),
    });
    ops.push(Op::SetOutlineThickness { pt: Pt(0.5) });
    // Top border
    ops.push(Op::DrawLine {
        line: Line {
            points: vec![lp(left_margin_mm, box_top), lp(right_margin_mm, box_top)],
            is_closed: false,
        },
    });
    // Bottom border
    ops.push(Op::DrawLine {
        line: Line {
            points: vec![
                lp(left_margin_mm, box_bottom),
                lp(right_margin_mm, box_bottom),
            ],
            is_closed: false,
        },
    });
    // Left border
    ops.push(Op::DrawLine {
        line: Line {
            points: vec![lp(left_margin_mm, box_top), lp(left_margin_mm, box_bottom)],
            is_closed: false,
        },
    });
    // Right border
    ops.push(Op::DrawLine {
        line: Line {
            points: vec![
                lp(right_margin_mm, box_top),
                lp(right_margin_mm, box_bottom),
            ],
            is_closed: false,
        },
    });

    // Draw rows with labels and fill lines
    let row_labels: &[(&str, Option<f32>)] = &[
        ("Name:", None),
        ("Adm. No.:", Some(60.0)),
        ("Class / Stream:", Some(60.0)),
        ("Signature:", Some(60.0)),
        ("Date:", Some(40.0)),
    ];

    let mut row_y = box_top - inner_padding - row_height / 2.0 + 2.0;

    for (label, fill_len_opt) in row_labels {
        let label_x = left_margin_mm + inner_padding;
        let right_inner = right_margin_mm - inner_padding;

        // Estimate label text width
        let label_text_width = label.len() as f32 * 10.0 * 0.5 * 0.3528;
        let fill_start_x = label_x + label_text_width + 2.0;
        let fill_end_x = match fill_len_opt {
            None => right_inner,
            Some(len) => (fill_start_x + len).min(right_inner),
        };

        // Draw label text
        ops.push(Op::StartTextSection);
        ops.push(Op::SetFont {
            font: font_regular.clone(),
            size: Pt(10.0),
        });
        ops.push(Op::SetTextCursor {
            pos: Point::new(Mm(label_x), Mm(row_y)),
        });
        ops.push(Op::ShowText {
            items: vec![TextItem::Text(label.to_string())],
        });
        ops.push(Op::EndTextSection);

        // Draw fill line 1.0 mm below baseline
        let fill_y = row_y - 1.0;
        ops.push(Op::SetOutlineColor {
            col: Color::Rgb(Rgb {
                r: 0.6,
                g: 0.6,
                b: 0.6,
                icc_profile: None,
            }),
        });
        ops.push(Op::SetOutlineThickness { pt: Pt(0.4) });
        ops.push(Op::DrawLine {
            line: Line {
                points: vec![lp(fill_start_x, fill_y), lp(fill_end_x, fill_y)],
                is_closed: false,
            },
        });

        row_y -= row_height;
    }

    y = box_bottom - 6.0; // 6mm gap after the box

    ops.push(Op::StartTextSection);

    // --- QUESTIONS ---

    let mut current_section: Option<&str> = None;
    let mut section_question_index: usize = 0;

    // Pre-compute section totals (section -> total marks)
    let mut section_totals: std::collections::HashMap<String, i16> =
        std::collections::HashMap::new();
    for (_, marks, _, section_opt) in questions.iter() {
        if let Some(sec) = section_opt {
            *section_totals.entry(sec.clone()).or_insert(0) += marks;
        }
    }

    // Check if ALL questions are unsectioned (fallback: number globally as before)
    let all_unsectioned = questions.iter().all(|(_, _, _, sec)| sec.is_none());
    let mut global_index: usize = 0; // used when all_unsectioned

    for (text, marks, _rubric, section_opt) in questions.iter() {
        // --- Section header (only when section changes and sections are in use) ---
        let section_ref = section_opt.as_deref();
        if !all_unsectioned && section_ref != current_section {
            current_section = section_ref;
            section_question_index = 0; // reset per-section counter

            if let Some(sec_letter) = section_ref {
                // 8mm gap before section header
                y -= 8.0;

                if y < bottom_margin_mm {
                    page_num += 1;
                    flush_page(&mut ops, &mut pages, page_num);
                    ops.push(Op::StartTextSection);
                    y = top_start_mm;
                }

                // "SECTION {letter}" — bold 13pt, centered
                let sec_header = format!("SECTION {}", sec_letter);
                ops.push(Op::EndTextSection);
                ops.push(Op::StartTextSection);
                ops.push(Op::SetFont {
                    font: font_bold.clone(),
                    size: Pt(13.0),
                });
                let sec_x = center_x(&sec_header, 13.0, left_margin_mm, right_margin_mm);
                ops.push(Op::SetTextCursor {
                    pos: Point::new(Mm(sec_x), Mm(y)),
                });
                ops.push(Op::ShowText {
                    items: vec![TextItem::Text(sec_header.clone())],
                });
                y -= 6.0;

                // "({total} marks)" — regular 11pt, centered
                let sec_total = section_totals.get(sec_letter).copied().unwrap_or(0);
                let sec_marks_str = if sec_total == 1 {
                    "(1 mark)".to_string()
                } else {
                    format!("({} marks)", sec_total)
                };
                ops.push(Op::EndTextSection);
                ops.push(Op::StartTextSection);
                ops.push(Op::SetFont {
                    font: font_regular.clone(),
                    size: Pt(11.0),
                });
                let sec_marks_x = center_x(&sec_marks_str, 11.0, left_margin_mm, right_margin_mm);
                ops.push(Op::SetTextCursor {
                    pos: Point::new(Mm(sec_marks_x), Mm(y)),
                });
                ops.push(Op::ShowText {
                    items: vec![TextItem::Text(sec_marks_str)],
                });
                y -= 6.0;

                // "Answer ALL questions in this section." — italic 10pt, centered
                let sec_instr = "Answer ALL questions in this section.";
                ops.push(Op::EndTextSection);
                ops.push(Op::StartTextSection);
                ops.push(Op::SetFont {
                    font: font_italic.clone(),
                    size: Pt(10.0),
                });
                let sec_instr_x = center_x(sec_instr, 10.0, left_margin_mm, right_margin_mm);
                ops.push(Op::SetTextCursor {
                    pos: Point::new(Mm(sec_instr_x), Mm(y)),
                });
                ops.push(Op::ShowText {
                    items: vec![TextItem::Text(sec_instr.to_string())],
                });
                y -= 5.0;

                // Full-width horizontal rule: 0.5pt, RGB (0.4, 0.4, 0.4)
                ops.push(Op::EndTextSection);
                ops.push(Op::SetOutlineColor {
                    col: Color::Rgb(Rgb {
                        r: 0.4,
                        g: 0.4,
                        b: 0.4,
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

                // 6mm before first question
                y -= 6.0;
            }
        }

        // --- Question number ---
        let display_num = if all_unsectioned {
            global_index += 1;
            global_index
        } else {
            section_question_index += 1;
            section_question_index
        };

        // Check if we need a new page before drawing this question's header
        if y < bottom_margin_mm {
            page_num += 1;
            flush_page(&mut ops, &mut pages, page_num);
            ops.push(Op::StartTextSection);
            y = top_start_mm;
        }

        // Fix 4: Compute right-aligned marks annotation width & position
        let marks_text = if *marks == 1 {
            "(1 mark)".to_string()
        } else {
            format!("({} marks)", marks)
        };
        let marks_text_width_mm = marks_text.len() as f64 * 11.0 * 0.5 * 0.3528;
        let marks_x = right_margin_mm - marks_text_width_mm as f32;

        // Question number: bold, 11pt (Fix 1: was 10pt)
        ops.push(Op::EndTextSection);
        ops.push(Op::StartTextSection);
        ops.push(Op::SetFont {
            font: font_bold.clone(),
            size: Pt(11.0), // Fix 1: was Pt(10.0)
        });
        ops.push(Op::SetTextCursor {
            pos: Point::new(Mm(left_margin_mm), Mm(y)),
        });
        ops.push(Op::ShowText {
            items: vec![TextItem::Text(format!("{}.", display_num))],
        });

        // Fix 4: Right-aligned marks annotation in the same text section as question number
        ops.push(Op::SetFont {
            font: font_regular.clone(),
            size: Pt(11.0),
        });
        ops.push(Op::SetTextCursor {
            pos: Point::new(Mm(marks_x), Mm(y)),
        });
        ops.push(Op::ShowText {
            items: vec![TextItem::Text(marks_text)],
        });

        // Question text in regular font, indented
        let indent_mm = left_margin_mm + 10.0;
        // Fix 4: Use text directly — no inline "(X marks)" suffix
        let available_chars = ((right_margin_mm - indent_mm) / (right_margin_mm - left_margin_mm)
            * max_chars_per_line as f32) as usize;
        let lines = word_wrap(text, available_chars.max(40));

        for line in lines.iter() {
            if y < bottom_margin_mm {
                page_num += 1;
                flush_page(&mut ops, &mut pages, page_num);
                ops.push(Op::StartTextSection);
                y = top_start_mm;
            }

            // Fix 1: 11pt for question text body (was 10pt)
            ops.push(Op::EndTextSection);
            ops.push(Op::StartTextSection);
            ops.push(Op::SetFont {
                font: font_regular.clone(),
                size: Pt(11.0), // Fix 1: was Pt(10.0)
            });
            ops.push(Op::SetTextCursor {
                pos: Point::new(Mm(indent_mm), Mm(y)),
            });
            ops.push(Op::ShowText {
                items: vec![TextItem::Text(line.clone())],
            });
            y -= line_height_mm;
        }

        // Fix 3: Close text section before drawing graphical answer lines
        ops.push(Op::EndTextSection);

        // Fix 3 + Fix 2: 4.0 mm gap between last text line and first answer line
        y -= 4.0;

        // Fix 3: Draw ruled answer lines (light grey, full-width)
        let num_answer_lines = ((*marks as usize) * 2).max(3);
        ops.push(Op::SetOutlineColor {
            col: Color::Rgb(Rgb {
                r: 0.75,
                g: 0.75,
                b: 0.75,
                icc_profile: None,
            }),
        });
        ops.push(Op::SetOutlineThickness { pt: Pt(0.3) });

        for _ in 0..num_answer_lines {
            // Check page space before drawing this line (need 7.0 mm)
            if y < bottom_margin_mm + 7.0 {
                page_num += 1;
                flush_page(&mut ops, &mut pages, page_num);
                y = top_start_mm;
                // Restore answer-line style after flush
                ops.push(Op::SetOutlineColor {
                    col: Color::Rgb(Rgb {
                        r: 0.75,
                        g: 0.75,
                        b: 0.75,
                        icc_profile: None,
                    }),
                });
                ops.push(Op::SetOutlineThickness { pt: Pt(0.3) });
            }
            y -= 7.0;
            ops.push(Op::DrawLine {
                line: Line {
                    points: vec![lp(left_margin_mm, y), lp(right_margin_mm, y)],
                    is_closed: false,
                },
            });
        }

        // Fix 2: Inter-question gap — 12.0 mm (was paragraph_spacing_mm = 3.0 mm)
        y -= 12.0;
    }

    // --- Footer: Total marks ---
    if y < bottom_margin_mm + 10.0 {
        page_num += 1;
        flush_page(&mut ops, &mut pages, page_num);
        ops.push(Op::StartTextSection);
        y = top_start_mm;
    }

    y -= 2.0;
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
    y -= line_height_mm;

    // Fix 5: "— END OF PAPER —" centred, bold, 11pt, with 10.0 mm top margin
    y -= 10.0;
    let end_text = "\u{2014} END OF PAPER \u{2014}";
    let end_x = center_x(end_text, 11.0, left_margin_mm, right_margin_mm);
    ops.push(Op::EndTextSection);
    ops.push(Op::StartTextSection);
    ops.push(Op::SetFont {
        font: font_bold.clone(),
        size: Pt(11.0),
    });
    ops.push(Op::SetTextCursor {
        pos: Point::new(Mm(end_x), Mm(y)),
    });
    ops.push(Op::ShowText {
        items: vec![TextItem::Text(end_text.to_string())],
    });

    // Flush the last page
    page_num += 1;
    flush_page(&mut ops, &mut pages, page_num);

    // Fix 5: Add "Turn over" to all non-final pages
    if pages.len() > 1 {
        let last_idx = pages.len() - 1;
        let turnover_text = "Turn over";
        let text_width_mm = turnover_text.len() as f32 * 9.0 * 0.5 * 0.3528;
        let turnover_x = right_margin_mm - text_width_mm;
        for page in &mut pages[0..last_idx] {
            page.ops.push(Op::StartTextSection);
            page.ops.push(Op::SetFont {
                font: font_regular.clone(),
                size: Pt(9.0),
            });
            page.ops.push(Op::SetTextCursor {
                pos: Point::new(Mm(turnover_x), Mm(bottom_margin_mm - 5.0)),
            });
            page.ops.push(Op::ShowText {
                items: vec![TextItem::Text(turnover_text.to_string())],
            });
            page.ops.push(Op::EndTextSection);
        }
    }

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

/// Generate a marking scheme PDF from question rubric data.
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
/// * `time_allowed_minutes` — Optional exam duration in minutes
/// * `custom_instructions` — Optional multi-line instructions string; falls back to 5 defaults
/// * `questions` — Slice of (question_text, marks, rubric_criteria, section) tuples
///   where rubric_criteria is a vec of (criterion_text, marks) and section is an optional
///   section letter string (ignored in the marking scheme renderer)
pub fn generate_marking_scheme_pdf(
    school_name: &str,
    school_motto: Option<&str>,
    exam_name: &str,
    subject_name: &str,
    paper_number: Option<i16>,
    grade: i16,
    time_allowed_minutes: Option<i16>,
    custom_instructions: Option<&str>,
    questions: &[(String, i16, Vec<(String, i16)>, Option<String>)],
) -> Result<Vec<u8>, String> {
    // A4 dimensions
    let page_width = Mm(210.0);
    let page_height = Mm(297.0);

    // Margins and layout constants (in mm)
    let left_margin_mm: f32 = 20.0;
    let right_margin_mm: f32 = 190.0;
    let top_start_mm: f32 = 277.0;
    let bottom_margin_mm: f32 = 25.0;
    let line_height_mm: f32 = 5.5;

    // Approximate characters per line at 11pt Helvetica on A4 with 20mm margins
    let max_chars_per_line: usize = 90;

    // Font handles
    let font_regular = PdfFontHandle::Builtin(BuiltinFont::Helvetica);
    let font_bold = PdfFontHandle::Builtin(BuiltinFont::HelveticaBold);
    let font_italic = PdfFontHandle::Builtin(BuiltinFont::HelveticaOblique);

    // Collect all pages; build ops for each page
    let mut pages: Vec<PdfPage> = Vec::new();
    let mut ops: Vec<Op> = Vec::new();
    let mut y = top_start_mm;
    let mut page_num: usize = 0;

    // Helper: flush current ops into a page (adds page-number footer) and start fresh.
    let flush_page = |ops: &mut Vec<Op>, pages: &mut Vec<PdfPage>, page_num: usize| {
        ops.push(Op::EndTextSection);

        let page_num_str = format!("- {} -", page_num);
        let footer_x = center_x(&page_num_str, 9.0, left_margin_mm, right_margin_mm);
        ops.push(Op::StartTextSection);
        ops.push(Op::SetFont {
            font: font_regular.clone(),
            size: Pt(9.0),
        });
        ops.push(Op::SetTextCursor {
            pos: Point::new(Mm(footer_x), Mm(bottom_margin_mm - 5.0)),
        });
        ops.push(Op::ShowText {
            items: vec![TextItem::Text(page_num_str)],
        });
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

    // --- HEADER: School name (bold, centred, 16pt) ---
    ops.push(Op::SetFont {
        font: font_bold.clone(),
        size: Pt(16.0),
    });
    let school_x = center_x(school_name, 16.0, left_margin_mm, right_margin_mm);
    ops.push(Op::SetTextCursor {
        pos: Point::new(Mm(school_x), Mm(y)),
    });
    ops.push(Op::ShowText {
        items: vec![TextItem::Text(school_name.to_string())],
    });
    y -= 8.0;

    // --- MOTTO (italic, centred, 10pt) if present ---
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

    // --- Divider line ---
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

    // --- Sub-header with MARKING SCHEME suffix (bold, 12pt) ---
    let scheme_exam_name = format!("{} \u{2014} MARKING SCHEME", exam_name);
    ops.push(Op::StartTextSection);
    ops.push(Op::SetFont {
        font: font_bold.clone(),
        size: Pt(12.0),
    });
    let paper_str = paper_number
        .map(|p| format!("  |  Paper {}", p))
        .unwrap_or_default();
    let sub_header = format!(
        "{}  |  {}{}  |  Form {}",
        scheme_exam_name, subject_name, paper_str, grade
    );
    ops.push(Op::SetTextCursor {
        pos: Point::new(Mm(left_margin_mm), Mm(y)),
    });
    ops.push(Op::ShowText {
        items: vec![TextItem::Text(sub_header)],
    });
    y -= 8.0;

    // Compute total marks early so it can be shown in the header section
    let total_marks: i16 = questions.iter().map(|(_, m, _, _)| m).sum();

    // --- Time allowed (if set) ---
    if let Some(mins) = time_allowed_minutes {
        let time_str = if mins >= 60 {
            let h = mins / 60;
            let rem = mins % 60;
            if rem == 0 {
                if h == 1 {
                    "Time: 1 hour".to_string()
                } else {
                    format!("Time: {} hours", h)
                }
            } else if h == 1 {
                format!("Time: 1 hour {} minutes", rem)
            } else {
                format!("Time: {} hours {} minutes", h, rem)
            }
        } else {
            format!("Time: {} minutes", mins)
        };
        ops.push(Op::EndTextSection);
        ops.push(Op::StartTextSection);
        ops.push(Op::SetFont {
            font: font_regular.clone(),
            size: Pt(11.0),
        });
        let time_x = center_x(&time_str, 11.0, left_margin_mm, right_margin_mm);
        ops.push(Op::SetTextCursor {
            pos: Point::new(Mm(time_x), Mm(y)),
        });
        ops.push(Op::ShowText {
            items: vec![TextItem::Text(time_str)],
        });
        y -= 6.0;
    }

    // --- Total marks (always) ---
    let total_str = format!("Total Marks: {}", total_marks);
    ops.push(Op::EndTextSection);
    ops.push(Op::StartTextSection);
    ops.push(Op::SetFont {
        font: font_regular.clone(),
        size: Pt(11.0),
    });
    let total_hdr_x = center_x(&total_str, 11.0, left_margin_mm, right_margin_mm);
    ops.push(Op::SetTextCursor {
        pos: Point::new(Mm(total_hdr_x), Mm(y)),
    });
    ops.push(Op::ShowText {
        items: vec![TextItem::Text(total_str)],
    });
    y -= 8.0;

    // --- Instructions block ---
    let effective_instructions: Vec<String> = if let Some(custom) = custom_instructions {
        custom.lines().map(|l| l.to_string()).collect()
    } else {
        vec![
            "Answer ALL questions in this paper.".to_string(),
            "Show all your working clearly in the spaces provided.".to_string(),
            "All answers must be written in the spaces provided.".to_string(),
            "Check that all pages are present before starting.".to_string(),
            "Candidates should check the paper for any missing pages.".to_string(),
        ]
    };
    for instr in &effective_instructions {
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
            items: vec![TextItem::Text(instr.clone())],
        });
        y -= 4.0;
    }
    y -= 3.0; // small extra gap before the rule

    // --- Horizontal rule below instructions ---
    ops.push(Op::EndTextSection);
    ops.push(Op::DrawLine {
        line: Line {
            points: vec![lp(left_margin_mm, y + 3.0), lp(right_margin_mm, y + 3.0)],
            is_closed: false,
        },
    });

    // No candidate info box in the marking scheme — just a small gap
    y -= 4.0;

    ops.push(Op::StartTextSection);

    // --- QUESTIONS with rubric criteria ---
    for (i, (text, marks, rubric, _section)) in questions.iter().enumerate() {
        // Page check
        if y < bottom_margin_mm {
            page_num += 1;
            flush_page(&mut ops, &mut pages, page_num);
            ops.push(Op::StartTextSection);
            y = top_start_mm;
        }

        // Question preview: first 120 chars, truncated with "..."
        let question_preview = if text.chars().count() > 120 {
            let truncated: String = text.chars().take(120).collect();
            format!("{}...", truncated)
        } else {
            text.clone()
        };

        // Render question number in bold 11pt
        ops.push(Op::EndTextSection);
        ops.push(Op::StartTextSection);
        ops.push(Op::SetFont {
            font: font_bold.clone(),
            size: Pt(11.0),
        });
        ops.push(Op::SetTextCursor {
            pos: Point::new(Mm(left_margin_mm), Mm(y)),
        });
        ops.push(Op::ShowText {
            items: vec![TextItem::Text(format!("{}.", i + 1))],
        });

        // Render question preview in italic 10pt, indented
        let indent_mm = left_margin_mm + 10.0;
        let available_chars = ((right_margin_mm - indent_mm) / (right_margin_mm - left_margin_mm)
            * max_chars_per_line as f32) as usize;
        let preview_lines = word_wrap(&question_preview, available_chars.max(40));
        for line in &preview_lines {
            if y < bottom_margin_mm {
                page_num += 1;
                flush_page(&mut ops, &mut pages, page_num);
                ops.push(Op::StartTextSection);
                y = top_start_mm;
            }
            ops.push(Op::EndTextSection);
            ops.push(Op::StartTextSection);
            ops.push(Op::SetFont {
                font: font_italic.clone(),
                size: Pt(10.0),
            });
            ops.push(Op::SetTextCursor {
                pos: Point::new(Mm(indent_mm), Mm(y)),
            });
            ops.push(Op::ShowText {
                items: vec![TextItem::Text(line.clone())],
            });
            y -= line_height_mm;
        }
        y -= 2.0; // gap before criteria

        // Render rubric criteria
        if rubric.is_empty() {
            ops.push(Op::EndTextSection);
            ops.push(Op::StartTextSection);
            ops.push(Op::SetFont {
                font: font_italic.clone(),
                size: Pt(10.0),
            });
            ops.push(Op::SetTextCursor {
                pos: Point::new(Mm(indent_mm), Mm(y)),
            });
            ops.push(Op::ShowText {
                items: vec![TextItem::Text("(No marking criteria defined)".to_string())],
            });
            y -= line_height_mm;
        } else {
            for (criterion_text, criterion_marks) in rubric {
                if y < bottom_margin_mm {
                    page_num += 1;
                    flush_page(&mut ops, &mut pages, page_num);
                    ops.push(Op::StartTextSection);
                    y = top_start_mm;
                }

                let bullet = format!("    \u{2022} {}", criterion_text);
                let crit_lines = word_wrap(&bullet, available_chars.max(40));
                // Save y before rendering so we can right-align marks at the first line's y
                let first_line_y = y;
                for line in &crit_lines {
                    ops.push(Op::EndTextSection);
                    ops.push(Op::StartTextSection);
                    ops.push(Op::SetFont {
                        font: font_regular.clone(),
                        size: Pt(10.0),
                    });
                    ops.push(Op::SetTextCursor {
                        pos: Point::new(Mm(indent_mm), Mm(y)),
                    });
                    ops.push(Op::ShowText {
                        items: vec![TextItem::Text(line.clone())],
                    });
                    y -= line_height_mm;
                }

                // Right-aligned marks "[N mark(s)]" at the same y as the first criterion line
                let crit_marks_str = if *criterion_marks == 1 {
                    "[1 mark]".to_string()
                } else {
                    format!("[{} marks]", criterion_marks)
                };
                let crit_marks_width = crit_marks_str.len() as f32 * 10.0 * 0.5 * 0.3528;
                let crit_marks_x = right_margin_mm - crit_marks_width;
                ops.push(Op::EndTextSection);
                ops.push(Op::StartTextSection);
                ops.push(Op::SetFont {
                    font: font_regular.clone(),
                    size: Pt(10.0),
                });
                ops.push(Op::SetTextCursor {
                    pos: Point::new(Mm(crit_marks_x), Mm(first_line_y)),
                });
                ops.push(Op::ShowText {
                    items: vec![TextItem::Text(crit_marks_str)],
                });

                y -= 3.0; // 3mm gap between criteria
            }
        }

        // Thin divider line (0.3pt, RGB 0.8/0.8/0.8)
        ops.push(Op::EndTextSection);
        ops.push(Op::SetOutlineColor {
            col: Color::Rgb(Rgb {
                r: 0.8,
                g: 0.8,
                b: 0.8,
                icc_profile: None,
            }),
        });
        ops.push(Op::SetOutlineThickness { pt: Pt(0.3) });
        ops.push(Op::DrawLine {
            line: Line {
                points: vec![lp(left_margin_mm, y), lp(right_margin_mm, y)],
                is_closed: false,
            },
        });
        y -= 4.0;

        // Total right-aligned: "Total: N mark(s)"
        let q_total: i16 = *marks;
        let q_total_str = if q_total == 1 {
            "Total: 1 mark".to_string()
        } else {
            format!("Total: {} marks", q_total)
        };
        let q_total_width = q_total_str.len() as f32 * 10.0 * 0.5 * 0.3528;
        let q_total_x = right_margin_mm - q_total_width;
        ops.push(Op::StartTextSection);
        ops.push(Op::SetFont {
            font: font_bold.clone(),
            size: Pt(10.0),
        });
        ops.push(Op::SetTextCursor {
            pos: Point::new(Mm(q_total_x), Mm(y)),
        });
        ops.push(Op::ShowText {
            items: vec![TextItem::Text(q_total_str)],
        });
        y -= line_height_mm;

        // Inter-question gap: 8mm
        y -= 8.0;
    }

    // --- Footer: Total marks ---
    if y < bottom_margin_mm + 10.0 {
        page_num += 1;
        flush_page(&mut ops, &mut pages, page_num);
        ops.push(Op::StartTextSection);
        y = top_start_mm;
    }

    y -= 2.0;
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
    let total_footer_str = format!("Total: {} marks", total_marks);
    let total_footer_x = right_margin_mm - (total_footer_str.len() as f32 * 2.5);
    ops.push(Op::SetTextCursor {
        pos: Point::new(Mm(total_footer_x.max(left_margin_mm)), Mm(y)),
    });
    ops.push(Op::ShowText {
        items: vec![TextItem::Text(total_footer_str)],
    });
    y -= line_height_mm;

    // "— END OF MARKING SCHEME —" centred, bold, 11pt, with 10.0 mm top margin
    y -= 10.0;
    let end_scheme_text = "\u{2014} END OF MARKING SCHEME \u{2014}";
    let end_scheme_x = center_x(end_scheme_text, 11.0, left_margin_mm, right_margin_mm);
    ops.push(Op::EndTextSection);
    ops.push(Op::StartTextSection);
    ops.push(Op::SetFont {
        font: font_bold.clone(),
        size: Pt(11.0),
    });
    ops.push(Op::SetTextCursor {
        pos: Point::new(Mm(end_scheme_x), Mm(y)),
    });
    ops.push(Op::ShowText {
        items: vec![TextItem::Text(end_scheme_text.to_string())],
    });

    // Flush the last page
    page_num += 1;
    flush_page(&mut ops, &mut pages, page_num);

    // Add "Turn over" to all non-final pages
    if pages.len() > 1 {
        let last_idx = pages.len() - 1;
        let turnover_text = "Turn over";
        let text_width_mm = turnover_text.len() as f32 * 9.0 * 0.5 * 0.3528;
        let turnover_x = right_margin_mm - text_width_mm;
        for page in &mut pages[0..last_idx] {
            page.ops.push(Op::StartTextSection);
            page.ops.push(Op::SetFont {
                font: font_regular.clone(),
                size: Pt(9.0),
            });
            page.ops.push(Op::SetTextCursor {
                pos: Point::new(Mm(turnover_x), Mm(bottom_margin_mm - 5.0)),
            });
            page.ops.push(Op::ShowText {
                items: vec![TextItem::Text(turnover_text.to_string())],
            });
            page.ops.push(Op::EndTextSection);
        }
    }

    // Build the document
    let mut doc = PdfDocument::new("Marking Scheme");
    doc.metadata.info.document_title = format!("{} - {} Marking Scheme", exam_name, subject_name);
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
/// and returns the x position (in mm) to start the text so it appears centred.
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
                None::<String>,
            ),
            (
                "Explain the water cycle.".to_string(),
                5_i16,
                vec![
                    ("Evaporation".to_string(), 2_i16),
                    ("Condensation and precipitation".to_string(), 3_i16),
                ],
                None::<String>,
            ),
        ];

        let result = generate_paper_pdf(
            "Moi High School",
            Some("Excellence in Education"),
            "End of Term 1 Exam 2025",
            "Mathematics",
            Some(1),
            10,
            None,
            None,
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
        let questions = vec![("Define osmosis.".to_string(), 3_i16, vec![], None::<String>)];

        let result = generate_paper_pdf(
            "Test School",
            None,
            "CAT 1",
            "Biology",
            None,
            9,
            None,
            None,
            &questions,
        );

        assert!(result.is_ok());
        let bytes = result.unwrap();
        assert_eq!(&bytes[0..5], b"%PDF-");
    }

    #[test]
    fn test_generate_paper_pdf_many_questions_multi_page() {
        // Generate enough questions to force multiple pages
        let questions: Vec<(String, i16, Vec<(String, i16)>, Option<String>)> = (0..60)
            .map(|i| {
                (
                    format!(
                        "This is question number {} which has a reasonably long text to take up space on the page and help trigger page breaks during PDF generation.",
                        i + 1
                    ),
                    3_i16,
                    vec![("criterion".to_string(), 3_i16)],
                    None::<String>,
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
            None,
            None,
            &questions,
        );

        assert!(result.is_ok());
        let bytes = result.unwrap();
        assert_eq!(&bytes[0..5], b"%PDF-");
        // Multi-page PDF should be larger
        assert!(bytes.len() > 1000);
    }
}
