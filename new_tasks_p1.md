# TASKS.md — Schema Migration and Content Improvement

## Overview

This file tracks the two-phase plan for migrating and improving the EduXal question bank.

**Phase 1 (Task 01):** Run `migrate_schema.py` — a one-time structural migration that transforms all 587 JSON files from the old schema (`text`, bare-string `example_answer`, catch-all rubric strings) to the new v2 schema (`body`, object `example_answer`, atomic rubric criteria, `type`, `difficulty`, `cognitive_level`, `answer_space_type`, `parts`, `max_marks`). This task runs ONCE across all files before any Phase 2 work begins.

**Phase 2 (Tasks 02–57):** Content improvement passes — one agent per form × subject combination reviews, fact-checks, upgrades, and expands every topic JSON file in that slot. All Phase 2 tasks are parallel with each other; they only depend on Task 01 completing first.

**Rule:** A task block's presence in this file means it is pending. When an agent completes a task, it deletes the entire block (from `### Task XX:` through the trailing `---` separator) and commits.

---

### Task 01: Run Schema Migration Script
**Phase:** 1 — Structural Migration
**Files to create:** `migrate_schema.py` (in the questions root directory)
**Depends on:** Nothing — run this first
**Parallel group:** None (must complete before all Phase 2 tasks)

**Specification:**

Write a Python 3 script at `migrate_schema.py` (questions root directory) that migrates every question JSON file from the old schema to the new v2 schema. The script must:

**Input/Output:** Read every `form{N}/{subject}/{topic}.json` file in the questions directory (excluding `scripts/`, `.git/`, and any non-question JSON). Overwrite each file in place with the migrated content.

**Per-question transformations:**

1. **Rename `text` → `body`**. Add `"body_format": "plain"` on the same question object.

2. **Convert `example_answer`** from a bare string to an object:
   `"example_answer": "foo"` → `"example_answer": { "format": "plain", "content": "foo" }`
   If `example_answer` is already an object, leave it unchanged.

3. **Auto-tag `type`** by heuristic (apply in this priority order):
   - If `body` contains `\n(a)` or `\n(b)` patterns → `"structured"`
   - Else if `body` lowercased starts with any of: `define`, `state`, `name`, `list`, `give`, `identify`, `mention` → `"definition"`
   - Else if `body` lowercased contains any of: `calculate`, `find`, `determine`, `compute`, `how many`, `what is the value`, `what is the mass`, `what is the force`, `what is the speed`, `what is the rate` → `"calculation"`
   - Else if `body` lowercased contains `describe an experiment` or `design an experiment` or `plan an experiment` → `"experiment"`
   - Else if `body` lowercased contains any of: `draw`, `sketch`, `label`, `complete the diagram`, `construct a diagram` → `"diagram"`
   - Else if `body` lowercased contains any of: `table`, `graph`, `figure`, `data`, `results show`, `the following data` → `"data_response"`
   - Else → `"explanation"`

4. **Set `difficulty`** from `marks`:
   - `marks == 1` → `difficulty: 1`
   - `marks == 2` or `marks == 3` → `difficulty: 2`
   - `marks == 4` or `marks == 5` → `difficulty: 3`
   - `marks >= 6` and `marks <= 8` → `difficulty: 4`
   - `marks >= 9` → `difficulty: 5`

5. **Set `cognitive_level`** from the auto-tagged `type`:
   - `definition` → `"recall"`
   - `explanation` → `"comprehension"`
   - `calculation` → `"application"`
   - `structured` → `"application"`
   - `experiment` → `"application"`
   - `data_response` → `"analysis"`
   - `diagram` → `"recall"`

6. **Set `stimulus: null`** on every question (the Phase 2 agent will add real stimulus objects where needed).

7. **Set `max_marks`** equal to `marks` on every question (Phase 2 agents will lower it where the rubric lists more valid answers than required).

8. **Set `answer_space_type`** and `answer_lines` or `answer_box_height_mm`:
   - If `type` is `"diagram"` → `"answer_space_type": "diagram_box"`, `"answer_box_height_mm": 90`
   - Else if `type` is `"calculation"` → `"answer_space_type": "lines"`, compute `answer_lines` from example_answer word count
   - Else → `"answer_space_type": "lines"`, compute `answer_lines` from example_answer word count
   - Word count formula: `ceil(word_count(example_answer_content) / 9 * 1.3)`, minimum 1

9. **Atomize catch-all rubric criteria** containing the pattern `"Any N of the following"`:
   - Detect criterion text matching: `r'any\s+(\d+)\s+of\s+the\s+following'` (case-insensitive)
   - Extract N from the match → set `max_marks` on the question/part to N
   - Parse the criterion text for individual answer items (split on `;` or newlines)
   - For each item, create a new criterion object: `{ "criterion": item.strip(), "marks": 1 }`
   - Replace the single catch-all criterion with the list of atomic criteria
   - If the parsing produces fewer than 2 items (garbled text), leave the criterion unchanged and log a warning

10. **Split sub-parts for `type: "structured"`**:
    - Detect `\n(a)`, `\n(b)`, `\n(c)` etc. in `body`
    - Split `body` into: parent preamble (text before `\n(a)`) and individual parts
    - For each part, create a part object with:
      - `"label"`: `"a"`, `"b"`, `"c"` etc.
      - `"body"`: the text of that sub-part (stripped)
      - `"body_format": "plain"`
      - `"marks"`: attempt to detect mark value from text like `(2 marks)` or `(3 marks)` at end of part text; if not found, distribute `marks` evenly across parts
      - `"max_marks"`: same as `marks` for this part
      - `"answer_space_type": "lines"`
      - `"answer_lines"`: 4 (default; Phase 2 agent will correct)
      - `"stimulus": null`
      - `"rubric"`: attempt to assign rubric criteria prefixed with `(a)`, `(b)` etc. to the matching part; unmatched criteria go on the first part
      - `"example_answer"`: attempt to split example_answer content by `(a)`, `(b)` etc. prefixes; unmatched content goes on the first part
    - Set the parent `"rubric": []` and `"example_answer": null`
    - Set the parent `body` to just the preamble text (before the first sub-part)

11. **Add `"images": []`** if the field is missing.

**Script behaviour:**
- Process every `.json` file recursively under the questions root (skip `scripts/`, `.git/`, `migrate_schema.py` itself)
- Validate output JSON before writing (if validation fails, skip the file and log an error)
- Print a final summary: files processed, questions migrated, sub-parts split, rubric criteria atomized, files skipped due to errors
- Do not crash on a single bad file — use try/except per file and continue

**Run command (from questions root):**
```
python3 migrate_schema.py
```

**After completion:**
- [ ] Mark this task `[x]` in TASKS.md
- [ ] git commit: `chore: run Phase 1 schema migration across all 587 topic files`

---

### Task 02: Form 1 Agriculture — Content Improvement
**Files to modify:** `form1/agriculture/*.json`
**Files in scope:**
- `agricultural-economics-i-basic-concepts-and-farm-records.json`
- `crop-production-i.json`
- `crop-production-i-land-preparation.json`
- `factors-influencing-agriculture.json`
- `farm-tools-and-equipment.json`
- `introduction-to-agriculture.json`
- `land-reclamation.json`
- `livestock-production-i-common-breeds.json`
- `soil-composition.json`
- `soil-fertility-i.json`
- `soil-fertility-i-organic-manures.json`
- `soil-formation.json`
- `soil-profile.json`
- `water-in-the-soil.json`
- `water-supply-irrigation-and-drainage.json`
**Depends on:** Task 01
**Parallel group:** P2

**Specification:**
Read `AGENT.md` in full before starting. For every topic JSON file listed above:
1. Verify factual accuracy of all question text, rubric criteria, and example answers against Form 1 Agriculture KCSE syllabus content. Correct any errors.
2. Ensure every question is strictly Form 1 scope — remove or flag questions that belong to Form 2+ Agriculture.
3. Add `stimulus` objects where a question references a table, diagram, or dataset not yet structured.
4. Upgrade `body_format` to `"tiptap"` where content contains formulas or procedure lists genuinely requiring rich structure.
5. Upgrade `example_answer.format` to `"svg"` for any farm layout, soil profile cross-section, or tool diagram that can be precisely rendered in SVG. Create the SVG file in `form1/agriculture/illustrations/`.
6. Review `answer_lines` values — correct any that are clearly too high or too low using the word-count rule from §2.6.
7. Verify `type`, `difficulty`, and `cognitive_level` are accurately set (not just the migration heuristic).
8. Ensure each topic has at minimum 30 high-quality questions. Add original KCSE-aligned questions for any topic below 30 or with coverage gaps.
9. Ensure the rubric for every question uses atomic 1-mark criteria — fix any remaining catch-all strings the migration script may have failed to atomize correctly.
10. Consult `../materials/form-1/agriculture/` and `../kcse-past-papers/2016/41/` through `../kcse-past-papers/2024/41/` for accuracy checks and topic coverage gaps.

**Update after completion:**
- [ ] Mark this task `[x]`
- [ ] git commit: `fix(agriculture): content improvement pass Form 1`

---

### Task 03: Form 1 Biology — Content Improvement
**Files to modify:** `form1/biology/*.json`
**Files in scope:**
- `cell-physiology.json`
- `classification-i.json`
- `excretion-and-homeostasis.json`
- `introduction-to-biology.json`
- `nutrition-in-plants-and-animals.json`
- `respiration-and-gaseous-exchange.json`
- `the-cell.json`
- `transport-in-plants-and-animals.json`
**Depends on:** Task 01
**Parallel group:** P2

**Specification:**
Read `AGENT.md` in full before starting. For every topic JSON file listed above:
1. Verify factual accuracy of all questions, rubric criteria, and example answers against Form 1 Biology content (KCSE syllabus). Correct any errors.
2. Ensure Form 1 scope only — The Cell (Form 1) covers cell structure and basic function, not cell physiology or osmosis experiments (those belong to Form 2).
3. Add `stimulus` objects for questions referencing experimental diagrams (osmosis, photosynthesis setups), data tables, or passages.
4. Upgrade `example_answer.format` to `"svg"` for cell diagrams, experimental setups (osmosis apparatus, photosynthesis apparatus), food web diagrams, and simple biological diagrams. Create SVG files in `form1/biology/illustrations/`.
5. Upgrade `example_answer.format` to `"image"` for complex biological structures (cross-section of the leaf, mitochondria ultrastructure, heart). Add image references with detailed `description` fields.
6. Review `answer_lines` and correct using the word-count formula.
7. Verify `type`, `difficulty`, `cognitive_level` are accurately set.
8. Ensure each topic has at minimum 30 questions. Add questions covering: experiment design, diagram labelling, data interpretation, and application/scenario questions.
9. Fix any atomization errors in rubric criteria.
10. Consult `../materials/form-1/biology/` and KCSE past papers for accuracy and coverage.

**Update after completion:**
- [ ] Mark this task `[x]`
- [ ] git commit: `fix(biology): content improvement pass Form 1`

---

### Task 04: Form 1 Business Studies — Content Improvement
**Files to modify:** `form1/business-studies/*.json`
**Files in scope:**
- `business-and-its-environment.json`
- `entrepreneurship.json`
- `factors-of-production.json`
- `introduction-to-business-studies.json`
- `office-practice.json`
- `production.json`
- `satisfaction-of-human-wants.json`
- `the-office.json`
**Depends on:** Task 01
**Parallel group:** P2

**Specification:**
Read `AGENT.md` in full before starting. For every topic JSON file listed above:
1. Verify factual accuracy of all questions, rubric criteria, and example answers against Form 1 Business Studies syllabus content.
2. Ensure Form 1 scope only — do not include Form 2 topics (communication, insurance, trade).
3. Add `stimulus` objects for questions referencing business scenarios, office layouts, or case studies not yet structured.
4. Upgrade `body_format` to `"tiptap"` for questions with tables (e.g., comparison tables for factors of production).
5. Verify `type`, `difficulty`, `cognitive_level` — Business Studies questions are predominantly `definition`, `explanation`, and `structured`; ensure appropriate distribution.
6. Review `answer_lines` values and correct using the word-count formula.
7. Ensure each topic has at minimum 30 questions. Business Studies topics often have fewer — expand with application/scenario questions, case study analysis, and comparison questions.
8. Fix any atomization errors in rubric criteria.
9. Consult `../materials/form-1/business-studies/` and KCSE past papers for accuracy and coverage.

**Update after completion:**
- [ ] Mark this task `[x]`
- [ ] git commit: `fix(business-studies): content improvement pass Form 1`

---

### Task 05: Form 1 Chemistry — Content Improvement
**Files to modify:** `form1/chemistry/*.json`
**Files in scope:**
- `acids-bases-and-indicators.json`
- `air-and-combustion.json`
- `atomic-structure-and-chemical-formulae.json`
- `chemical-families-patterns-and-trends.json`
- `introduction-to-chemistry.json`
- `laboratory-apparatus-and-safety.json`
- `particulate-nature-of-matter.json`
- `separation-of-mixtures.json`
- `simple-classification-of-substances.json`
- `states-of-matter.json`
- `structure-and-bonding.json`
- `structure-of-the-atom-and-the-periodic-table.json`
- `water-and-hydrogen.json`
**Depends on:** Task 01
**Parallel group:** P2

**Specification:**
Read `AGENT.md` in full before starting. For every topic JSON file listed above:
1. Verify factual accuracy — chemical equations must be balanced, atomic numbers and masses must be correct, electronic configurations must match the KCSE syllabus (up to atomic number 20).
2. Ensure Form 1 scope — Atomic Structure at Form 1 covers protons/neutrons/electrons, isotopes, and simple electronic configurations; it does not cover sub-shells or quantum numbers.
3. Add `stimulus` objects for questions providing data tables (atomic masses, periodic table excerpts, titration results).
4. Upgrade `example_answer.format` to `"svg"` for: laboratory apparatus setups (filtration, distillation, chromatography, gas collection), atomic structure diagrams (dot-and-cross, Bohr models), and simple periodic table excerpts. Create SVG files in `form1/chemistry/illustrations/`.
5. Upgrade `example_answer.format` to `"tiptap"` for answers with chemical equations or formulae.
6. Review `answer_lines` values and correct using the word-count formula.
7. Verify `type`, `difficulty`, `cognitive_level` — Chemistry should have good representation of `calculation` (relative atomic mass, formula mass) and `experiment` types.
8. Ensure each topic has at minimum 30 questions with diverse types.
9. Fix atomization errors in rubric criteria. Particularly check enumeration questions (e.g., "state properties of acids") — these must list all valid answers as separate criteria with `max_marks`.
10. Consult `../materials/form-1/chemistry/` and KCSE past papers for accuracy and coverage.

**Update after completion:**
- [ ] Mark this task `[x]`
- [ ] git commit: `fix(chemistry): content improvement pass Form 1`

---

### Task 06: Form 1 Computer Studies — Content Improvement
**Files to modify:** `form1/computer-studies/*.json`
**Files in scope:**
- `computer-care-and-safety.json`
- `computer-hardware.json`
- `computer-memory.json`
- `computer-software.json`
- `computer-systems.json`
- `data-representation.json`
- `information-processing-cycle.json`
- `input-devices.json`
- `introduction-to-computer-studies.json`
- `operating-systems.json`
- `output-devices.json`
- `the-system-unit.json`
- `word-processing.json`
**Depends on:** Task 01
**Parallel group:** P2

**Specification:**
Read `AGENT.md` in full before starting. For every topic JSON file listed above:
1. Verify factual accuracy — memory capacities, binary/denary conversions, device classifications, and software definitions must be correct per the KCSE Computer Studies syllabus.
2. Ensure Form 1 scope — data representation at Form 1 covers binary, denary, and basic ASCII; it does not cover hexadecimal, BCD, or other advanced coding schemes (those are Form 2/3).
3. Add `stimulus` objects for questions referencing screenshots, system diagrams, or data tables.
4. Upgrade `example_answer.format` to `"svg"` for: computer system block diagrams (input-process-output), flowcharts, memory hierarchy diagrams. Create SVG files in `form1/computer-studies/illustrations/`.
5. Upgrade `example_answer.format` to `"tiptap"` for answers with binary conversions shown step-by-step.
6. Review `answer_lines` values and correct.
7. Verify `type`, `difficulty`, `cognitive_level` — include `calculation` questions for binary/denary conversion and `diagram` questions for system diagrams.
8. Ensure each topic has at minimum 30 questions.
9. Fix atomization errors in rubric criteria.
10. Consult `../materials/form-1/computer-studies/` and KCSE past papers for coverage.

**Update after completion:**
- [ ] Mark this task `[x]`
- [ ] git commit: `fix(computer-studies): content improvement pass Form 1`

---

### Task 07: Form 1 CRE — Content Improvement
**Files to modify:** `form1/cre/*.json`
**Files in scope:**
- `african-concept-of-god-spirits-and-ancestors.json`
- `african-moral-and-cultural-values.json`
- `creation-and-the-fall-of-man.json`
- `faith-and-gods-promises-abraham.json`
- `introduction-to-christian-religious-education.json`
- `leadership-in-israel-david-and-solomon.json`
- `loyalty-to-god-elijah.json`
- `sinai-covenant.json`
- `the-bible.json`
**Depends on:** Task 01
**Parallel group:** P2

**Specification:**
Read `AGENT.md` in full before starting. For every topic JSON file listed above:
1. Verify theological accuracy — biblical references, names, events, and places must be accurate per the KCSE CRE syllabus and standard Bible translations (KJV/NIV as reference). Cross-check with `../materials/form-1/cre/`.
2. Ensure Form 1 scope — Old Testament content as specified in the Form 1 CRE syllabus.
3. Add `stimulus` objects for passage-based questions that quote Bible verses or provide narrative excerpts for analysis.
4. CRE is text-based; illustrations are rarely needed. Only add SVG illustrations if a question requires a map or timeline (e.g., journey of the Israelites).
5. Review `answer_lines` values and correct.
6. Verify `type`, `difficulty`, `cognitive_level` — CRE questions are predominantly `definition`, `explanation`, and `structured`. Include application questions relating biblical teachings to contemporary Kenyan life.
7. Ensure each topic has at minimum 30 questions, including questions asking students to relate lessons to modern life.
8. Fix atomization errors in rubric criteria — CRE enumeration questions (e.g., "state ways in which David showed his faith") must list all valid answers.
9. Consult KCSE CRE past papers for question style and coverage.

**Update after completion:**
- [ ] Mark this task `[x]`
- [ ] git commit: `fix(cre): content improvement pass Form 1`

---

### Task 08: Form 1 English — Content Improvement
**Files to modify:** `form1/english/*.json`
**Files in scope:**
- `adjectives.json`
- `adverbs.json`
- `comprehension-skills.json`
- `conjunctions.json`
- `creative-writing.json`
- `debate-and-public-speaking.json`
- `etiquette-and-courteous-language.json`
- `functional-writing.json`
- `informal-letters.json`
- `introduction-to-oral-literature-and-oral-narratives.json`
- `minimal-pairs-and-homophones.json`
- `nouns.json`
- `oral-poetry-and-songs.json`
- `phrases-and-sentence-structure.json`
- `prepositions.json`
- `pronouns.json`
- `pronunciation-of-consonant-sounds.json`
- `pronunciation-of-vowel-sounds-and-diphthongs.json`
- `punctuation-and-capitalization.json`
- `reading-skills-dictionary-use-and-library-skills.json`
- `short-forms-of-oral-literature.json`
- `silent-letters.json`
- `spelling-rules.json`
- `summary-and-note-making.json`
- `verbs-and-tenses.json`
**Depends on:** Task 01
**Parallel group:** P2

**Specification:**
Read `AGENT.md` in full, especially §11.6 (English Language Question Formats), before starting.
1. English questions must follow the passage-based format conventions in §11.6. Verify that comprehension questions embed the full passage in `body`. Verify cloze tests have exactly 10 blanks labelled (a)–(j). Verify oral narrative questions include the full story text.
2. Add `stimulus` objects for passage-based questions: comprehension passages → `stimulus.type: "passage"`, cloze tests → `stimulus.type: "passage"`, dialogue analysis → `stimulus.type: "passage"`.
3. Grammar questions (rewrite as instructed, word formation, phrasal verbs) should use `type: "structured"` with each instruction as a separate part.
4. For creative writing and functional writing: verify prompts are realistic and detailed; rubric must use descriptive marking criteria (content/language/organisation/mechanics), not point-by-point criteria.
5. Verify `type`, `difficulty`, `cognitive_level` — English has high diversity; ensure all type values are represented.
6. Review `answer_lines` — note that writing tasks (composition, functional writing) should use `answer_space_type: "plain_box"` with a generous `answer_box_height_mm` (140–180 mm) since students write full essays.
7. Ensure each topic has at minimum 30 questions. English topics can sustain many more — comprehension skills and grammar topics should have 40+.
8. Fix atomization errors. Note: English rubrics for open-ended writing use holistic descriptors — these should NOT be atomized into 1-mark criteria; keep them as descriptive band descriptors.
9. Consult `../materials/form-1/english/` and KCSE English past papers (Paper 1, Paper 2) for question style and marking scheme conventions.

**Update after completion:**
- [ ] Mark this task `[x]`
- [ ] git commit: `fix(english): content improvement pass Form 1`

---

### Task 09: Form 1 Geography — Content Improvement
**Files to modify:** `form1/geography/*.json`
**Files in scope:**
- `field-work.json`
- `introduction-to-geography.json`
- `minerals-and-rocks.json`
- `mining.json`
- `statistics.json`
- `the-earth-and-the-solar-system.json`
- `the-structure-of-the-earth.json`
- `weather.json`
**Depends on:** Task 01
**Parallel group:** P2

**Specification:**
Read `AGENT.md` in full before starting. For every topic JSON file listed above:
1. Verify factual accuracy — rock types, mineral properties, Earth's layers (depths and compositions), solar system facts, and weather instrument readings must match KCSE Geography syllabus and standard geographic facts.
2. Ensure Form 1 scope only.
3. Add `stimulus` objects for questions referencing maps, cross-sections, weather charts, photographs, or statistical data tables.
4. Upgrade `example_answer.format` to `"svg"` for: Earth cross-section diagrams, rock cycle diagrams, weather instrument diagrams (thermometer, barometer, rain gauge setups), statistical charts (bar graphs, line graphs, pie charts). Create SVG files in `form1/geography/illustrations/`.
5. For field work questions, include data response questions with sample field data tables.
6. Review `answer_lines` and correct.
7. Verify `type`, `difficulty`, `cognitive_level` — Geography should include `data_response` questions (map reading, graph interpretation, statistical analysis) and `diagram` questions.
8. Ensure each topic has at minimum 30 questions.
9. Fix atomization errors in rubric criteria.
10. Consult `../materials/form-1/geography/` and KCSE Geography past papers for accuracy and coverage.

**Update after completion:**
- [ ] Mark this task `[x]`
- [ ] git commit: `fix(geography): content improvement pass Form 1`

---

### Task 10: Form 1 History and Government — Content Improvement
**Files to modify:** `form1/history/*.json`
**Files in scope:**
- `citizenship.json`
- `contacts-between-east-africa-and-the-outside-world.json`
- `development-of-agriculture.json`
- `early-man.json`
- `introduction-to-history-and-government.json`
- `national-integration.json`
- `social-economic-and-political-organisation.json`
- `the-peoples-of-kenya.json`
**Depends on:** Task 01
**Parallel group:** P2

**Specification:**
Read `AGENT.md` in full before starting. For every topic JSON file listed above:
1. Verify historical accuracy — dates, names of peoples, migration routes, trading contacts, and political structures must be accurate per the KCSE History and Government syllabus.
2. Ensure Form 1 scope — pre-colonial Kenya, early African history, and early contacts with outside world. Do not include Form 2 content (colonialism, industry).
3. History is predominantly text-based. Add SVG illustrations only where a map or timeline diagram would genuinely help (e.g., migration routes of Kenyan peoples, trade routes).
4. Add `stimulus` objects for questions that reference historical documents, maps, or source material extracts.
5. Review `answer_lines` and correct.
6. Verify `type`, `difficulty`, `cognitive_level` — History questions should include `explanation`, `structured`, and application questions relating historical events to modern Kenya.
7. Ensure each topic has at minimum 30 questions.
8. Fix atomization errors — History enumeration questions (e.g., "state factors that led to the migration of...") must list all valid answers as separate criteria with `max_marks`.
9. Consult `../materials/form-1/history/` and KCSE History past papers for accuracy and coverage.

**Update after completion:**
- [ ] Mark this task `[x]`
- [ ] git commit: `fix(history): content improvement pass Form 1`

---

### Task 11: Form 1 Home Science — Content Improvement
**Files to modify:** `form1/home-science/*.json`
**Files in scope:**
- `care-of-the-home.json`
- `clothing-and-textiles.json`
- `consumer-education.json`
- `food-and-nutrition.json`
- `foods-and-nutrition.json`
- `home-management.json`
- `household-equipment.json`
- `housing.json`
- `housing-the-family.json`
- `introduction-to-home-science.json`
- `meal-planning-and-service.json`
- `needlework-and-garment-construction.json`
- `personal-hygiene.json`
- `the-home.json`
**Depends on:** Task 01
**Parallel group:** P2

**Specification:**
Read `AGENT.md` in full before starting. Note: there appear to be duplicate or near-duplicate topic files (e.g., `food-and-nutrition.json` and `foods-and-nutrition.json`; `housing.json` and `housing-the-family.json`; `the-home.json` and `care-of-the-home.json`). For every topic JSON file listed above:
1. Verify factual accuracy — nutritional content values, fabric properties, cleaning methods, and housing standards must be accurate per the KCSE Home Science syllabus.
2. Check for and resolve duplicate content: if two topic files cover the same syllabus topic, consolidate the best questions into one file and note the duplication in the commit message.
3. Ensure Form 1 scope only.
4. Add `stimulus` objects for questions referencing food composition tables, fabric identification charts, or floor plan diagrams.
5. Upgrade `example_answer.format` to `"svg"` for: floor plan diagrams, garment pattern layouts, sewing stitch diagrams, table setting layouts. Create SVG files in `form1/home-science/illustrations/`.
6. Review `answer_lines` and correct.
7. Verify `type`, `difficulty`, `cognitive_level`.
8. Ensure each unique topic has at minimum 30 questions after de-duplication.
9. Fix atomization errors in rubric criteria.
10. Consult `../materials/form-1/home-science/` and KCSE Home Science past papers for accuracy and coverage.

**Update after completion:**
- [ ] Mark this task `[x]`
- [ ] git commit: `fix(home-science): content improvement pass Form 1`

---

### Task 12: Form 1 IRE — Content Improvement
**Files to modify:** `form1/ire/*.json`
**Files in scope:**
- `akhlaq.json`
- `hadith.json`
- `life-of-prophet-muhammad.json`
- `muamalat.json`
- `pillars-of-iman.json`
- `pillars-of-islam.json`
- `quran.json`
- `salat.json`
- `sources-of-shariah.json`
- `tahara.json`
**Depends on:** Task 01
**Parallel group:** P2

**Specification:**
Read `AGENT.md` in full before starting. For every topic JSON file listed above:
1. Verify religious accuracy — Quranic references, Hadith classifications, names of pillars, Arabic terminology, and historical facts about Prophet Muhammad (PBUH) must be accurate per the KCSE IRE syllabus.
2. Arabic terms must be correctly spelled and explained. Do not use transliterations inconsistently.
3. Ensure Form 1 scope — Introduction to Islam, pillars, Tahara, Salat, basic Quran knowledge, basic Hadith, life of the Prophet (early life to Hijra period), basic Muamalat and Akhlaq at Form 1 level.
4. IRE is predominantly text-based. Add SVG illustrations only where genuinely useful (e.g., wudu procedure steps illustrated, direction of qibla diagram).
5. Add `stimulus` objects for questions referencing Quranic verses or Hadith text excerpts provided for analysis.
6. Review `answer_lines` and correct.
7. Verify `type`, `difficulty`, `cognitive_level` — include application questions relating Islamic teachings to contemporary Kenyan Muslim life.
8. Ensure each topic has at minimum 30 questions.
9. Fix atomization errors in rubric criteria.
10. Consult `../materials/form-1/ire/` and KCSE IRE past papers for accuracy and coverage.

**Update after completion:**
- [ ] Mark this task `[x]`
- [ ] git commit: `fix(ire): content improvement pass Form 1`

---

### Task 13: Form 1 Kiswahili — Content Improvement
**Files to modify:** `form1/kiswahili/*.json`
**Files in scope:**
- `aina-za-maneno.json`
- `barua-ya-kirafiki.json`
- `fasihi-simulizi-methali-vitendawili-na-nyimbo.json`
- `fasihi-simulizi-ngano.json`
- `insha-ya-masimulizi-na-maelezo.json`
- `msamiati-na-matumizi-ya-kamusi.json`
- `ngeli-za-nomino-msingi.json`
- `njeo-za-vitenzi.json`
- `sentensi-sahili-na-uakifishaji.json`
- `ufahamu.json`
- `ufupisho.json`
- `ushairi-utangulizi.json`
**Depends on:** Task 01
**Parallel group:** P2

**Specification:**
Read `AGENT.md` in full before starting. All questions must be written in correct, idiomatic Kiswahili sanifu (standard Swahili) appropriate for the KCSE Kiswahili syllabus.
1. Verify linguistic accuracy — ngeli classifications, verb conjugations, tense forms, and grammatical rules must conform to standard Kiswahili grammar as taught in Form 1.
2. Ensure Form 1 scope — basic ngeli (M-WA, M-MI), simple tenses (present, past, future), simple sentence structure, basic ufahamu and ufupisho passages, introduction to fasihi simulizi.
3. For ufahamu (comprehension) questions: the full passage (vifungu vya habari) must be embedded in `body` or `stimulus`. Follow KCSE Kiswahili comprehension format.
4. For ufupisho (summary) questions: include a passage and a clear instruction on what to summarise and the word limit.
5. For fasihi simulizi questions (ngano, methali, vitendawili): include the full text of the oral literature item in the `body` or `stimulus`, followed by analysis questions.
6. For ushairi questions: include the complete poem text before the sub-questions.
7. Add `stimulus` objects for all passage-based questions.
8. Review `answer_lines` and correct — Kiswahili writing tasks require generous line allocation.
9. Verify `type`, `difficulty`, `cognitive_level`.
10. Ensure each topic has at minimum 30 questions.
11. Fix atomization errors in rubric criteria.
12. Consult `../materials/form-1/kiswahili/` and KCSE Kiswahili past papers for accuracy and style.

**Update after completion:**
- [ ] Mark this task `[x]`
- [ ] git commit: `fix(kiswahili): content improvement pass Form 1`

---

### Task 14: Form 1 Mathematics — Content Improvement
**Files to modify:** `form1/mathematics/*.json`
**Files in scope:**
- `algebraic-expressions.json`
- `angles-and-plane-figures.json`
- `area.json`
- `commercial-arithmetic.json`
- `common-solids.json`
- `coordinates-and-graphs.json`
- `decimals.json`
- `divisibility-tests.json`
- `factors.json`
- `fractions.json`
- `gcd.json`
- `geometric-constructions.json`
- `integers.json`
- `lcm.json`
- `length.json`
- `linear-equations.json`
- `mass-weight-and-density.json`
- `natural-numbers.json`
- `rates-ratio-proportion-and-percentage.json`
- `scale-drawing.json`
- `squares-and-square-roots.json`
- `time.json`
- `volume-and-capacity.json`
**Depends on:** Task 01
**Parallel group:** P2

**Specification:**
Read `AGENT.md` in full before starting. For every topic JSON file listed above:
1. Verify mathematical accuracy — all calculations, answers, and working must be correct. Verify geometric constructions are described precisely.
2. Ensure Form 1 scope — no Form 2+ content (trigonometry, quadratics, logarithms are not Form 1).
3. Upgrade `example_answer.format` to `"svg"` for ALL geometric construction questions (bisection, perpendicular, angle construction, loci). SVG must show ALL compass arcs, not just the final figure. Create SVG files in `form1/mathematics/illustrations/`.
4. Upgrade `example_answer.format` to `"svg"` for: coordinate plane graphs, geometric figures (triangles, quadrilaterals, circles), scale drawings.
5. Set `answer_space_type: "construction_box"` with appropriate `answer_box_height_mm` (90–110) for all construction questions.
6. Set `answer_space_type: "grid_box"` for all graph-plotting questions.
7. Upgrade `example_answer.format` to `"tiptap"` for answers with step-by-step working using mathematical notation.
8. Calculation rubrics must follow the formula + substitution + answer pattern (§3.4).
9. Review `answer_lines` and correct.
10. Verify `type`, `difficulty`, `cognitive_level` — Mathematics must have strong representation of `calculation` and `diagram` types.
11. Ensure each topic has at minimum 30 questions with graduated difficulty (easy computation → multi-step word problem → proof).
12. Fix atomization errors.
13. Consult `../materials/form-1/mathematics/` and KCSE Mathematics past papers for accuracy and style.

**Update after completion:**
- [ ] Mark this task `[x]`
- [ ] git commit: `fix(mathematics): content improvement pass Form 1`

---

### Task 15: Form 1 Physics — Content Improvement
**Files to modify:** `form1/physics/*.json`
**Files in scope:**
- `cells-and-simple-circuits.json`
- `electrostatics-i.json`
- `forces.json`
- `heat-transfer.json`
- `introduction-to-physics.json`
- `measurement-i.json`
- `measurement-ii.json`
- `particulate-nature-of-matter.json`
- `pressure.json`
- `rectilinear-propagation-and-reflection-at-plane-surfaces.json`
- `thermal-expansion.json`
**Depends on:** Task 01
**Parallel group:** P2

**Specification:**
Read `AGENT.md` in full before starting. For every topic JSON file listed above:
1. Verify factual accuracy — all formulae, units, constants (e.g., atmospheric pressure = 101,325 Pa or approximately 10⁵ Pa for KCSE), and physical laws must be stated correctly.
2. Ensure Form 1 scope — Electrostatics I covers static charges and basic laws; it does not cover capacitors or electric fields (Form 3+). Reflection covers plane mirrors only (curved mirrors are Form 2).
3. Upgrade `example_answer.format` to `"svg"` for: ray diagrams (reflection at plane mirrors), circuit diagrams (series and parallel circuits), force diagrams, apparatus setups (measuring instruments, pressure apparatus). Create SVG files in `form1/physics/illustrations/`.
4. Set `answer_space_type: "diagram_box"` for all diagram-drawing questions.
5. Upgrade `example_answer.format` to `"tiptap"` for answers with SI units and numerical working.
6. Calculation rubrics must follow formula + substitution + answer pattern.
7. Review `answer_lines` and correct.
8. Verify `type`, `difficulty`, `cognitive_level` — Physics must have strong representation of `calculation`, `diagram`, and `experiment` types.
9. Ensure each topic has at minimum 30 questions including experiment design questions and data interpretation questions.
10. Fix atomization errors.
11. Consult `../materials/form-1/physics/` and KCSE Physics past papers for accuracy and style.

**Update after completion:**
- [ ] Mark this task `[x]`
- [ ] git commit: `fix(physics): content improvement pass Form 1`

---
