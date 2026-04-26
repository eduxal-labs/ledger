## 2. Question JSON Schema

Every topic produces exactly **one JSON file**. The file path follows the convention:

```
form{N}/{subject-slug}/{topic-slug}.json
```

### 2.1 Top-Level Structure

```json
{
  "subject": "Physics",
  "curriculum": "844",
  "grade": 2,
  "topic": "Hooke's Law",
  "questions": [ ... ]
}
```

| Field | Type | Description |
|-------|------|-------------|
| `subject` | string | Human-readable subject name. Title case. Must match exactly across all files for the same subject. |
| `curriculum` | string | `"844"` or `"cbc"` |
| `grade` | integer | Form number: `1`, `2`, `3`, or `4` (8-4-4). For CBC: `7`–`12`. |
| `topic` | string | Human-readable topic name. Title case. |
| `questions` | array | Array of question objects (see §2.2). |

### 2.2 Question Object — Full Schema

```json
{
  "type": "structured",
  "difficulty": 3,
  "cognitive_level": "application",

  "stimulus": null,

  "body": "A spring has a natural length of 20 cm. A force of 6 N extends it to 32 cm.",
  "body_format": "plain",
  "marks": 9,
  "max_marks": 9,

  "answer_space_type": "lines",
  "answer_lines": 4,

  "parts": [
    {
      "label": "a",
      "body": "Calculate the spring constant.",
      "body_format": "plain",
      "marks": 3,
      "max_marks": 3,
      "answer_space_type": "lines",
      "answer_lines": 6,
      "stimulus": null,
      "rubric": [
        { "criterion": "Correct formula: k = F/e", "marks": 1 },
        { "criterion": "Correct substitution: k = 6 / 0.12", "marks": 1 },
        { "criterion": "Correct answer with units: k = 50 N/m", "marks": 1 }
      ],
      "example_answer": {
        "format": "plain",
        "content": "k = F/e = 6 / 0.12 = 50 N/m"
      }
    },
    {
      "label": "b",
      "body": "State the law that relates the extension of the spring to the applied force.",
      "body_format": "plain",
      "marks": 1,
      "max_marks": 1,
      "answer_space_type": "lines",
      "answer_lines": 2,
      "stimulus": null,
      "rubric": [
        { "criterion": "The extension of a spring is directly proportional to the applied force.", "marks": 1 },
        { "criterion": "Provided the elastic limit is not exceeded.", "marks": 0, "required": true }
      ],
      "example_answer": {
        "format": "plain",
        "content": "The extension of a spring is directly proportional to the applied force, provided the elastic limit is not exceeded."
      }
    }
  ],

  "rubric": [],
  "example_answer": null,

  "images": []
}
```

For a simple (non-structured) question:

```json
{
  "type": "definition",
  "difficulty": 1,
  "cognitive_level": "recall",

  "stimulus": null,

  "body": "Define osmosis.",
  "body_format": "plain",
  "marks": 2,
  "max_marks": 2,

  "answer_space_type": "lines",
  "answer_lines": 2,

  "rubric": [
    { "criterion": "Movement of water molecules (solvent molecules)", "marks": 1 },
    { "criterion": "From a region of lower solute concentration (higher water potential) to a region of higher solute concentration (lower water potential)", "marks": 1 },
    { "criterion": "Through a semi-permeable / selectively permeable membrane", "marks": 0, "required": true }
  ],
  "example_answer": {
    "format": "plain",
    "content": "Osmosis is the movement of water molecules from a region of lower solute concentration to a region of higher solute concentration through a semi-permeable membrane."
  },

  "images": []
}
```

### 2.3 Field Reference

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `type` | string | ✅ | Question classification. See §2.4. |
| `difficulty` | integer 1–5 | ✅ | Difficulty rating. See §2.5. |
| `cognitive_level` | string | ✅ | Bloom's taxonomy level. See §2.5. |
| `stimulus` | object or null | ✅ | Pre-question reading/viewing material. `null` if none. |
| `body` | string | ✅ | Main question text. **Replaces the old `text` field.** |
| `body_format` | string | ✅ | `"plain"` or `"tiptap"`. Use `"plain"` unless the content genuinely needs rich structure. |
| `marks` | integer | ✅ | Total marks. For structured questions, equals sum of all part marks. |
| `max_marks` | integer | ✅ | Mark cap. Usually equals `marks`. Set lower when the rubric lists more valid answers than required (e.g., "any 4 of 6 criteria"). |
| `answer_space_type` | string | ✅ | Type of answer space to print for the student. See §2.6. |
| `answer_lines` | integer | conditional | Number of ruled lines. Required when `answer_space_type` is `"lines"`. |
| `answer_box_height_mm` | integer | conditional | Box height in millimetres. Required when `answer_space_type` is `"diagram_box"`, `"construction_box"`, or `"grid_box"`. |
| `parts` | array | conditional | Sub-part objects. Present only on `type: "structured"` questions. Omit entirely on all other types. |
| `rubric` | array | ✅ | Atomic criterion objects. For structured questions this is `[]`; all criteria live inside `parts`. |
| `example_answer` | object or null | ✅ | Model answer object. `null` for structured questions (answers live inside parts). |
| `images` | array | ✅ | Image/illustration reference objects. `[]` if none. |

### 2.4 `type` Field Values

| `type` | When to use |
|--------|-------------|
| `definition` | "Define / State / Name / List" — pure recall, no sub-parts |
| `explanation` | "Explain / Describe / Discuss" — prose answer, no sub-parts |
| `calculation` | Numerical — requires formula + substitution + final answer, no sub-parts |
| `structured` | Has sub-parts (a), (b), (c)… regardless of the content type of each part |
| `experiment` | "Describe an experiment to show / verify / investigate…" |
| `data_response` | Requires reading a given table, graph, or dataset before answering |
| `diagram` | "Draw / Sketch / Label / Complete a diagram" |

### 2.5 `difficulty` and `cognitive_level` Values

| `difficulty` | Meaning |
|---|---|
| 1 | Basic recall — define, state, name, list |
| 2 | Comprehension — explain, distinguish, describe |
| 3 | Application — calculate, apply a formula to a new scenario |
| 4 | Analysis — multi-step, interpret data, evaluate |
| 5 | Synthesis — complex multi-part, design, evaluate an experiment |

| `cognitive_level` | Meaning |
|---|---|
| `recall` | Direct memory retrieval |
| `comprehension` | Understand and explain |
| `application` | Use knowledge in a new context |
| `analysis` | Break down, compare, evaluate |

### 2.6 `answer_space_type` Values and Rules

| Value | Use case | Companion field |
|-------|----------|-----------------|
| `lines` | Written prose, definitions, explanations, calculations | `answer_lines` |
| `plain_box` | Short answer not needing line guides | `answer_box_height_mm` |
| `diagram_box` | Student must draw a diagram | `answer_box_height_mm` |
| `construction_box` | Geometric construction (compass + ruler) | `answer_box_height_mm` |
| `grid_box` | Graph plotting | `answer_box_height_mm` |

**Line count rule — AI must use its own example answer to calculate:**

> Count the words in `example_answer.content`. Divide by 9 (average words per handwritten line at 8 mm ruling). Multiply by 1.3 (buffer for larger handwriting). Round up. That is `answer_lines`. Minimum value: 1.

Specific constraints:
- A 1-mark definition: almost always 1 line. Rarely 2. Never 3.
- A 3-step calculation: typically 5–7 lines.
- Never allocate fewer than 1 line for any written response.
- Do not over-allocate — schools pay per printed sheet.
- Construction answers → `construction_box` always.
- Graph plotting → `grid_box` always.
- Diagram drawing → `diagram_box` always.
- For `diagram_box`, `construction_box`, `grid_box`: `answer_box_height_mm` — simple diagram = 70, medium = 90, complex = 110.

### 2.7 The `parts` Array

For `type: "structured"` questions, sub-parts are first-class objects. Each part object has all the same answer-space, rubric, and example-answer fields as a top-level question:

| Field | Type | Description |
|-------|------|-------------|
| `label` | string | Sub-part label: `"a"`, `"b"`, `"c"`, etc. |
| `body` | string | Sub-part question text. |
| `body_format` | string | `"plain"` or `"tiptap"`. |
| `marks` | integer | Marks for this part. |
| `max_marks` | integer | Mark cap for this part. |
| `answer_space_type` | string | Per §2.6. |
| `answer_lines` or `answer_box_height_mm` | integer | Per §2.6 rules applied to this part's example answer. |
| `stimulus` | object or null | A per-part stimulus if this sub-part has its own unique table or diagram. |
| `rubric` | array | Atomic criterion objects for this part only. |
| `example_answer` | object | Model answer object for this part. Never null. |

For simple questions (no sub-parts), omit `parts` entirely — do not include an empty `"parts": []`.

### 2.8 The `stimulus` Object

Use a `stimulus` when the question (or an individual part) requires the student to read or examine something before answering.

```json
"stimulus": {
  "type": "passage",
  "body": "Full passage text here...",
  "body_format": "plain",
  "caption": "Read the following passage carefully and answer the questions that follow."
}
```

| Field | Type | Description |
|-------|------|-------------|
| `type` | string | `"passage"`, `"table"`, `"graph"`, or `"diagram"` |
| `body` | string | The stimulus content. Full passage text for `"passage"`. TipTap JSON for `"table"`. For `"graph"` and `"diagram"`, leave `body` as an empty string and use an entry in `images` with `"context": "stimulus"`. |
| `body_format` | string | `"plain"` for passages, `"tiptap"` for tables. |
| `caption` | string | The instruction shown to the student before the stimulus (e.g., "Study the table below and answer the questions that follow."). |

When to add a `stimulus`:
- English comprehension → `type: "passage"`, full passage in `body`.
- Data table given → `type: "table"`, TipTap JSON table in `body`, `body_format: "tiptap"`.
- Graph given → `type: "graph"`, image in `images` array with `"context": "stimulus"`.
- Labelled diagram given → `type: "diagram"`, image in `images` array with `"context": "stimulus"`.

The `stimulus` can appear at the question level (shared by all parts) or on an individual `part` (when only that sub-part has its own unique stimulus).

### 2.9 The `example_answer` Object

```json
"example_answer": {
  "format": "plain",
  "content": "The spring constant k = F/e = 6 / 0.12 = 50 N/m."
}
```

| Field | Type | Description |
|-------|------|-------------|
| `format` | string | `"plain"`, `"tiptap"`, `"svg"`, or `"image"`. See §4.2 for the full decision table. |
| `content` | string or null | The answer content. A text string for `"plain"`, a TipTap JSON string for `"tiptap"`, an inline SVG string for `"svg"`. Set to `null` when `format` is `"image"`. |
| `image` | object | Present only when `format` is `"image"`. Contains `filename`, `caption`, and `description`. See §4.7. |

Plain text that has no formulas, tables, or lists must always use `"plain"` — not TipTap. TipTap is only for content that genuinely requires rich structure.

### 2.10 TipTap Node Vocabulary

Only use these node types when `body_format` or `example_answer.format` is `"tiptap"`:

- `paragraph` — regular prose
- `text` with marks: `bold`, `italic`
- `mathInline { attrs: { latex: "..." } }` — inline LaTeX formula
- `mathBlock { attrs: { latex: "..." } }` — display equation
- `orderedList` / `listItem` — numbered procedure steps
- `bulletList` / `listItem` — bullet point lists
- `table` / `tableRow` / `tableHeader` / `tableCell` — data tables
- `hardBreak` — line break within a node

Plain text with no formulas, tables, or lists → always `body_format: "plain"`. Do not use TipTap merely to bold a word or add a newline.

### 2.11 Question Types to Include per Subject

For each topic, include a diverse mix across all `type` values applicable to the subject. Every topic must have at least **three distinct `type` values** represented. See §5 for the full coverage checklist.

---

## 3. Rubric Design Rules

The rubric is the most critical part of every question — it determines how the AI marking agent grades student responses.

### 3.1 The Atomic Criterion Rule

**Every criterion object awards exactly 1 mark.** No criterion may ever award more than 1 mark. No catch-all strings.

```json
{ "criterion": "Seeds are easy to handle.", "marks": 1 }
```

Multi-mark criterion objects from the old schema are **banned**. If a marking point is worth 2 marks in the KCSE scheme, split it into two 1-mark objects (one for the fact, one for the explanation/reason).

### 3.2 Enumeration Questions — List ALL Valid Answers with `max_marks`

For questions asking students to "state/name/list N items" where many scientifically valid answers exist, list **every known valid answer** as a separate 1-mark criterion object. Set `max_marks` on the question or part to cap the total awarded.

**Correct approach:**

```json
"rubric": [
  { "criterion": "Seeds are easy to handle.", "marks": 1 },
  { "criterion": "Seeds are easy to store.", "marks": 1 },
  { "criterion": "Seeds are easy to transport / are less bulky than vegetative material.", "marks": 1 },
  { "criterion": "Seeds are relatively cheap.", "marks": 1 },
  { "criterion": "Seeds can be obtained in large quantities.", "marks": 1 },
  { "criterion": "Seeds produce plants with strong tap roots.", "marks": 1 },
  { "criterion": "Seeds are free from soil-borne pests and diseases.", "marks": 1 }
],
"max_marks": 4
```

The AI marker awards 1 mark per matching criterion up to `max_marks`. Listing all valid answers ensures no correct student response is penalised. The old pattern of embedding the catch-all inside a single criterion string is **banned**:

❌ `"Any 4 of the following (1 mark each, max 4 marks): Seeds are easy to handle; Seeds are easy to store; ..."`

✅ Separate criterion objects, `max_marks: 4`.

### 3.3 The `required: true` Flag

A criterion may carry `"required": true` to indicate that the answer is **incomplete without this element**, regardless of the mark cap.

```json
{ "criterion": "Provided the elastic limit is not exceeded.", "marks": 0, "required": true }
```

- `"marks": 0` — the student earns no extra mark for including this; it is expected as part of the complete answer.
- `"required": true` — if absent, the answer is considered incomplete and the question's full marks should be withheld.

Use `required: true` for:
- Essential legal/scientific qualifiers (e.g., Hooke's Law qualifier above).
- Mandatory units on a final numerical answer when the question explicitly demands them.
- Mandatory structural elements of a definition (e.g., "through a semi-permeable membrane" in the osmosis definition).

### 3.4 Calculation Rubrics

Award 1 mark per distinct step:

```json
"rubric": [
  { "criterion": "Correct formula or correct approach: k = F/e (or F = ke rearranged)", "marks": 1 },
  { "criterion": "Correct substitution of values: k = 6 / 0.12", "marks": 1 },
  { "criterion": "Correct final answer: k = 50 N/m (units required)", "marks": 1 }
]
```

For multi-step calculations, each distinct logical step earns 1 mark. A student who makes a single arithmetic error but uses the correct method earns all marks except the final answer mark (follow-through principle). Specify "follow-through from (i)" in the criterion text when applicable.

### 3.5 Diagram Rubrics

Award 1 mark per key structure, label, or positional feature:

```json
"rubric": [
  { "criterion": "Cell membrane correctly drawn as the outermost boundary of the cell.", "marks": 1 },
  { "criterion": "Nucleus present, roughly central, with nuclear membrane clearly drawn.", "marks": 1 },
  { "criterion": "At least one mitochondrion present and correctly labelled.", "marks": 1 },
  { "criterion": "Cytoplasm filling the cell interior, correctly labelled.", "marks": 1 },
  { "criterion": "At least one other named organelle (ribosome, ER, Golgi body, lysosome) correctly labelled.", "marks": 1 }
],
"max_marks": 4
```

Specify which labels are mandatory (and warrant `required: true` if applicable) versus optional extras that earn bonus marks up to `max_marks`.

### 3.6 Structured Question Rubrics

For `type: "structured"` questions, the parent `rubric` array is **always `[]`**. All criterion objects live inside the `parts` array on their respective part objects. Never duplicate criteria between the parent and the parts.

The parent `marks` must equal the sum of all part `marks` values. The parent `max_marks` must also equal the sum of all part `max_marks` values.

### 3.7 Sum and Cap Rules

- **Without `parts`:** The sum of all criterion `marks` (excluding `required: true` criteria with `marks: 0`) must equal the question's `marks` field, OR be greater than `marks` (with `max_marks` set to cap the award at the intended total).
- **With `parts`:** Each part's criterion marks must sum to (or exceed, if capped by `max_marks`) that part's `marks`. The parent `marks` equals the sum of all part `marks`.
- `max_marks` ≤ `marks` always. If the rubric offers exactly as many criteria as required, set `max_marks` equal to `marks`.

### 3.8 Explanation and Description Rubrics

For "Explain / Describe" questions, award marks for:
1. The factual statement (what happens).
2. The scientific reason or mechanism (why / how it happens).

```json
"rubric": [
  { "criterion": "As temperature increases, the kinetic energy of particles increases.", "marks": 1 },
  { "criterion": "More particles have sufficient energy to overcome the activation energy barrier.", "marks": 1 },
  { "criterion": "Therefore the frequency of effective collisions increases, increasing the reaction rate.", "marks": 1 }
]
```

Each mark corresponds to a distinct scientific idea — never bundle two ideas into one criterion.