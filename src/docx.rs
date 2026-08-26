//! DOCX generation for exam papers, assessments, and marking schemes.

use std::io::{Cursor, Write};
use zip::write::SimpleFileOptions;
use zip::ZipWriter;

use crate::pdf::PaperPdfInput;
use crate::types::question::{ExampleAnswer, ExampleAnswerFormat, Stimulus, StimulusType};

// ── XML helper functions ──────────────────────────────────────────────────────

fn xml_escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&apos;"),
            _ => out.push(c),
        }
    }
    out
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

fn render_body_text(body: &str, body_format: u8) -> String {
    if body_format == 1 {
        strip_html(body)
    } else {
        body.to_owned()
    }
}

fn format_grade_display(grade: i16) -> String {
    match grade {
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
        41 => "Form 1".into(),
        42 => "Form 2".into(),
        43 => "Form 3".into(),
        44 => "Form 4".into(),
        other => format!("Grade {}", other),
    }
}

fn detect_image_mime_and_ext(bytes: &[u8]) -> (&'static str, &'static str) {
    if bytes.len() >= 4 {
        if bytes[0..4] == [0x89, 0x50, 0x4E, 0x47] {
            return ("image/png", "png");
        }
        if bytes[0..2] == [0xFF, 0xD8] {
            return ("image/jpeg", "jpg");
        }
        if bytes[0..3] == [0x47, 0x49, 0x46] {
            return ("image/gif", "gif");
        }
    }
    ("image/png", "png")
}

// ── Docx Generation Engine ────────────────────────────────────────────────────

struct DocxContext<'a> {
    input: &'a PaperPdfInput<'a>,
    student_name: Option<&'a str>,
    student_adm: Option<i32>,
    is_marking_scheme: bool,
    images: Vec<DocxImage>,
}

struct DocxImage {
    key: String,
    r_id: String,
    filename: String,
    bytes: Vec<u8>,
}

impl<'a> DocxContext<'a> {
    fn new(
        input: &'a PaperPdfInput<'a>,
        student_name: Option<&'a str>,
        student_adm: Option<i32>,
        is_marking_scheme: bool,
    ) -> Self {
        let mut images = Vec::new();
        let mut img_counter = 1;

        for q in input.questions {
            for img in &q.images {
                if let Some(bytes) = input.image_data.get(&img.key) {
                    let (_mime, ext) = detect_image_mime_and_ext(bytes);
                    let filename = format!("image{}.{}", img_counter, ext);
                    let r_id = format!("rIdImg{}", img_counter);
                    img_counter += 1;
                    images.push(DocxImage {
                        key: img.key.clone(),
                        r_id,
                        filename,
                        bytes: bytes.clone(),
                    });
                }
            }
        }

        Self {
            input,
            student_name,
            student_adm,
            is_marking_scheme,
            images,
        }
    }

    fn build_content_types_xml(&self) -> String {
        let extensions = vec![
            ("rels", "application/vnd.openxmlformats-package.relationships+xml"),
            ("xml", "application/xml"),
            ("png", "image/png"),
            ("jpg", "image/jpeg"),
            ("jpeg", "image/jpeg"),
            ("gif", "image/gif"),
        ];

        let mut out = String::from(
            r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types">
"#,
        );
        for (ext, ct) in extensions {
            out.push_str(&format!(
                r#"  <Default Extension="{}" ContentType="{}"/>
"#,
                ext, ct
            ));
        }
        out.push_str(
            r#"  <Override PartName="/word/document.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.document.main+xml"/>
  <Override PartName="/word/styles.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.styles+xml"/>
  <Override PartName="/word/settings.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.settings+xml"/>
  <Override PartName="/word/fontTable.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.fontTable+xml"/>
  <Override PartName="/word/footer1.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.footer+xml"/>
</Types>"#,
        );
        out
    }

    fn build_rels_xml(&self) -> String {
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
  <Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="word/document.xml"/>
</Relationships>"#
            .to_string()
    }

    fn build_doc_rels_xml(&self) -> String {
        let mut out = String::from(
            r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
  <Relationship Id="rIdStyles" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/styles" Target="styles.xml"/>
  <Relationship Id="rIdSettings" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/settings" Target="settings.xml"/>
  <Relationship Id="rIdFontTable" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/fontTable" Target="fontTable.xml"/>
  <Relationship Id="rIdFooter1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/footer" Target="footer1.xml"/>
"#,
        );

        for img in &self.images {
            out.push_str(&format!(
                r#"  <Relationship Id="{}" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/image" Target="media/{}"/>
"#,
                img.r_id, img.filename
            ));
        }

        out.push_str("</Relationships>");
        out
    }

    fn build_styles_xml(&self) -> String {
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<w:styles xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
  <w:docDefaults>
    <w:rPrDefault>
      <w:rPr>
        <w:rFonts w:ascii="Calibri" w:hAnsi="Calibri" w:cs="Calibri"/>
        <w:sz w:val="22"/>
        <w:szCs w:val="22"/>
        <w:lang w:val="en-GB"/>
      </w:rPr>
    </w:rPrDefault>
    <w:pPrDefault>
      <w:pPr>
        <w:spacing w:after="120" w:line="240" w:lineRule="auto"/>
      </w:pPr>
    </w:pPrDefault>
  </w:docDefaults>
  <w:style w:type="paragraph" w:default="1" w:styleId="Normal">
    <w:name w:val="Normal"/>
    <w:qFormat/>
  </w:style>
  <w:style w:type="paragraph" w:styleId="Heading1">
    <w:name w:val="heading 1"/>
    <w:pPr>
      <w:spacing w:before="240" w:after="120"/>
      <w:jc w:val="center"/>
    </w:pPr>
    <w:rPr>
      <w:b/>
      <w:sz w:val="32"/>
      <w:szCs w:val="32"/>
    </w:rPr>
  </w:style>
  <w:style w:type="paragraph" w:styleId="SectionHeader">
    <w:name w:val="Section Header"/>
    <w:pPr>
      <w:spacing w:before="200" w:after="100"/>
      <w:jc w:val="center"/>
    </w:pPr>
    <w:rPr>
      <w:b/>
      <w:sz w:val="24"/>
      <w:szCs w:val="24"/>
    </w:rPr>
  </w:style>
</w:styles>"#
            .to_string()
    }

    fn build_settings_xml(&self) -> String {
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<w:settings xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
  <w:defaultTabStop w:val="720"/>
</w:settings>"#
            .to_string()
    }

    fn build_font_table_xml(&self) -> String {
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<w:fonts xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
  <w:font w:name="Calibri">
    <w:panose1 w:val="020F0502020204030204"/>
    <w:charset w:val="00"/>
    <w:family w:val="swiss"/>
    <w:pitch w:val="variable"/>
  </w:font>
  <w:font w:name="Times New Roman">
    <w:panose1 w:val="02020603050405020304"/>
    <w:charset w:val="00"/>
    <w:family w:val="roman"/>
    <w:pitch w:val="variable"/>
  </w:font>
</w:fonts>"#
            .to_string()
    }

    fn build_footer_xml(&self) -> String {
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<w:ftr xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
  <w:p>
    <w:pPr>
      <w:jc w:val="center"/>
    </w:pPr>
    <w:r>
      <w:rPr>
        <w:sz w:val="18"/>
        <w:color w:val="666666"/>
      </w:rPr>
      <w:t xml:space="preserve">Page </w:t>
    </w:r>
    <w:fldSimple w:instr="PAGE"/>
    <w:r>
      <w:rPr>
        <w:sz w:val="18"/>
        <w:color w:val="666666"/>
      </w:rPr>
      <w:t xml:space="preserve"> of </w:t>
    </w:r>
    <w:fldSimple w:instr="NUMPAGES"/>
  </w:p>
</w:ftr>"#
            .to_string()
    }

    fn build_document_xml(&self) -> String {
        let mut body = String::new();

        // ── 1. School Header ─────────────────────────────────────────────────
        body.push_str(&format!(
            r#"<w:p>
  <w:pPr>
    <w:jc w:val="center"/>
    <w:spacing w:before="0" w:after="60"/>
  </w:pPr>
  <w:r>
    <w:rPr>
      <w:b/>
      <w:sz w:val="32"/>
      <w:szCs w:val="32"/>
    </w:rPr>
    <w:t>{}</w:t>
  </w:r>
</w:p>
"#,
            xml_escape(&self.input.school_name.to_uppercase())
        ));

        if let Some(motto) = self.input.school_motto {
            if !motto.is_empty() {
                body.push_str(&format!(
                    r#"<w:p>
  <w:pPr>
    <w:jc w:val="center"/>
    <w:spacing w:before="0" w:after="100"/>
  </w:pPr>
  <w:r>
    <w:rPr>
      <w:i/>
      <w:sz w:val="20"/>
      <w:szCs w:val="20"/>
      <w:color w:val="555555"/>
    </w:rPr>
    <w:t>&quot;{}&quot;</w:t>
  </w:r>
</w:p>
"#,
                    xml_escape(motto)
                ));
            }
        }

        // Marking scheme badge or Paper Title
        if self.is_marking_scheme {
            body.push_str(
                r#"<w:p>
  <w:pPr>
    <w:jc w:val="center"/>
    <w:spacing w:before="40" w:after="80"/>
  </w:pPr>
  <w:r>
    <w:rPr>
      <w:b/>
      <w:sz w:val="26"/>
      <w:color w:val="8A1C14"/>
    </w:rPr>
    <w:t>[MARKING SCHEME &amp; RUBRIC]</w:t>
  </w:r>
</w:p>
"#,
            );
        }

        // Paper Name
        body.push_str(&format!(
            r#"<w:p>
  <w:pPr>
    <w:jc w:val="center"/>
    <w:spacing w:before="60" w:after="80"/>
  </w:pPr>
  <w:r>
    <w:rPr>
      <w:b/>
      <w:sz w:val="28"/>
      <w:szCs w:val="28"/>
    </w:rPr>
    <w:t>{}</w:t>
  </w:r>
</w:p>
"#,
            xml_escape(&self.input.paper_name.to_uppercase())
        ));

        // Subject, Grade, Paper Number
        let grade_label = format_grade_display(self.input.grade);
        let paper_num_str = self
            .input
            .paper_number
            .map(|p| format!(" — Paper {}", p))
            .unwrap_or_default();
        let sub_header = format!(
            "{} — {}{}",
            self.input.subject_name.to_uppercase(),
            grade_label.to_uppercase(),
            paper_num_str
        );

        body.push_str(&format!(
            r#"<w:p>
  <w:pPr>
    <w:jc w:val="center"/>
    <w:spacing w:before="0" w:after="80"/>
  </w:pPr>
  <w:r>
    <w:rPr>
      <w:b/>
      <w:sz w:val="24"/>
      <w:szCs w:val="24"/>
    </w:rPr>
    <w:t>{}</w:t>
  </w:r>
</w:p>
"#,
            xml_escape(&sub_header)
        ));

        // Duration / Time Allowed
        if let Some(dur) = self.input.duration_minutes {
            let dur_str = if dur >= 60 {
                let h = dur / 60;
                let m = dur % 60;
                if m > 0 {
                    format!("{} Hours {} Minutes", h, m)
                } else if h == 1 {
                    "1 Hour".into()
                } else {
                    format!("{} Hours", h)
                }
            } else {
                format!("{} Minutes", dur)
            };

            body.push_str(&format!(
                r#"<w:p>
  <w:pPr>
    <w:jc w:val="center"/>
    <w:spacing w:before="0" w:after="160"/>
  </w:pPr>
  <w:r>
    <w:rPr>
      <w:b/>
      <w:sz w:val="20"/>
      <w:color w:val="444444"/>
    </w:rPr>
    <w:t>TIME ALLOWED: {}</w:t>
  </w:r>
</w:p>
"#,
                xml_escape(&dur_str)
            ));
        }

        // ── 2. Candidate Details Table (Only for exam papers) ─────────────────
        if !self.is_marking_scheme {
            let cand_name = self.student_name.unwrap_or("________________________________________________");
            let cand_adm = self
                .student_adm
                .map(|a| a.to_string())
                .unwrap_or_else(|| "________________".into());

            body.push_str(&format!(
                r#"<w:tbl>
  <w:tblPr>
    <w:tblW w:w="5000" w:type="pct"/>
    <w:tblBorders>
      <w:top w:val="single" w:sz="6" w:space="0" w:color="CCCCCC"/>
      <w:left w:val="single" w:sz="6" w:space="0" w:color="CCCCCC"/>
      <w:bottom w:val="single" w:sz="6" w:space="0" w:color="CCCCCC"/>
      <w:right w:val="single" w:sz="6" w:space="0" w:color="CCCCCC"/>
      <w:insideH w:val="single" w:sz="4" w:space="0" w:color="EAEAEA"/>
      <w:insideV w:val="single" w:sz="4" w:space="0" w:color="EAEAEA"/>
    </w:tblBorders>
    <w:tblCellMar>
      <w:top w:w="120" w:type="dxa"/>
      <w:bottom w:w="120" w:type="dxa"/>
      <w:left w:w="160" w:type="dxa"/>
      <w:right w:w="160" w:type="dxa"/>
    </w:tblCellMar>
  </w:tblPr>
  <w:tr>
    <w:tc>
      <w:tcPr><w:tcW w:w="3200" w:type="pct"/></w:tcPr>
      <w:p>
        <w:pPr><w:spacing w:after="40" w:before="40"/></w:pPr>
        <w:r><w:rPr><w:b/><w:sz w:val="20"/></w:rPr><w:t>STUDENT NAME: </w:t></w:r>
        <w:r><w:rPr><w:sz w:val="20"/></w:rPr><w:t>{}</w:t></w:r>
      </w:p>
    </w:tc>
    <w:tc>
      <w:tcPr><w:tcW w:w="1800" w:type="pct"/></w:tcPr>
      <w:p>
        <w:pPr><w:spacing w:after="40" w:before="40"/></w:pPr>
        <w:r><w:rPr><w:b/><w:sz w:val="20"/></w:rPr><w:t>ADM NO: </w:t></w:r>
        <w:r><w:rPr><w:sz w:val="20"/></w:rPr><w:t>{}</w:t></w:r>
      </w:p>
    </w:tc>
  </w:tr>
  <w:tr>
    <w:tc>
      <w:tcPr><w:tcW w:w="3200" w:type="pct"/></w:tcPr>
      <w:p>
        <w:pPr><w:spacing w:after="40" w:before="40"/></w:pPr>
        <w:r><w:rPr><w:b/><w:sz w:val="20"/></w:rPr><w:t>CLASS / STREAM: </w:t></w:r>
        <w:r><w:rPr><w:sz w:val="20"/></w:rPr><w:t>____________________________________</w:t></w:r>
      </w:p>
    </w:tc>
    <w:tc>
      <w:tcPr><w:tcW w:w="1800" w:type="pct"/></w:tcPr>
      <w:p>
        <w:pPr><w:spacing w:after="40" w:before="40"/></w:pPr>
        <w:r><w:rPr><w:b/><w:sz w:val="20"/></w:rPr><w:t>SCORE: </w:t></w:r>
        <w:r><w:rPr><w:sz w:val="20"/></w:rPr><w:t>______ / ______</w:t></w:r>
      </w:p>
    </w:tc>
  </w:tr>
</w:tbl>
<w:p><w:pPr><w:spacing w:after="160"/></w:pPr></w:p>
"#,
                xml_escape(cand_name),
                xml_escape(&cand_adm)
            ));
        }

        // ── 3. Instructions Box ───────────────────────────────────────────────
        if let Some(instructions) = self.input.instructions {
            if !instructions.is_empty() {
                body.push_str(&format!(
                    r#"<w:tbl>
  <w:tblPr>
    <w:tblW w:w="5000" w:type="pct"/>
    <w:tblBorders>
      <w:top w:val="single" w:sz="6" w:space="0" w:color="999999"/>
      <w:left w:val="single" w:sz="6" w:space="0" w:color="999999"/>
      <w:bottom w:val="single" w:sz="6" w:space="0" w:color="999999"/>
      <w:right w:val="single" w:sz="6" w:space="0" w:color="999999"/>
    </w:tblBorders>
    <w:tblCellMar>
      <w:top w:w="140" w:type="dxa"/>
      <w:bottom w:w="140" w:type="dxa"/>
      <w:left w:w="180" w:type="dxa"/>
      <w:right w:w="180" w:type="dxa"/>
    </w:tblCellMar>
  </w:tblPr>
  <w:tr>
    <w:tc>
      <w:tcPr>
        <w:shd w:val="clear" w:color="auto" w:fill="F8F9FA"/>
      </w:tcPr>
      <w:p>
        <w:pPr><w:spacing w:after="60"/></w:pPr>
        <w:r><w:rPr><w:b/><w:sz w:val="20"/></w:rPr><w:t>INSTRUCTIONS TO CANDIDATES:</w:t></w:r>
      </w:p>
      <w:p>
        <w:pPr><w:spacing w:after="40"/></w:pPr>
        <w:r><w:rPr><w:sz w:val="20"/></w:rPr><w:t>{}</w:t></w:r>
      </w:p>
    </w:tc>
  </w:tr>
</w:tbl>
<w:p><w:pPr><w:spacing w:after="200"/></w:pPr></w:p>
"#,
                    xml_escape(instructions)
                ));
            }
        }

        // ── 4. Questions & Sections ──────────────────────────────────────────
        let mut current_section: Option<String> = None;
        let mut q_num = 1;

        for q in self.input.questions {
            // Render section header if section changed
            if let Some(sec) = &q.section {
                if current_section.as_ref() != Some(sec) {
                    current_section = Some(sec.clone());
                    body.push_str(&format!(
                        r#"<w:p>
  <w:pPr>
    <w:pStyle w:val="SectionHeader"/>
    <w:spacing w:before="240" w:after="120"/>
    <w:jc w:val="center"/>
    <w:pBdr>
      <w:bottom w:val="single" w:sz="6" w:space="4" w:color="CCCCCC"/>
    </w:pBdr>
  </w:pPr>
  <w:r>
    <w:rPr><w:b/><w:sz w:val="24"/><w:szCs w:val="24"/></w:rPr>
    <w:t>SECTION {}</w:t>
  </w:r>
</w:p>
"#,
                        xml_escape(sec)
                    ));
                }
            }

            // Question Main Body
            let body_text = render_body_text(&q.body, q.body_format);
            let marks_label = format!("[{} mark{}]", q.marks, if q.marks == 1 { "" } else { "s" });

            body.push_str(&format!(
                r#"<w:p>
  <w:pPr>
    <w:spacing w:before="160" w:after="80"/>
  </w:pPr>
  <w:r>
    <w:rPr><w:b/><w:sz w:val="22"/><w:szCs w:val="22"/></w:rPr>
    <w:t xml:space="preserve">{}.  </w:t>
  </w:r>
  <w:r>
    <w:rPr><w:sz w:val="22"/><w:szCs w:val="22"/></w:rPr>
    <w:t xml:space="preserve">{} </w:t>
  </w:r>
  <w:r>
    <w:rPr><w:b/><w:sz w:val="20"/><w:color w:val="555555"/></w:rPr>
    <w:t>{}</w:t>
  </w:r>
</w:p>
"#,
                q_num,
                xml_escape(&body_text),
                xml_escape(&marks_label)
            ));

            // Stimulus block (if present)
            if let Some(stim_json) = &q.stimulus {
                self.append_stimulus(&mut body, stim_json);
            }

            // Question Images
            for img in &q.images {
                self.append_image(&mut body, &img.key, img.caption.as_deref());
            }

            // Sub-parts (if any)
            for part in &q.parts {
                let part_body = render_body_text(&part.body, part.body_format);
                let part_marks = format!(
                    "[{} mark{}]",
                    part.marks,
                    if part.marks == 1 { "" } else { "s" }
                );

                body.push_str(&format!(
                    r#"<w:p>
  <w:pPr>
    <w:ind w:left="360"/>
    <w:spacing w:before="100" w:after="60"/>
  </w:pPr>
  <w:r>
    <w:rPr><w:b/><w:sz w:val="22"/><w:szCs w:val="22"/></w:rPr>
    <w:t xml:space="preserve">({})  </w:t>
  </w:r>
  <w:r>
    <w:rPr><w:sz w:val="22"/><w:szCs w:val="22"/></w:rPr>
    <w:t xml:space="preserve">{} </w:t>
  </w:r>
  <w:r>
    <w:rPr><w:b/><w:sz w:val="20"/><w:color w:val="555555"/></w:rPr>
    <w:t>{}</w:t>
  </w:r>
</w:p>
"#,
                    xml_escape(&part.label),
                    xml_escape(&part_body),
                    xml_escape(&part_marks)
                ));

                if let Some(stim_json) = &part.stimulus {
                    self.append_stimulus(&mut body, stim_json);
                }

                // If not marking scheme, render answer lines/boxes for part
                if !self.is_marking_scheme {
                    self.append_answer_space(
                        &mut body,
                        part.answer_space_type,
                        part.answer_lines.unwrap_or(3),
                        part.answer_box_height_mm.unwrap_or(40),
                        360,
                    );
                } else {
                    // Render rubric criteria for part
                    self.append_rubric(&mut body, &part.rubric, 720);
                }
            }

            // If main question had no parts, render answer space / rubric for main question
            if q.parts.is_empty() {
                if !self.is_marking_scheme {
                    self.append_answer_space(
                        &mut body,
                        q.answer_space_type,
                        q.answer_lines.unwrap_or(4),
                        q.answer_box_height_mm.unwrap_or(50),
                        0,
                    );
                } else {
                    self.append_rubric(&mut body, &q.rubric, 360);
                    if let Some(ea_json) = &q.example_answer {
                        self.append_example_answer(&mut body, ea_json, 360);
                    }
                }
            } else if self.is_marking_scheme {
                // If main question has general rubric/example answer
                self.append_rubric(&mut body, &q.rubric, 360);
                if let Some(ea_json) = &q.example_answer {
                    self.append_example_answer(&mut body, ea_json, 360);
                }
            }

            q_num += 1;
        }

        // ── 5. End of Paper notice ───────────────────────────────────────────
        body.push_str(
            r#"<w:p>
  <w:pPr>
    <w:jc w:val="center"/>
    <w:spacing w:before="360" w:after="120"/>
  </w:pPr>
  <w:r>
    <w:rPr><w:b/><w:i/><w:sz w:val="20"/><w:color w:val="888888"/></w:rPr>
    <w:t>— END OF PAPER —</w:t>
  </w:r>
</w:p>
"#,
        );

        // Section properties (A4 Page with 1-inch margins and footer)
        let sect_pr = r#"<w:sectPr>
  <w:footerReference w:type="default" r:id="rIdFooter1"/>
  <w:pgSz w:w="11906" w:h="16838"/>
  <w:pgMar w:top="1440" w:right="1440" w:bottom="1440" w:left="1440" w:header="720" w:footer="720" w:gutter="0"/>
</w:sectPr>"#;

        format!(
            r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"
            xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"
            xmlns:m="http://schemas.openxmlformats.org/officeDocument/2006/math"
            xmlns:wp="http://schemas.openxmlformats.org/drawingml/2006/wordprocessingDrawing"
            xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"
            xmlns:pic="http://schemas.openxmlformats.org/drawingml/2006/picture">
  <w:body>
{}
{}
  </w:body>
</w:document>"#,
            body, sect_pr
        )
    }

    fn append_stimulus(&self, body: &mut String, stim_json: &str) {
        let (label, text, caption) = match serde_json::from_str::<Stimulus>(stim_json) {
            Ok(stim) => {
                let lbl = match stim.type_ {
                    StimulusType::Passage => "PASSAGE: ",
                    StimulusType::Table => "[TABLE] ",
                    StimulusType::Graph => "[GRAPH] ",
                    StimulusType::Diagram => "[DIAGRAM] ",
                };
                (lbl, stim.body, stim.caption)
            }
            Err(_) => ("", stim_json.to_string(), String::new()),
        };

        body.push_str(&format!(
            r#"<w:tbl>
  <w:tblPr>
    <w:tblW w:w="4800" w:type="pct"/>
    <w:tblBorders>
      <w:top w:val="single" w:sz="4" w:space="0" w:color="CCCCCC"/>
      <w:left w:val="single" w:sz="12" w:space="0" w:color="888888"/>
      <w:bottom w:val="single" w:sz="4" w:space="0" w:color="CCCCCC"/>
      <w:right w:val="single" w:sz="4" w:space="0" w:color="CCCCCC"/>
    </w:tblBorders>
    <w:tblCellMar>
      <w:top w:w="120" w:type="dxa"/>
      <w:bottom w:w="120" w:type="dxa"/>
      <w:left w:w="160" w:type="dxa"/>
      <w:right w:w="160" w:type="dxa"/>
    </w:tblCellMar>
  </w:tblPr>
  <w:tr>
    <w:tc>
      <w:tcPr><w:shd w:val="clear" w:color="auto" w:fill="F9F9F9"/></w:tcPr>
      <w:p>
        <w:pPr><w:spacing w:after="40" w:before="40"/></w:pPr>
        <w:r><w:rPr><w:b/><w:sz w:val="20"/></w:rPr><w:t>{}</w:t></w:r>
        <w:r><w:rPr><w:sz w:val="20"/></w:rPr><w:t>{}</w:t></w:r>
      </w:p>
"#,
            xml_escape(label),
            xml_escape(&text)
        ));

        if !caption.is_empty() {
            body.push_str(&format!(
                r#"      <w:p>
        <w:pPr><w:spacing w:after="40" w:before="0"/></w:pPr>
        <w:r><w:rPr><w:i/><w:sz w:val="18"/><w:color w:val="666666"/></w:rPr><w:t>{}</w:t></w:r>
      </w:p>
"#,
                xml_escape(&caption)
            ));
        }

        body.push_str(
            r#"    </w:tc>
  </w:tr>
</w:tbl>
<w:p><w:pPr><w:spacing w:after="80"/></w:pPr></w:p>
"#,
        );
    }

    fn append_image(&self, body: &mut String, key: &str, caption: Option<&str>) {
        if let Some(img) = self.images.iter().find(|i| i.key == key) {
            let cx = 4500000; // ~4.7 inches width in EMUs
            let cy = 3000000; // ~3.1 inches height in EMUs

            body.push_str(&format!(
                r#"<w:p>
  <w:pPr><w:jc w:val="center"/><w:spacing w:before="80" w:after="60"/></w:pPr>
  <w:r>
    <w:drawing>
      <wp:inline distT="0" distB="0" distL="0" distR="0">
        <wp:extent cx="{}" cy="{}"/>
        <wp:docPr id="1" name="Picture"/>
        <a:graphic xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main">
          <a:graphicData uri="http://schemas.openxmlformats.org/drawingml/2006/picture">
            <pic:pic xmlns:pic="http://schemas.openxmlformats.org/drawingml/2006/picture">
              <pic:nvPicPr>
                <pic:cNvPr id="0" name="Image"/>
                <pic:cNvPicPr/>
              </pic:nvPicPr>
              <pic:blipFill>
                <a:blip r:embed="{}"/>
                <a:stretch><a:fillRect/></a:stretch>
              </pic:blipFill>
              <pic:spPr>
                <a:xfrm><a:off x="0" y="0"/><a:ext cx="{}" cy="{}"/></a:xfrm>
                <a:prstGeom prst="rect"><a:avLst/></a:prstGeom>
              </pic:spPr>
            </pic:pic>
          </a:graphicData>
        </a:graphic>
      </wp:inline>
    </w:drawing>
  </w:r>
</w:p>
"#,
                cx, cy, img.r_id, cx, cy
            ));

            if let Some(cap) = caption {
                if !cap.is_empty() {
                    body.push_str(&format!(
                        r#"<w:p>
  <w:pPr><w:jc w:val="center"/><w:spacing w:before="0" w:after="80"/></w:pPr>
  <w:r><w:rPr><w:i/><w:sz w:val="18"/><w:color w:val="555555"/></w:rPr><w:t>Figure: {}</w:t></w:r>
</w:p>
"#,
                        xml_escape(cap)
                    ));
                }
            }
        }
    }

    fn append_answer_space(
        &self,
        body: &mut String,
        space_type: u8,
        lines: i16,
        box_height_mm: i16,
        indent_dxa: i32,
    ) {
        if space_type == 0 {
            // Dotted Ruled lines
            let count = lines.max(1).min(20);
            for _ in 0..count {
                body.push_str(&format!(
                    r#"<w:p>
  <w:pPr>
    <w:ind w:left="{}"/>
    <w:spacing w:before="60" w:after="80"/>
    <w:pBdr>
      <w:bottom w:val="dotted" w:sz="6" w:space="4" w:color="999999"/>
    </w:pBdr>
  </w:pPr>
  <w:r><w:t xml:space="preserve"> </w:t></w:r>
</w:p>
"#,
                    indent_dxa
                ));
            }
        } else {
            // Working Box / Diagram Box / Grid Box
            let height_dxa = (box_height_mm as f32 * 56.7) as i32;
            let label = match space_type {
                2 => "[ Diagram Space ]",
                3 => "[ Construction Space ]",
                4 => "[ Grid / Graph Space ]",
                _ => "[ Working Space ]",
            };

            body.push_str(&format!(
                r#"<w:tbl>
  <w:tblPr>
    <w:tblW w:w="4800" w:type="pct"/>
    <w:tblBorders>
      <w:top w:val="single" w:sz="4" w:space="0" w:color="AAAAAA"/>
      <w:left w:val="single" w:sz="4" w:space="0" w:color="AAAAAA"/>
      <w:bottom w:val="single" w:sz="4" w:space="0" w:color="AAAAAA"/>
      <w:right w:val="single" w:sz="4" w:space="0" w:color="AAAAAA"/>
    </w:tblBorders>
    <w:tblCellMar>
      <w:top w:w="120" w:type="dxa"/>
      <w:bottom w:w="120" w:type="dxa"/>
      <w:left w:w="160" w:type="dxa"/>
      <w:right w:w="160" w:type="dxa"/>
    </w:tblCellMar>
  </w:tblPr>
  <w:tr>
    <w:trPr>
      <w:trHeight w:val="{}" w:hRule="atLeast"/>
    </w:trPr>
    <w:tc>
      <w:p>
        <w:pPr><w:jc w:val="center"/><w:spacing w:after="40" w:before="40"/></w:pPr>
        <w:r><w:rPr><w:i/><w:sz w:val="18"/><w:color w:val="AAAAAA"/></w:rPr><w:t>{}</w:t></w:r>
      </w:p>
    </w:tc>
  </w:tr>
</w:tbl>
<w:p><w:pPr><w:spacing w:after="80"/></w:pPr></w:p>
"#,
                height_dxa, label
            ));
        }
    }

    fn append_rubric(&self, body: &mut String, rubric: &[(String, i16, bool)], indent_dxa: i32) {
        if rubric.is_empty() {
            return;
        }

        body.push_str(&format!(
            r#"<w:p>
  <w:pPr>
    <w:ind w:left="{}"/>
    <w:spacing w:before="60" w:after="40"/>
  </w:pPr>
  <w:r><w:rPr><w:b/><w:sz w:val="20"/><w:color w:val="2E7D32"/></w:rPr><w:t>Marking Scheme / Rubric:</w:t></w:r>
</w:p>
"#,
            indent_dxa
        ));

        for (criterion, marks, req) in rubric {
            let req_tag = if *req { " [Required]" } else { "" };
            body.push_str(&format!(
                r#"<w:p>
  <w:pPr>
    <w:ind w:left="{}"/>
    <w:spacing w:before="20" w:after="20"/>
  </w:pPr>
  <w:r><w:rPr><w:sz w:val="20"/><w:color w:val="2E7D32"/></w:rPr><w:t xml:space="preserve">&#x2713;  {}</w:t></w:r>
  <w:r><w:rPr><w:b/><w:sz w:val="20"/><w:color w:val="2E7D32"/></w:rPr><w:t xml:space="preserve">  [{} mark{}]</w:t></w:r>
</w:p>
"#,
                indent_dxa + 240,
                xml_escape(&format!("{}{}", criterion, req_tag)),
                marks,
                if *marks == 1 { "" } else { "s" }
            ));
        }
    }

    fn append_example_answer(&self, body: &mut String, ea_json: &str, indent_dxa: i32) {
        let text = match serde_json::from_str::<ExampleAnswer>(ea_json) {
            Ok(ea) => match ea.format {
                ExampleAnswerFormat::Plain | ExampleAnswerFormat::Tiptap => {
                    ea.content.map(|c| strip_html(&c)).unwrap_or_default()
                }
                _ => String::new(),
            },
            Err(_) => strip_html(ea_json),
        };

        if !text.is_empty() {
            body.push_str(&format!(
                r#"<w:p>
  <w:pPr>
    <w:ind w:left="{}"/>
    <w:spacing w:before="40" w:after="80"/>
  </w:pPr>
  <w:r><w:rPr><w:b/><w:i/><w:sz w:val="20"/><w:color w:val="555555"/></w:rPr><w:t xml:space="preserve">Example Answer: </w:t></w:r>
  <w:r><w:rPr><w:i/><w:sz w:val="20"/><w:color w:val="444444"/></w:rPr><w:t>{}</w:t></w:r>
</w:p>
"#,
                indent_dxa + 240,
                xml_escape(&text)
            ));
        }
    }

    fn generate_zip(&self) -> Result<Vec<u8>, String> {
        let mut buffer = Cursor::new(Vec::new());
        let mut zip = ZipWriter::new(&mut buffer);

        let options = SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Deflated)
            .unix_permissions(0o644);

        // 1. [Content_Types].xml
        zip.start_file("[Content_Types].xml", options)
            .map_err(|e| e.to_string())?;
        zip.write_all(self.build_content_types_xml().as_bytes())
            .map_err(|e| e.to_string())?;

        // 2. _rels/.rels
        zip.start_file("_rels/.rels", options)
            .map_err(|e| e.to_string())?;
        zip.write_all(self.build_rels_xml().as_bytes())
            .map_err(|e| e.to_string())?;

        // 3. word/_rels/document.xml.rels
        zip.start_file("word/_rels/document.xml.rels", options)
            .map_err(|e| e.to_string())?;
        zip.write_all(self.build_doc_rels_xml().as_bytes())
            .map_err(|e| e.to_string())?;

        // 4. word/styles.xml
        zip.start_file("word/styles.xml", options)
            .map_err(|e| e.to_string())?;
        zip.write_all(self.build_styles_xml().as_bytes())
            .map_err(|e| e.to_string())?;

        // 5. word/settings.xml
        zip.start_file("word/settings.xml", options)
            .map_err(|e| e.to_string())?;
        zip.write_all(self.build_settings_xml().as_bytes())
            .map_err(|e| e.to_string())?;

        // 6. word/fontTable.xml
        zip.start_file("word/fontTable.xml", options)
            .map_err(|e| e.to_string())?;
        zip.write_all(self.build_font_table_xml().as_bytes())
            .map_err(|e| e.to_string())?;

        // 7. word/footer1.xml
        zip.start_file("word/footer1.xml", options)
            .map_err(|e| e.to_string())?;
        zip.write_all(self.build_footer_xml().as_bytes())
            .map_err(|e| e.to_string())?;

        // 8. word/document.xml
        zip.start_file("word/document.xml", options)
            .map_err(|e| e.to_string())?;
        zip.write_all(self.build_document_xml().as_bytes())
            .map_err(|e| e.to_string())?;

        // 9. word/media/ images
        for img in &self.images {
            let path = format!("word/media/{}", img.filename);
            zip.start_file(path, options).map_err(|e| e.to_string())?;
            zip.write_all(&img.bytes).map_err(|e| e.to_string())?;
        }

        zip.finish().map_err(|e| e.to_string())?;
        Ok(buffer.into_inner())
    }
}

// ── Public API ────────────────────────────────────────────────────────────────

/// Generate an exam / assessment paper as an editable Microsoft Word (.docx) document.
pub fn generate_paper_docx(input: &PaperPdfInput) -> Result<Vec<u8>, String> {
    let ctx = DocxContext::new(input, None, None, false);
    ctx.generate_zip()
}

/// Generate a marking scheme and rubric as an editable Microsoft Word (.docx) document.
#[allow(dead_code)]
pub fn generate_marking_scheme_docx(input: &PaperPdfInput) -> Result<Vec<u8>, String> {
    let ctx = DocxContext::new(input, None, None, true);
    ctx.generate_zip()
}

/// Generate a student exam paper with candidate details pre-filled as a (.docx) document.
#[allow(dead_code)]
pub fn generate_student_paper_docx(
    input: &PaperPdfInput,
    student_name: &str,
    student_adm: i32,
) -> Result<Vec<u8>, String> {
    let ctx = DocxContext::new(input, Some(student_name), Some(student_adm), false);
    ctx.generate_zip()
}
