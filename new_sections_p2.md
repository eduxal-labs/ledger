---

## 4. Illustration and Diagram Rules

Many science and mathematics questions require visual aids. The Topic Agent must create illustrations in the correct format.

### 4.1 When Illustrations Are Needed

Illustrations are typically needed for:
- Biological diagrams (cells, organs, organ systems, organisms, experimental setups)
- Physics diagrams (circuits, ray diagrams, force diagrams, experimental apparatus)
- Chemistry diagrams (apparatus setups, molecular structures, lab equipment)
- Mathematics diagrams (geometric figures, graphs, coordinate planes, constructions)
- Geography diagrams (maps, cross-sections, weather charts, landform diagrams)
- Agriculture diagrams (farm tools, soil profiles, crop layouts, irrigation systems)

### 4.2 `example_answer` Format Decision

Choose the correct `format` value for each answer type:

| Scenario | `format` | `content` field |
|----------|----------|-----------------|
| Prose answer (definitions, explanations, lists) | `"plain"` | Text string |
| Answer with formulas, tables, or procedure steps | `"tiptap"` | TipTap JSON string |
| Geometric construction (compass + ruler) | `"svg"` | Inline SVG string with exact arcs, lines, angles, all construction marks visible |
| Physics exact diagram (force diagram, ray diagram, circuit) | `"svg"` | Inline SVG string |
| Biological organ or cell diagram | `"image"` | `null` — use `image` sub-object instead |
| Chemistry apparatus setup | `"image"` | `null` — use `image` sub-object instead |

**SVG answers must be precise** — correct angles, correct proportions, all construction arcs visible, all labels present. An SVG answer for a geometric construction must show every arc drawn by the compass, not just the final figure.

### 4.3 Format Decision: SVG vs. Image Reference (for question illustrations)

| Use SVG (create actual file) | Use image reference (described, for future generation) |
|------------------------------|--------------------------------------------------------|
| Geometric shapes and constructions | Complex biological structures (heart cross-section, kidney, cell under microscope) |
| Simple circuit diagrams | Microscope views of tissues or cells |
| Graphs and coordinate planes | Photographs of specimens or apparatus |
| Force and vector diagrams | Complex 3D molecular structures |
| Simple apparatus setups | Detailed anatomical cross-sections |
| Flowcharts, bar charts, pie charts | Maps and geographical features requiring artistic rendering |
| Basic chemical structure diagrams | Complex multi-component experimental setups |

**Rule of thumb:** If the illustration can be represented clearly with lines, shapes, and text labels using geometric primitives → **SVG**. If it requires complex organic curves, shading, textures, or photorealistic detail → **image reference** (the `description` field serves as the spec for future AI image generation).

### 4.4 SVG Files for Question Illustrations

**File location:** `form{N}/{subject-slug}/illustrations/{filename}.svg`

**Requirements:**
- Clean, minimal SVG — no embedded raster images.
- Black lines on white or transparent background (prints clearly).
- Use `<text>` elements for labels with `font-family="sans-serif"`.
- `font-size` 14–18px for readability at print scale.
- Include a `viewBox` attribute for proper scaling.
- Leader lines from labels to the structures they identify.
- Maximum width: 800px equivalent. Keep diagrams focused and uncluttered.
- No external dependencies (no linked stylesheets or external fonts).

**Naming convention:** kebab-case, descriptive. Examples:
- `right-angled-triangle-pythagoras.svg`
- `simple-series-circuit.svg`
- `beam-balance-moments.svg`
- `dandelion-osmosis-experiment.svg`

### 4.5 Image Reference Object (`images` array)

```json
{
  "context": "question",
  "filename": "/home/abdihakim/Documents/GITHUB/eduxal-labs/questions/form2/physics/illustrations/hookes-law-setup.svg",
  "caption": "Figure 1: Experimental setup for Hooke's Law investigation",
  "description": "A diagram showing a retort stand with a spring hanging vertically from a clamp. A mass holder with slotted masses is attached to the lower end of the spring. A metre rule is positioned vertically alongside the spring. Labels point to: retort stand, clamp, spring, mass holder, metre rule, and the extension e."
}
```

| Field | Type | Description |
|-------|------|-------------|
| `context` | string | `"question"` — shown with the question text; `"answer"` — shown in the marking guide; `"stimulus"` — shown as the stimulus material the student reads/examines. |
| `filename` | string | **Full OS absolute filesystem path.** Always begins with `/home/abdihakim/Documents/GITHUB/eduxal-labs/questions/form{N}/{subject-slug}/illustrations/{name}.svg`. Never use project-root-relative or bare filenames. |
| `caption` | string | Figure caption shown to the student. Numbered sequentially within the question (Figure 1, Figure 2, …). |
| `description` | string | Detailed description — sufficient for a human illustrator or AI image generator to recreate the image from scratch. Also serves as alt-text for accessibility. |

> **⚠️ Absolute paths only.** The `filename` field must always begin with `/home/abdihakim/Documents/GITHUB/eduxal-labs/questions/`. Never use relative paths (`form2/physics/...`) or bare filenames (`hookes-law-setup.svg`). The client app reads this path to locate the file for upload to S3.

### 4.6 `context` Values for Images

| `context` value | When to use |
|-----------------|-------------|
| `"question"` | The image accompanies the question text — the student must study it to answer. |
| `"answer"` | The image is part of the model answer — shown in the marking guide. Used for "Draw a diagram of…" questions where the answer diagram is provided. |
| `"stimulus"` | The image IS the stimulus — a graph, labelled diagram, or photograph the student is asked to interpret. The question's `stimulus.type` should be `"graph"` or `"diagram"` and `stimulus.body` should be empty. |

### 4.7 SVG Example Answers

When `example_answer.format` is `"svg"`, the `content` field holds the complete SVG markup as a JSON string:

```json
"example_answer": {
  "format": "svg",
  "content": "<svg viewBox='0 0 300 200' xmlns='http://www.w3.org/2000/svg'><line x1='150' y1='10' x2='150' y2='190' stroke='black' stroke-width='1.5'/><circle cx='150' cy='100' r='5' fill='black'/><text x='160' y='104' font-family='sans-serif' font-size='14'>O</text></svg>"
}
```

SVG answer requirements:
- Self-contained and printable (black on transparent/white background).
- All geometry must be accurate — use trigonometry to compute coordinates when necessary.
- For constructions: show ALL compass arcs, not just the final result.
- Labels must be present with `<text>` elements, not embedded as image text.

### 4.8 Image Example Answers

When `example_answer.format` is `"image"`, use the `image` sub-object:

```json
"example_answer": {
  "format": "image",
  "content": null,
  "image": {
    "filename": "/home/abdihakim/Documents/GITHUB/eduxal-labs/questions/form2/biology/illustrations/human-heart-labelled.svg",
    "caption": "Correctly labelled diagram of the human heart (vertical section)",
    "description": "A vertical cross-section of the human heart showing: right atrium, right ventricle, left atrium, left ventricle, pulmonary artery, pulmonary veins, aorta, superior and inferior vena cava, tricuspid valve, bicuspid (mitral) valve, semi-lunar valves of aorta and pulmonary artery, chordae tendineae, papillary muscles, and the septum. The right side of the heart should be shaded to indicate deoxygenated blood and the left side left unshaded for oxygenated blood."
  }
}
```

---

## 5. Quality Standards

### 5.1 Question Quality Checklist

Every question MUST:
- [ ] Be **original** — not copied verbatim from any past paper (inspired by past-paper style and depth is fine).
- [ ] Be **self-contained** — all information needed to answer is in the `body`, `stimulus`, or referenced diagram.
- [ ] Have **correct marks allocation**:
  - Questions without `parts`: `marks` equals the sum of all criterion `marks` values in `rubric` (excluding `required: true` criteria with `marks: 0`), OR the rubric intentionally offers more criteria than `marks` (in which case `max_marks` caps the award).
  - Questions with `parts`: each part's `marks` equals its own criterion sum; parent `marks` equals the sum of all part `marks`.
- [ ] Have a **complete model answer** — `example_answer` object with appropriate `format` and non-null `content` (unless `format` is `"image"`). Never `null` for non-structured questions.
- [ ] Use **proper English** — clear, grammatically correct, unambiguous.
- [ ] Be **grade-appropriate** — content matches the specific form level, not lower or higher.
- [ ] Be **curriculum-aligned** — covers content actually in the KNEC syllabus for that form and subject.
- [ ] Have `type`, `difficulty`, and `cognitive_level` **accurately set** — these must reflect genuine assessment judgement, not just heuristic output.
- [ ] Have `answer_space_type` set correctly, with `answer_lines` calculated from the word count of `example_answer.content` using the rule in §2.6.
- [ ] Use `body_format: "tiptap"` **only** when the content genuinely requires rich structure (formulas, tables, procedure lists). Plain prose → always `"plain"`.
- [ ] Include a `stimulus` object wherever the question references a passage, table, or graph the student must read/interpret.

### 5.2 Topic Coverage Checklist

For every topic, the question set MUST:
- [ ] Cover **every sub-topic** within the topic — definitions, processes, applications, experiments, diagram work, comparisons, calculations.
- [ ] Include questions at **multiple difficulty levels** — at minimum difficulty 1, 2, and 3.
- [ ] Include at least **three distinct `type` values** represented (e.g., `definition`, `calculation`, `structured`).
- [ ] Include `stimulus`-bearing questions where applicable (`data_response`, comprehension, graph-reading).
- [ ] Include questions requiring illustrations where the topic involves visual concepts (diagrams, apparatus, structures, graphs).
- [ ] Have at least **30 questions** (or demonstrate the topic is genuinely exhausted).
- [ ] **Not duplicate** questions — each question tests a distinct concept, angle, or application.

### 5.3 Scientific Accuracy

- All definitions must match KNEC-accepted definitions (use marking schemes from `../kcse-past-papers/` as the authority).
- Numerical values in calculations must use correct constants and units.
- Chemical equations must be balanced.
- Biological processes must follow the correct sequence and use correct terminology.
- Physical laws must be stated with all conditions (e.g., Hooke's Law: "provided the elastic limit is not exceeded").

### 5.4 KCSE Mark Allocation Conventions

Follow KCSE standard mark allocations:

**Science and Mathematics:**
- **Simple definition or state**: 1 mark
- **Explain with reason**: 2 marks (1 for the fact, 1 for the reason)
- **Distinguish between X and Y**: 2 marks (1 per concept)
- **List / Name / State N items**: 1 mark each (total = N); set `max_marks` = N and list all valid answers in rubric
- **Describe a process**: 3–5 marks depending on number of steps
- **Describe an experiment**: 4–8 marks (setup + procedure + observation + conclusion)
- **Calculations**: 2–4 marks (formula + substitution + answer with units)
- **Multi-part structured**: 8–20 marks total, broken into parts

**English Language (follows KCSE Paper 1/2/3 conventions):**
- **Cloze test**: 10 marks (10 blanks × 1 mark each)
- **Comprehension passage**: 20 marks total (7–8 sub-questions)
- **Poetry analysis**: 20 marks total (6–7 sub-questions)
- **Oral narrative analysis**: 20 marks total (6–7 sub-questions)
- **Functional writing**: 20 marks (format + content + language)
- **Creative composition**: 20 marks (communication + language + organisation + mechanics)
- **Grammar — rewrite as instructed**: 1 mark per sentence
- **Grammar — word formation**: 1 mark per blank

---

## 6. Directory Structure

```
questions/
├── AGENT.md                              # This file
├── TASKS.md                              # Task list (generated by Examiner or reset by migration plan)
├── migrate_schema.py                     # One-time structural migration script (Phase 1)
├── scripts/                              # Utility scripts
├── form1/
│   ├── biology/
│   │   ├── cell-physiology.json          # Topic question bank (v2 schema)
│   │   ├── the-cell.json
│   │   ├── illustrations/
│   │   │   ├── animal-cell-diagram.svg
│   │   │   └── microscope-setup.svg
│   │   └── ...
│   ├── mathematics/
│   │   ├── natural-numbers.json
│   │   ├── illustrations/
│   │   └── ...
│   ├── physics/
│   ├── chemistry/
│   └── ...
├── form2/
├── form3/
└── form4/
```

### 6.1 Naming Conventions

| Element | Convention | Example |
|---------|-----------|---------|
| Form directory | `form{N}` (no hyphen, no space) | `form1`, `form2`, `form3`, `form4` |
| Subject directory | kebab-case, lowercase | `biology`, `mathematics`, `business-studies` |
| Topic JSON file | kebab-case, lowercase | `cell-physiology.json`, `acids-bases-and-indicators.json` |
| Illustrations directory | `illustrations/` inside each subject directory | `form2/physics/illustrations/` |
| SVG files | kebab-case, descriptive | `series-circuit-two-bulbs.svg`, `human-heart-cross-section.svg` |

### 6.2 Subject Name Mapping

Use these exact subject names and directory slugs consistently:

| Subject Name (in JSON) | Directory Slug | Notes |
|------------------------|---------------|-------|
| Biology | `biology` | |
| Mathematics | `mathematics` | Never "maths" or "math" |
| Physics | `physics` | |
| Chemistry | `chemistry` | |
| English | `english` | |
| Kiswahili | `kiswahili` | |
| History and Government | `history` | Directory uses short form |
| Geography | `geography` | |
| Agriculture | `agriculture` | |
| Business Studies | `business-studies` | |
| Computer Studies | `computer-studies` | |
| CRE | `cre` | Christian Religious Education |
| IRE | `ire` | Islamic Religious Education |
| Home Science | `home-science` | |

### 6.3 JSON Schema Version Note

All topic JSON files must conform to the **v2 schema** after Phase 1 migration:
- `body` (not `text`)
- `type`, `difficulty`, `cognitive_level` present on every question
- `example_answer` is an object `{ format, content }` (not a bare string)
- `rubric` uses atomic 1-mark criteria (no catch-all strings)
- `max_marks` present on every question and part
- `answer_space_type` present on every question and part
- `parts` array present on `type: "structured"` questions

Any file not yet migrated through `migrate_schema.py` should be treated as pending Phase 1. Do not manually hand-edit files to the v2 schema — run the migration script instead.

---
