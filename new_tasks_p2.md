### Task 16: Form 2 Agriculture — Content Improvement
**Files to modify:** `form2/agriculture/*.json`
**Files in scope:**
- `crop-production-ii-planting.json`
- `crop-production-iii-nursery-practices.json`
- `crop-production-iv-field-practices.json`
- `crop-production-v-vegetables.json`
- `livestock-health-i-introduction.json`
- `livestock-health-ii-parasites.json`
- `livestock-production-ii-nutrition.json`
- `soil-fertility-ii-inorganic-fertilizers.json`
**Depends on:** Task 01
**Parallel group:** P2

**Specification:**
Read `AGENT.md` in full before starting. For every topic JSON file listed above:
1. Verify factual accuracy of all questions, rubric criteria, and example answers against Form 2 Agriculture KCSE syllabus content. Correct any errors.
2. Ensure Form 2 scope only — do not overlap with Form 1 Agriculture (soil formation, introduction to crops) or Form 3+ content.
3. Add `stimulus` objects for questions referencing planting schedule tables, parasite life-cycle diagrams, fertilizer composition tables, or nursery layout diagrams.
4. Upgrade `example_answer.format` to `"svg"` for: nursery bed diagrams, planting pattern diagrams (broadcasting, row planting, drills), parasite diagrams (external/internal parasites of livestock), and fertilizer application diagrams. Create SVG files in `form2/agriculture/illustrations/`.
5. Review `answer_lines` values and correct using the word-count rule from §2.6.
6. Verify `type`, `difficulty`, `cognitive_level` — include `experiment` questions for nursery management and `data_response` questions for fertilizer application rates.
7. Ensure each topic has at minimum 30 questions. Add questions covering: pest and disease identification, parasite control methods, livestock nutrition calculations (feed rations), and practical application scenarios.
8. Fix any atomization errors in rubric criteria.
9. Consult `../materials/form-2/agriculture/` and `../kcse-past-papers/2016/42/` through `../kcse-past-papers/2024/42/` for accuracy and coverage.

**Update after completion:**
- [ ] Mark this task `[x]`
- [ ] git commit: `fix(agriculture): content improvement pass Form 2`

---

### Task 17: Form 2 Biology — Content Improvement
**Files to modify:** `form2/biology/*.json`
**Files in scope:**
- `excretion-and-homeostasis.json`
- `gaseous-exchange.json`
- `respiration.json`
- `transport-in-plants-and-animals.json`
**Depends on:** Task 01
**Parallel group:** P2

**Specification:**
Read `AGENT.md` in full before starting. For every topic JSON file listed above:
1. Verify factual accuracy — structures of gaseous exchange surfaces (alveoli, stomata, gills), transport mechanisms (active transport, osmosis), blood composition, and homeostasis processes must be accurate per the KCSE Biology syllabus.
2. Ensure Form 2 scope — Transport in Plants and Animals at Form 2 covers the circulatory system, xylem/phloem, transpiration, and translocation. Do NOT include nervous system or hormonal control (Form 4).
3. Add `stimulus` objects for questions referencing diagrams of the heart, cross-sections of leaves, blood smear data, or experimental observation tables.
4. Upgrade `example_answer.format` to `"image"` for complex biological diagrams: human heart cross-section, leaf cross-section (TS), alveolus diagram, kidney tubule structure. Add detailed `description` fields. Create SVG files for simpler diagrams (blood circulation pathway, transpiration stream). Store in `form2/biology/illustrations/`.
5. Upgrade `example_answer.format` to `"svg"` for experimental apparatus setups (potometer, bubble experiment for photosynthesis, respirometer).
6. Review `answer_lines` and correct.
7. Verify `type`, `difficulty`, `cognitive_level` — Biology Form 2 must have strong `experiment` and `structured` question representation.
8. Ensure each topic has at minimum 30 questions, covering: definitions, processes, comparisons, experiments, diagram labelling, and data interpretation.
9. Fix atomization errors — particularly enumeration rubrics for "state adaptations of..." questions.
10. Consult `../materials/form-2/biology/` and KCSE Biology past papers for accuracy and coverage.

**Update after completion:**
- [ ] Mark this task `[x]`
- [ ] git commit: `fix(biology): content improvement pass Form 2`

---

### Task 18: Form 2 Business Studies — Content Improvement
**Files to modify:** `form2/business-studies/*.json`
**Files in scope:**
- `communication.json`
- `forms-of-business-units.json`
- `government-and-business.json`
- `insurance.json`
- `internal-trade.json`
- `product-markets.json`
- `transport.json`
- `warehousing.json`
**Depends on:** Task 01
**Parallel group:** P2

**Specification:**
Read `AGENT.md` in full before starting. For every topic JSON file listed above:
1. Verify factual accuracy — insurance terminology (premium, indemnity, subrogation, contribution), types of business units (sole trader, partnership, cooperative, company), and trade concepts must be accurate per the KCSE Business Studies syllabus.
2. Ensure Form 2 scope only — do not include Form 1 content or Form 3 accounting topics.
3. Add `stimulus` objects for questions referencing business scenario case studies, insurance policy excerpts, or communication medium comparison tables.
4. Upgrade `body_format` to `"tiptap"` for questions containing comparison tables (e.g., advantages/disadvantages tables for types of business units).
5. Verify `type`, `difficulty`, `cognitive_level` — Business Studies should have good representation of `explanation`, `structured`, and application/scenario questions.
6. Review `answer_lines` and correct.
7. Ensure each topic has at minimum 30 questions with diverse question styles including scenario-based application questions that mirror KCSE Paper 1 format.
8. Fix atomization errors in rubric criteria.
9. Consult `../materials/form-2/business-studies/` and KCSE Business Studies past papers for accuracy and coverage.

**Update after completion:**
- [ ] Mark this task `[x]`
- [ ] git commit: `fix(business-studies): content improvement pass Form 2`

---

### Task 19: Form 2 Chemistry — Content Improvement
**Files to modify:** `form2/chemistry/*.json`
**Files in scope:**
- `carbon-and-its-compounds.json`
- `salts.json`
- `structure-and-bonding.json`
- `the-periodic-table.json`
- `water-and-hydrogen.json`
**Depends on:** Task 01
**Parallel group:** P2

**Specification:**
Read `AGENT.md` in full before starting. For every topic JSON file listed above:
1. Verify factual accuracy — all chemical equations must be balanced and state symbols correct; ionic equations for salt preparation must be accurate; periodic table trends must match KCSE syllabus content; carbon compound properties must be correct.
2. Ensure Form 2 scope — Structure and Bonding at Form 2 covers ionic and covalent bonding, dot-and-cross diagrams, and metallic bonding. The Periodic Table at Form 2 covers Groups I, II, VII and Period 3 trends.
3. Add `stimulus` objects for questions referencing periodic table excerpts, experimental observation tables, or reaction data tables.
4. Upgrade `example_answer.format` to `"svg"` for: dot-and-cross diagrams, periodic table segment diagrams, salt preparation apparatus setups, carbon structure diagrams (graphite, diamond). Create SVG files in `form2/chemistry/illustrations/`.
5. Upgrade `example_answer.format` to `"tiptap"` for answers containing balanced chemical equations.
6. Review `answer_lines` and correct.
7. Verify `type`, `difficulty`, `cognitive_level` — Chemistry must have strong `calculation` (mole calculations, salt preparation calculations) and `experiment` representation.
8. Ensure each topic has at minimum 30 questions.
9. Fix atomization errors — particularly for "state properties of..." and "name uses of..." type rubrics.
10. Consult `../materials/form-2/chemistry/` and KCSE Chemistry past papers for accuracy and coverage.

**Update after completion:**
- [ ] Mark this task `[x]`
- [ ] git commit: `fix(chemistry): content improvement pass Form 2`

---

### Task 20: Form 2 Computer Studies — Content Improvement
**Files to modify:** `form2/computer-studies/*.json`
**Files in scope:**
- `computer-hardware.json`
- `databases.json`
- `data-representation.json`
- `data-security-and-controls.json`
- `desktop-publishing.json`
- `logic-gates.json`
- `spreadsheets.json`
- `the-internet-and-email.json`
- `word-processing.json`
**Depends on:** Task 01
**Parallel group:** P2

**Specification:**
Read `AGENT.md` in full before starting. For every topic JSON file listed above:
1. Verify factual accuracy — logic gate truth tables (AND, OR, NOT, NAND, NOR, XOR) must be correct; data representation conversions (binary, denary, hexadecimal, BCD) must be accurate; database terminology must match KCSE Computer Studies syllabus.
2. Ensure Form 2 scope — hexadecimal and BCD representation are Form 2/3 content; logic gates are Form 2.
3. Upgrade `example_answer.format` to `"svg"` for: logic gate symbols and circuits, truth table diagrams, database ER diagrams, computer network diagrams. Create SVG files in `form2/computer-studies/illustrations/`.
4. Upgrade `example_answer.format` to `"tiptap"` for answers showing step-by-step binary/hexadecimal conversions.
5. Add `stimulus` objects for questions referencing truth tables, network diagrams, or database schemas provided as data.
6. Set `answer_space_type: "grid_box"` for truth table completion questions.
7. Review `answer_lines` and correct.
8. Verify `type`, `difficulty`, `cognitive_level` — include `calculation` questions for number system conversions and `diagram` questions for logic circuits.
9. Ensure each topic has at minimum 30 questions.
10. Fix atomization errors.
11. Consult `../materials/form-2/computer-studies/` and KCSE Computer Studies past papers for accuracy and coverage.

**Update after completion:**
- [ ] Mark this task `[x]`
- [ ] git commit: `fix(computer-studies): content improvement pass Form 2`

---

### Task 21: Form 2 CRE — Content Improvement
**Files to modify:** `form2/cre/*.json`
**Files in scope:**
- `jesus-ministry-in-jerusalem.json`
- `old-testament-prophesies-about-the-messiah.json`
- `the-galilean-ministry.json`
- `the-infancy-and-early-life-of-jesus.json`
- `the-journey-to-jerusalem.json`
- `the-passion-death-and-resurrection-of-jesus.json`
**Depends on:** Task 01
**Parallel group:** P2

**Specification:**
Read `AGENT.md` in full before starting. For every topic JSON file listed above:
1. Verify theological accuracy — Gospel references (Matthew, Mark, Luke, John), events of Jesus's ministry, Old Testament prophecies (Isaiah, Micah, Zechariah), and their New Testament fulfilments must be accurate per the KCSE CRE syllabus.
2. Ensure Form 2 scope — the life and ministry of Jesus Christ as taught in Form 2 CRE.
3. Add `stimulus` objects for questions referencing Bible passages provided for analysis or comparison.
4. Review `answer_lines` and correct.
5. Verify `type`, `difficulty`, `cognitive_level` — include application questions relating lessons from Jesus's ministry to the life of a young Kenyan Christian today.
6. Ensure each topic has at minimum 30 questions including: events of the ministry, teachings and parables, miracles, character traits, and contemporary application.
7. Fix atomization errors.
8. Consult `../materials/form-2/cre/` and KCSE CRE past papers for accuracy and coverage.

**Update after completion:**
- [ ] Mark this task `[x]`
- [ ] git commit: `fix(cre): content improvement pass Form 2`

---

### Task 22: Form 2 English — Content Improvement
**Files to modify:** `form2/english/*.json`
**Files in scope:**
- `active-and-passive-voice.json`
- `cloze-tests-and-error-identification.json`
- `compound-and-complex-sentences.json`
- `comprehension-skills.json`
- `creative-writing-narratives-and-descriptions.json`
- `functional-writing-formal-letters.json`
- `functional-writing-reports-notices-and-messages.json`
- `literary-devices-and-figurative-language.json`
- `modal-verbs.json`
- `oral-skills-listening-and-speaking.json`
- `oral-skills-pronunciation-and-word-stress.json`
- `phrasal-verbs-and-idiomatic-expressions.json`
- `question-tags.json`
- `subject-verb-agreement.json`
- `tenses.json`
- `vocabulary-and-word-formation.json`
**Depends on:** Task 01
**Parallel group:** P2

**Specification:**
Read `AGENT.md` in full, especially §11.6 (English Language Question Formats), before starting.
1. Verify linguistic accuracy — passive voice transformations, question tag rules, modal verb usage, and tense sequencing must be grammatically correct per standard English grammar and KCSE marking conventions.
2. Ensure Form 2 scope — formal letters (letters of complaint, letters of inquiry) are Form 2 functional writing; application letters and CVs are Form 3.
3. Add `stimulus` objects for all passage-based questions: comprehension passages → `stimulus.type: "passage"`, cloze tests → `stimulus.type: "passage"`, dialogue analysis → `stimulus.type: "passage"`.
4. Grammar transformation questions must use `type: "structured"` with each sentence transformation as a separate part, with the exact expected rewrite in `example_answer.content` and exact criterion in the rubric.
5. For functional writing: rubrics must use holistic band descriptors (format/content/language) — NOT atomized point-by-point criteria.
6. For comprehension: ensure full passage is embedded. Ensure sub-questions follow the KCSE pattern from §11.6b (factual → inference → summary → vocabulary in context).
7. For cloze tests: verify each passage has exactly 10 blanks labelled (a)–(j) and that the correct answers are unambiguous given the context.
8. Review `answer_lines` — writing tasks (formal letters, reports) need `answer_space_type: "plain_box"` with `answer_box_height_mm: 160`.
9. Ensure each topic has at minimum 30 questions; comprehension and grammar topics should aim for 40+.
10. Consult `../materials/form-2/english/` and KCSE English Paper 1 and Paper 2 past papers.

**Update after completion:**
- [ ] Mark this task `[x]`
- [ ] git commit: `fix(english): content improvement pass Form 2`

---

### Task 23: Form 2 Geography — Content Improvement
**Files to modify:** `form2/geography/*.json`
**Files in scope:**
- `climate.json`
- `forestry.json`
- `internal-land-forming-processes.json`
- `map-work.json`
- `photograph-work.json`
- `statistical-methods.json`
- `vegetation.json`
**Depends on:** Task 01
**Parallel group:** P2

**Specification:**
Read `AGENT.md` in full before starting. For every topic JSON file listed above:
1. Verify factual accuracy — climate classification systems (Koppen), vegetation zones, tectonic plate boundaries, volcanic and seismic activity zones, map scale calculations, and statistical method formulas must be accurate per the KCSE Geography syllabus.
2. Ensure Form 2 scope — Internal Land-Forming Processes covers vulcanicity, folding, faulting, and earthquakes at Form 2 level.
3. Add `stimulus` objects for questions referencing topographic maps, climate graphs, photograph extracts, or statistical data tables.
4. Upgrade `example_answer.format` to `"svg"` for: fold and fault diagrams (syncline, anticline, horst, rift valley), volcanic structure cross-sections, climate graphs, population pyramids in statistical methods, vegetation zone transect diagrams. Create SVG files in `form2/geography/illustrations/`.
5. Set `answer_space_type: "diagram_box"` for diagram-drawing questions, `"grid_box"` for graph-plotting questions.
6. Include `data_response` questions with actual map reading exercises (grid references, bearings, scale calculations).
7. Review `answer_lines` and correct.
8. Verify `type`, `difficulty`, `cognitive_level` — Geography must have strong `data_response`, `diagram`, and `calculation` representation.
9. Ensure each topic has at minimum 30 questions.
10. Fix atomization errors.
11. Consult `../materials/form-2/geography/` and KCSE Geography past papers for accuracy and coverage.

**Update after completion:**
- [ ] Mark this task `[x]`
- [ ] git commit: `fix(geography): content improvement pass Form 2`

---

### Task 24: Form 2 History and Government — Content Improvement
**Files to modify:** `form2/history/*.json`
**Files in scope:**
- `constitution-and-constitution-making.json`
- `democracy-and-human-rights.json`
- `development-of-industry.json`
- `development-of-transport-and-communication.json`
- `organisation-of-african-societies-in-the-19th-century.json`
- `trade.json`
- `urbanisation.json`
**Depends on:** Task 01
**Parallel group:** P2

**Specification:**
Read `AGENT.md` in full before starting. For every topic JSON file listed above:
1. Verify historical accuracy — dates and facts about the Industrial Revolution, development of transport (railways, steamships), trade routes, and the constitution-making process must be accurate per the KCSE History and Government syllabus.
2. Ensure Form 2 scope — covers world history (Industrial Revolution, trade, transport) and introduction to government (democracy, human rights, constitutions).
3. Add `stimulus` objects for questions referencing historical sources, charts showing industrial output, or comparative data on transport methods.
4. History is predominantly text-based. Add SVG illustrations only for: trade route maps, timeline diagrams of industrial development.
5. Review `answer_lines` and correct.
6. Verify `type`, `difficulty`, `cognitive_level` — include application questions connecting historical developments to Kenya and Africa today.
7. Ensure each topic has at minimum 30 questions.
8. Fix atomization errors — History enumeration rubrics must list all valid answers as separate criteria with `max_marks`.
9. Consult `../materials/form-2/history/` and KCSE History past papers for accuracy and coverage.

**Update after completion:**
- [ ] Mark this task `[x]`
- [ ] git commit: `fix(history): content improvement pass Form 2`

---

### Task 25: Form 2 Home Science — Content Improvement
**Files to modify:** `form2/home-science/*.json`
**Files in scope:**
- `child-development-and-care.json`
- `clothing-construction-and-pattern-making.json`
- `consumer-awareness-and-budgeting.json`
- `cooking-methods-and-meal-planning.json`
- `family-health-and-first-aid.json`
- `food-preservation-and-storage.json`
- `home-improvement-and-interior-decoration.json`
- `laundry-work.json`
**Depends on:** Task 01
**Parallel group:** P2

**Specification:**
Read `AGENT.md` in full before starting. For every topic JSON file listed above:
1. Verify factual accuracy — child developmental milestones, first aid procedures, food preservation methods (canning temperatures, pH levels for pickling), fabric care symbols, and cooking temperatures must be accurate per the KCSE Home Science syllabus.
2. Ensure Form 2 scope only.
3. Add `stimulus` objects for questions referencing food composition tables, child development milestone charts, or fabric care label symbols.
4. Upgrade `example_answer.format` to `"svg"` for: garment pattern layout diagrams, seam types and stitch diagrams, food preservation methods diagrams (bottling apparatus, dehydration), child developmental stage timelines. Create SVG files in `form2/home-science/illustrations/`.
5. Review `answer_lines` and correct.
6. Verify `type`, `difficulty`, `cognitive_level`.
7. Ensure each topic has at minimum 30 questions.
8. Fix atomization errors.
9. Consult `../materials/form-2/home-science/` and KCSE Home Science past papers for accuracy and coverage.

**Update after completion:**
- [ ] Mark this task `[x]`
- [ ] git commit: `fix(home-science): content improvement pass Form 2`

---

### Task 26: Form 2 IRE — Content Improvement
**Files to modify:** `form2/ire/*.json`
**Files in scope:**
- `akhlaq-islamic-moral-teachings.json`
- `hajj.json`
- `muamalat-islamic-family-law.json`
- `saum.json`
- `selected-hadith-and-teachings.json`
- `selected-surahs-and-their-teachings.json`
- `sources-of-shariah.json`
- `tajweed-rules.json`
- `the-medinan-period.json`
- `zakat.json`
**Depends on:** Task 01
**Parallel group:** P2

**Specification:**
Read `AGENT.md` in full before starting. For every topic JSON file listed above:
1. Verify religious accuracy — Hajj rituals and their sequence, Zakat nisab thresholds and calculation, Saum rules, Medinan period historical events, selected Surah content, Tajweed rules, and family law principles must be accurate per the KCSE IRE syllabus.
2. Ensure Form 2 scope — Medinan period covers Hijra through the consolidation of the Muslim community; it does not include later Islamic history (Khulafaa Rashidun is Form 3).
3. Add `stimulus` objects for questions referencing Quranic verses or Hadith text provided for analysis.
4. For Tajweed rules questions: include phonetic examples and require students to identify or apply specific rules.
5. Review `answer_lines` and correct.
6. Verify `type`, `difficulty`, `cognitive_level` — include application questions relating Islamic teachings to contemporary issues faced by Kenyan Muslims.
7. Ensure each topic has at minimum 30 questions.
8. Fix atomization errors.
9. Consult `../materials/form-2/ire/` and KCSE IRE past papers for accuracy and coverage.

**Update after completion:**
- [ ] Mark this task `[x]`
- [ ] git commit: `fix(ire): content improvement pass Form 2`

---

### Task 27: Form 2 Kiswahili — Content Improvement
**Files to modify:** `form2/kiswahili/*.json`
**Files in scope:**
- `barua-rasmi.json`
- `fasihi-simulizi-hadithi-za-mtanziko.json`
- `insha-ya-mdahalo-na-hotuba.json`
- `isimu-jamii-utangulizi.json`
- `kauli-za-vitenzi.json`
- `nahau-na-misemo.json`
- `ngeli-za-nomino-za-juu.json`
- `sentensi-changamano-na-ambatano.json`
- `tamathali-za-usemi.json`
- `ufahamu.json`
- `ufupisho.json`
- `ukanushaji-wa-vitenzi.json`
- `ushairi-uchambuzi-wa-shairi.json`
- `viambishi-vya-kiswahili.json`
- `vielezi-viunganishi-na-vihusishi.json`
**Depends on:** Task 01
**Parallel group:** P2

**Specification:**
Read `AGENT.md` in full before starting. All questions must be in correct Kiswahili sanifu.
1. Verify linguistic accuracy — advanced ngeli classifications, verb moods (kauli za vitenzi: kauli tendwa, tendea, tendeana, tendeza, tenda), negation forms (ukanushaji), and tamathali za usemi (figures of speech) must be grammatically and linguistically accurate per the KCSE Kiswahili syllabus.
2. Ensure Form 2 scope — advanced ngeli (N-, U-/W-, MA-, KI-/VI-, etc.), complex sentences, formal letters, poetry analysis, and introduction to sociolinguistics at Form 2 level.
3. For ufahamu and ufupisho: embed full passages in `stimulus`. Follow KCSE Kiswahili format precisely.
4. For ushairi (poetry) questions: embed the full shairi (poem) text before sub-questions. Include questions on: dhamira (theme), mtindo (style), tamathali za usemi, maudhui, and muundo.
5. For hadithi za mtanziko (dilemma stories): include the full narrative before analysis questions.
6. For barua rasmi: verify letter format conventions for formal Kiswahili letters (tarehe, anwani, salamu, mwili, hitimisho).
7. Add `stimulus` objects for all passage-based questions.
8. Review `answer_lines` and correct.
9. Verify `type`, `difficulty`, `cognitive_level`.
10. Ensure each topic has at minimum 30 questions.
11. Fix atomization errors.
12. Consult `../materials/form-2/kiswahili/` and KCSE Kiswahili past papers for accuracy and style.

**Update after completion:**
- [ ] Mark this task `[x]`
- [ ] git commit: `fix(kiswahili): content improvement pass Form 2`

---

### Task 28: Form 2 Mathematics — Content Improvement
**Files to modify:** `form2/mathematics/*.json`
**Files in scope:**
- `angle-properties-of-a-circle.json`
- `area-of-a-triangle.json`
- `area-of-part-of-a-circle.json`
- `area-of-quadrilaterals.json`
- `cubes-and-cube-roots.json`
- `gradient-and-equations-of-straight-lines.json`
- `indices-and-logarithms.json`
- `linear-motion.json`
- `quadratic-expressions-and-equations.json`
- `reciprocals.json`
- `reflection-and-congruence.json`
- `rotation.json`
- `similarity-and-enlargement.json`
- `statistics-i.json`
- `surface-area-of-solids.json`
- `the-pythagoras-theorem.json`
- `trigonometric-ratios-i.json`
- `vectors-i.json`
- `volume-of-solids.json`
**Depends on:** Task 01
**Parallel group:** P2

**Specification:**
Read `AGENT.md` in full before starting. For every topic JSON file listed above:
1. Verify mathematical accuracy — all calculations, proofs, and geometric constructions must be correct.
2. Ensure Form 2 scope — Trigonometry at Form 2 covers sine, cosine, tangent ratios only (no sine rule, cosine rule — those are Form 3/4).
3. Upgrade `example_answer.format` to `"svg"` for: geometric transformations (reflection, rotation, enlargement), circle theorems diagrams, trigonometry right-triangle diagrams, Pythagoras theorem diagrams, vector diagrams, solids diagrams (for surface area and volume). Create SVG files in `form2/mathematics/illustrations/`.
4. Set `answer_space_type: "grid_box"` for all coordinate geometry and graph-plotting questions.
5. Set `answer_space_type: "construction_box"` for geometric construction questions (e.g., constructing angles using circle properties).
6. Upgrade `example_answer.format` to `"tiptap"` for answers showing step-by-step logarithm or index calculations.
7. Calculation rubrics must follow formula + substitution + answer pattern with follow-through.
8. Review `answer_lines` and correct.
9. Verify `type`, `difficulty`, `cognitive_level` — strong `calculation` and `diagram` representation required.
10. Ensure each topic has at minimum 30 questions with graduated difficulty.
11. Fix atomization errors.
12. Consult `../materials/form-2/mathematics/` and KCSE Mathematics past papers for accuracy and coverage.

**Update after completion:**
- [ ] Mark this task `[x]`
- [ ] git commit: `fix(mathematics): content improvement pass Form 2`

---

### Task 29: Form 2 Physics — Content Improvement
**Files to modify:** `form2/physics/*.json`
**Files in scope:**
- `electrostatics-i.json`
- `equilibrium-and-centre-of-gravity.json`
- `fluid-flow.json`
- `hookes-law.json`
- `magnetic-effect-of-electric-current.json`
- `magnetism-i.json`
- `measurement-ii.json`
- `reflection-at-curved-surfaces.json`
- `sound.json`
- `turning-effect-of-a-force.json`
- `waves-i.json`
**Depends on:** Task 01
**Parallel group:** P2

**Specification:**
Read `AGENT.md` in full before starting. For every topic JSON file listed above:
1. Verify factual accuracy — all formulae (moment = force × perpendicular distance, v = fλ, F = ke), units, mirror formulae (1/f = 1/u + 1/v), and experimental procedures must be correct per the KCSE Physics syllabus.
2. Ensure Form 2 scope — Electrostatics at Form 2 covers charging by induction and the Van de Graaff generator; capacitors are Form 3+. Reflection at Curved Surfaces covers concave and convex mirrors at Form 2.
3. Upgrade `example_answer.format` to `"svg"` for: ray diagrams for concave and convex mirrors (all principal ray types), moment and equilibrium diagrams, wave diagrams (transverse and longitudinal), magnetic field line diagrams, current-carrying conductor field patterns, Hooke's Law graphs. Create SVG files in `form2/physics/illustrations/`.
4. Set `answer_space_type: "diagram_box"` for all ray diagram and field line questions.
5. Set `answer_space_type: "grid_box"` for graph-plotting questions (Hooke's Law graph, wave graph).
6. Upgrade `example_answer.format` to `"tiptap"` for answers with SI units and multi-step calculations.
7. Calculation rubrics must follow formula + substitution + answer with units pattern.
8. Review `answer_lines` and correct.
9. Verify `type`, `difficulty`, `cognitive_level` — Physics must have strong `calculation`, `diagram`, and `experiment` representation.
10. Ensure each topic has at minimum 30 questions.
11. Fix atomization errors.
12. Consult `../materials/form-2/physics/` and KCSE Physics past papers for accuracy and coverage.

**Update after completion:**
- [ ] Mark this task `[x]`
- [ ] git commit: `fix(physics): content improvement pass Form 2`

---
