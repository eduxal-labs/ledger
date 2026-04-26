### Task 30: Form 3 Agriculture — Content Improvement
**Files to modify:** `form3/agriculture/*.json`
**Files in scope:**
- `agricultural-economics-ii-land-tenure-and-land-reform.json`
- `crop-pests-and-diseases.json`
- `crop-production-vi-field-practices-ii.json`
- `farm-structures.json`
- `forage-crops.json`
- `livestock-health-iii-diseases.json`
- `livestock-production-iii-selection-and-breeding.json`
- `livestock-production-iv-livestock-rearing.json`
- `soil-and-water-conservation.json`
- `weeds-and-weed-control.json`
**Depends on:** Task 01
**Parallel group:** P2

**Specification:**
Read `AGENT.md` in full before starting. For every topic JSON file listed above:
1. Verify factual accuracy of all questions, rubric criteria, and example answers against Form 3 Agriculture KCSE syllabus content. Correct any errors.
2. Ensure Form 3 scope only — do not overlap with Form 1/2 crop and livestock content.
3. Add `stimulus` objects for questions referencing pest and disease identification charts, soil conservation diagrams, farm structure plans, or breeding data tables.
4. Upgrade `example_answer.format` to `"svg"` for: farm structure cross-sections (cattle dip, store, greenhouse), soil erosion and conservation diagrams (terracing, contour farming), pest life-cycle diagrams, weed control method diagrams, livestock selection criteria flowcharts. Create SVG files in `form3/agriculture/illustrations/`.
5. Upgrade `example_answer.format` to `"image"` for complex livestock disease diagrams (tick species, internal parasite anatomy).
6. Review `answer_lines` values and correct using the word-count rule from §2.6.
7. Verify `type`, `difficulty`, `cognitive_level` — include `experiment` questions for soil conservation tests and `data_response` questions for livestock performance records.
8. Ensure each topic has at minimum 30 questions, including: identification questions, control methods, economic importance, and practical application scenarios.
9. Fix any atomization errors in rubric criteria — particularly for "state methods of controlling..." and "give reasons for..." type enumeration rubrics.
10. Consult `../materials/form-3/agriculture/` and `../kcse-past-papers/2016/43/` through `../kcse-past-papers/2024/43/` for accuracy and coverage.

**Update after completion:**
- [ ] Mark this task `[x]`
- [ ] git commit: `fix(agriculture): content improvement pass Form 3`

---

### Task 31: Form 3 Biology — Content Improvement
**Files to modify:** `form3/biology/*.json`
**Files in scope:**
- `classification-ii.json`
- `ecology.json`
- `growth-and-development.json`
- `reproduction-in-plants-and-animals.json`
**Depends on:** Task 01
**Parallel group:** P2

**Specification:**
Read `AGENT.md` in full before starting. For every topic JSON file listed above:
1. Verify factual accuracy — taxonomic classification (kingdom through species), ecological concepts (food webs, energy pyramids, nutrient cycles), growth curves (sigmoid, J-shaped), reproductive strategies, and germination processes must be accurate per the KCSE Biology syllabus.
2. Ensure Form 3 scope — Classification II covers Kingdoms Fungi, Plantae, and Animalia in detail; Ecology covers ecosystems, energy flow, and population dynamics; Reproduction covers both asexual and sexual reproduction across kingdoms.
3. Add `stimulus` objects for questions referencing ecological data tables, growth curve graphs, food web diagrams, and classification keys.
4. Upgrade `example_answer.format` to `"svg"` for: food web and food chain diagrams, energy pyramid diagrams, sigmoid growth curves, nitrogen/carbon cycle diagrams, flower structure diagrams, germination sequence diagrams. Create SVG files in `form3/biology/illustrations/`.
5. Upgrade `example_answer.format` to `"image"` for: complex organism anatomy (insect internal organs, mammalian reproductive system cross-section, seed structure). Add detailed `description` fields.
6. Include `data_response` questions with ecological data tables (population counts, energy transfer percentages, classification keys to work through).
7. Review `answer_lines` and correct.
8. Verify `type`, `difficulty`, `cognitive_level` — Biology Form 3 requires strong `experiment`, `structured`, and `data_response` representation.
9. Ensure each topic has at minimum 30 questions; Ecology and Classification II can sustain 50+ questions.
10. Fix atomization errors — particularly for "state adaptations of..." and "explain factors affecting..." rubrics.
11. Consult `../materials/form-3/biology/` and KCSE Biology past papers for accuracy and coverage.

**Update after completion:**
- [ ] Mark this task `[x]`
- [ ] git commit: `fix(biology): content improvement pass Form 3`

---

### Task 32: Form 3 Business Studies — Content Improvement
**Files to modify:** `form3/business-studies/*.json`
**Files in scope:**
- `books-of-original-entry.json`
- `business-finance.json`
- `demand-and-supply.json`
- `financial-statements.json`
- `inflation.json`
- `ledger-accounts-and-trial-balance.json`
- `money-and-banking.json`
- `national-income.json`
- `population-and-employment.json`
- `price-determination.json`
**Depends on:** Task 01
**Parallel group:** P2

**Specification:**
Read `AGENT.md` in full before starting. For every topic JSON file listed above:
1. Verify factual accuracy — accounting entries (debit/credit rules), financial statement formats (trading account, profit and loss account, balance sheet), economic concepts (demand and supply shifts, price determination, inflation types and causes) must be accurate per the KCSE Business Studies syllabus.
2. Ensure Form 3 scope — Books of Original Entry, Ledger Accounts, and Financial Statements are Form 3 Accounting content. National Income and Demand/Supply are Form 3 Economics content.
3. Add `stimulus` objects for questions referencing given ledger extracts, financial statement excerpts, demand/supply data tables, or national income calculation data.
4. Upgrade `body_format` to `"tiptap"` for questions presenting accounting data in table format (trial balance, ledger account T-format).
5. Upgrade `example_answer.format` to `"tiptap"` for answers presenting complete financial statements (properly formatted T-accounts, trading accounts, balance sheets).
6. Set `answer_space_type: "plain_box"` with generous `answer_box_height_mm` (120–160) for questions requiring students to draw up complete financial statements.
7. Include `calculation` questions for: national income calculations (GDP, GNP, NNP), demand and supply equilibrium price calculations, and profit/loss calculations from given data.
8. Include `data_response` questions with given financial data for analysis.
9. Review `answer_lines` and correct.
10. Verify `type`, `difficulty`, `cognitive_level`.
11. Ensure each topic has at minimum 30 questions.
12. Fix atomization errors.
13. Consult `../materials/form-3/business-studies/` and KCSE Business Studies past papers for accuracy and format conventions.

**Update after completion:**
- [ ] Mark this task `[x]`
- [ ] git commit: `fix(business-studies): content improvement pass Form 3`

---

### Task 33: Form 3 Chemistry — Content Improvement
**Files to modify:** `form3/chemistry/*.json`
**Files in scope:**
- `gas-laws.json`
- `nitrogen-and-its-compounds.json`
- `organic-chemistry-i.json`
- `reaction-rates.json`
- `sulphur-and-its-compounds.json`
- `the-mole.json`
**Depends on:** Task 01
**Parallel group:** P2

**Specification:**
Read `AGENT.md` in full before starting. For every topic JSON file listed above:
1. Verify factual accuracy — gas law equations (Boyle's, Charles', Combined), mole calculations (Avogadro's number = 6.022 × 10²³), organic compound structures (alkanes, alkenes, alcohols, carboxylic acids), reaction rate factors, Haber and Contact processes, and sulphuric acid/nitric acid industrial processes must be accurate per the KCSE Chemistry syllabus.
2. Ensure Form 3 scope — The Mole at Form 3 covers molar mass, mole ratios from equations, empirical formulae, molecular formulae, and concentration calculations; it does not cover electrochemistry mole calculations (Form 4).
3. Add `stimulus` objects for questions referencing reaction rate graphs, gas law experimental data tables, or organic structure diagrams provided for analysis.
4. Upgrade `example_answer.format` to `"svg"` for: apparatus setups for gas law experiments (gas syringe, manometer setups), organic molecular structure diagrams (displayed formulae, structural formulae), Haber process flow diagram, Contact process flow diagram, reaction rate graphs. Create SVG files in `form3/chemistry/illustrations/`.
5. Upgrade `example_answer.format` to `"tiptap"` for answers containing balanced equations with state symbols, mole calculations, and structural formula writing.
6. Set `answer_space_type: "grid_box"` for questions asking students to plot reaction rate graphs or gas law graphs.
7. Calculation rubrics must follow formula + substitution + answer with units pattern and allow follow-through marking.
8. Review `answer_lines` and correct.
9. Verify `type`, `difficulty`, `cognitive_level` — Chemistry Form 3 must have very strong `calculation` representation (mole calculations are the dominant KCSE question type for this level).
10. Ensure each topic has at minimum 30 questions; The Mole and Organic Chemistry I can sustain 50+ varied calculation and structural questions.
11. Fix atomization errors.
12. Consult `../materials/form-3/chemistry/` and KCSE Chemistry past papers for accuracy and calculation format conventions.

**Update after completion:**
- [ ] Mark this task `[x]`
- [ ] git commit: `fix(chemistry): content improvement pass Form 3`

---

### Task 34: Form 3 Computer Studies — Content Improvement
**Files to modify:** `form3/computer-studies/*.json`
**Files in scope:**
- `algorithms-flowcharts-and-pseudocode.json`
- `binary-arithmetic-and-coding-schemes.json`
- `data-processing.json`
- `data-representation-and-number-systems.json`
- `introduction-to-html-and-web-page-design.json`
- `programming-concepts-and-languages.json`
- `system-development-life-cycle.json`
**Depends on:** Task 01
**Parallel group:** P2

**Specification:**
Read `AGENT.md` in full before starting. For every topic JSON file listed above:
1. Verify factual accuracy — flowchart symbols (standard ANSI/ISO symbols), pseudocode conventions, HTML tags and their correct usage, number system conversions (binary, octal, hexadecimal, denary, BCD, Gray code), and SDLC phases must be accurate per the KCSE Computer Studies syllabus.
2. Ensure Form 3 scope — Programming concepts at Form 3 covers high-level vs low-level languages, generations of programming languages, and introduction to structured programming; it does not include advanced OOP or specific language syntax in depth.
3. Upgrade `example_answer.format` to `"svg"` for: flowcharts (complete flowcharts for algorithms), SDLC phase diagrams, data processing cycle diagrams. Create SVG files in `form3/computer-studies/illustrations/`.
4. Upgrade `example_answer.format` to `"tiptap"` for: pseudocode answers (using formatted code blocks), HTML code examples, number conversion working.
5. Set `answer_space_type: "plain_box"` with `answer_box_height_mm: 120` for flowchart drawing questions.
6. Include `calculation` questions for all number system conversion types and binary arithmetic (addition, subtraction using two's complement).
7. Review `answer_lines` and correct.
8. Verify `type`, `difficulty`, `cognitive_level`.
9. Ensure each topic has at minimum 30 questions.
10. Fix atomization errors.
11. Consult `../materials/form-3/computer-studies/` and KCSE Computer Studies past papers for accuracy and coverage.

**Update after completion:**
- [ ] Mark this task `[x]`
- [ ] git commit: `fix(computer-studies): content improvement pass Form 3`

---

### Task 35: Form 3 CRE — Content Improvement
**Files to modify:** `form3/cre/*.json`
**Files in scope:**
- `gifts-of-the-holy-spirit.json`
- `nehemiah.json`
- `prophet-amos.json`
- `prophet-jeremiah.json`
- `selected-old-testament-prophets.json`
- `unity-of-believers.json`
**Depends on:** Task 01
**Parallel group:** P2

**Specification:**
Read `AGENT.md` in full before starting. For every topic JSON file listed above:
1. Verify theological accuracy — prophecies of Amos and Jeremiah (specific references to chapters and verses), Nehemiah's rebuilding narrative, gifts of the Holy Spirit (1 Corinthians 12), and early church unity (Acts) must be accurate per the KCSE CRE syllabus.
2. Ensure Form 3 scope — New Testament themes (Holy Spirit, early church) and selected prophets (Amos, Jeremiah, Nehemiah) as specified for Form 3 CRE.
3. Add `stimulus` objects for questions referencing specific Bible passages provided for analysis or comparison.
4. Review `answer_lines` and correct.
5. Verify `type`, `difficulty`, `cognitive_level` — include application questions relating prophetic messages to social justice issues in contemporary Kenya, and questions on the relevance of the gifts of the Holy Spirit to the modern church.
6. Ensure each topic has at minimum 30 questions including: factual recall of events, explanation of teachings, comparison of prophets' messages, and contemporary application.
7. Fix atomization errors — enumeration rubrics (e.g., "state ways in which Nehemiah showed leadership") must list all valid answers.
8. Consult `../materials/form-3/cre/` and KCSE CRE past papers for accuracy and coverage.

**Update after completion:**
- [ ] Mark this task `[x]`
- [ ] git commit: `fix(cre): content improvement pass Form 3`

---

### Task 36: Form 3 English — Content Improvement
**Files to modify:** `form3/english/*.json`
**Files in scope:**
- `adjectives-quantifiers-predicative-and-attributive-use.json`
- `adverbs-formation-and-functions.json`
- `clauses-noun-adjective-and-conditional-clauses.json`
- `comprehension-and-summary-writing.json`
- `direct-and-indirect-reported-speech.json`
- `etiquette-and-register.json`
- `formal-letters-and-letters-of-application.json`
- `group-discussion-skills.json`
- `idiomatic-expressions.json`
- `imaginative-composition.json`
- `institutional-writing-notices-agendas-minutes-and-memoranda.json`
- `noun-derivation-and-gender-sensitive-language.json`
- `oral-literature.json`
- `participles.json`
- `personal-and-social-writing.json`
- `phrasal-verbs.json`
- `phrases-adjective-and-prepositional-phrases.json`
- `prepositions-conjunctions-and-connectors.json`
- `pronouns.json`
- `pronunciation-stress-and-intonation.json`
- `report-writing-and-argumentative-essays.json`
- `rhythm-assonance-and-alliteration-in-poetry.json`
- `sentence-building-paragraphing-and-punctuation.json`
- `speech-writing-and-delivery.json`
- `verbs-transitive-intransitive-and-infinitives.json`
**Depends on:** Task 01
**Parallel group:** P2

**Specification:**
Read `AGENT.md` in full, especially §11.6 (English Language Question Formats), before starting.
1. Verify linguistic accuracy — reported speech transformations (tense backshift, pronoun changes, time expressions), conditional clause types (zero, first, second, third), participial phrase usage, and institutional document formats (agenda, minutes, memo, notice) must be accurate per standard English grammar and KCSE conventions.
2. Ensure Form 3 scope — Application letters and CVs are Form 3 functional writing. Reports and argumentative essays are Form 3. Institutional writing (notices, agendas, minutes, memoranda) is Form 3.
3. Add `stimulus` objects for all passage-based questions: comprehension passages, oral literature items (poems, narratives, dialogues), and documents provided for analysis.
4. For oral literature questions: embed the full poem, narrative, or oral literature item in the `body` or `stimulus`. Follow KCSE oral literature question format precisely (see §11.6c–§11.6e).
5. For comprehension and summary: ensure full passage is embedded in `stimulus`; sub-questions must follow the KCSE pattern from §11.6b.
6. For institutional writing (agenda, minutes, memo, notice): verify that format conventions match standard Kenyan business/institutional practice as tested in KCSE.
7. For imaginative composition: prompts must include two options (narrative with prescribed ending + expository/argumentative topic). Rubrics use holistic band descriptors.
8. Grammar questions use `type: "structured"` with each item as a part.
9. Review `answer_lines` — writing tasks (letters, reports, speeches, compositions) need `answer_space_type: "plain_box"` with `answer_box_height_mm: 160–200`.
10. Ensure each topic has at minimum 30 questions; comprehension, grammar, and composition topics should aim for 40+.
11. Fix atomization errors. Note: writing task rubrics must NOT be atomized — keep them as holistic descriptors.
12. Consult `../materials/form-3/english/` and KCSE English Paper 1 and Paper 2 past papers.

**Update after completion:**
- [ ] Mark this task `[x]`
- [ ] git commit: `fix(english): content improvement pass Form 3`

---

### Task 37: Form 3 Geography — Content Improvement
**Files to modify:** `form3/geography/*.json`
**Files in scope:**
- `action-of-rivers.json`
- `action-of-water-in-limestone-regions.json`
- `action-of-wind-and-water-in-arid-areas.json`
- `agriculture.json`
- `glaciation.json`
- `hydrological-cycle.json`
- `lakes.json`
- `map-work.json`
- `mass-wasting.json`
- `oceans-seas-and-coasts.json`
- `soil.json`
- `statistics.json`
- `underground-water.json`
- `weathering.json`
**Depends on:** Task 01
**Parallel group:** P2

**Specification:**
Read `AGENT.md` in full before starting. For every topic JSON file listed above:
1. Verify factual accuracy — landform formation processes (fluvial, glacial, aeolian, karst), ocean current systems, soil formation and classification (zonal soils), hydrological cycle stages, and agricultural systems in Kenya and Africa must be accurate per the KCSE Geography syllabus.
2. Ensure Form 3 scope — Rivers, Glaciation, Limestone, Arid and Semi-Arid landforms, Coasts, and related processes are Form 3 Physical Geography content.
3. Add `stimulus` objects for questions referencing topographic map extracts, cross-section diagrams, photograph descriptions, or statistical data on agricultural output.
4. Upgrade `example_answer.format` to `"svg"` for: river landform diagrams (meander, oxbow lake, waterfall, delta, V-shaped valley), glacial landforms (U-shaped valley, corrie, arête, drumlin, moraine), coastal landforms (cliff, wave-cut platform, beach, spit, bay), karst features (stalactite, stalagmite, swallow hole, polje), aeolian features (barchan, seif dune, yardang), hydrological cycle diagram. Create SVG files in `form3/geography/illustrations/`.
5. Set `answer_space_type: "diagram_box"` for diagram-drawing questions, `"grid_box"` for graph-plotting and statistics questions.
6. Include `data_response` questions with map-reading tasks (bearing calculations, grid references, cross-sections), photograph analysis questions, and statistical analysis tasks.
7. Review `answer_lines` and correct.
8. Verify `type`, `difficulty`, `cognitive_level` — Geography Form 3 requires very strong `diagram`, `data_response`, and `explanation` type coverage.
9. Ensure each topic has at minimum 30 questions; Map Work and major landform topics can sustain 50+ questions.
10. Fix atomization errors — particularly for "explain the formation of..." rubrics which must award 1 mark per distinct process step.
11. Consult `../materials/form-3/geography/` and KCSE Geography past papers for accuracy and format.

**Update after completion:**
- [ ] Mark this task `[x]`
- [ ] git commit: `fix(geography): content improvement pass Form 3`

---

### Task 38: Form 3 History and Government — Content Improvement
**Files to modify:** `form3/history/*.json`
**Files in scope:**
- `colonial-administration-in-kenya.json`
- `establishment-of-colonial-rule-in-kenya.json`
- `european-invasion-of-africa.json`
- `lives-and-contributions-of-kenyan-leaders.json`
- `rise-of-african-nationalism.json`
- `social-and-economic-developments-during-colonial-period.json`
- `struggle-for-independence-in-kenya.json`
- `the-judiciary-in-kenya.json`
- `the-legislature.json`
- `the-national-executive.json`
**Depends on:** Task 01
**Parallel group:** P2

**Specification:**
Read `AGENT.md` in full before starting. For every topic JSON file listed above:
1. Verify historical accuracy — facts about colonial administration systems (direct rule, indirect rule, assimilation), African nationalist movements, specific dates and names of Kenyan independence leaders (Jomo Kenyatta, Oginga Odinga, Tom Mboya, Dedan Kimathi), and the structure of Kenya's government must be accurate per the KCSE History and Government syllabus.
2. Ensure Form 3 scope — colonialism, African nationalism, Kenyan independence struggle, and Kenya's government structure (judiciary, legislature, executive) at Form 3 level.
3. For government structure topics (judiciary, legislature, executive): ensure constitutional accuracy per Kenya's 2010 Constitution.
4. Add SVG illustrations where genuinely useful: colonial administrative map of Africa, timeline of Kenyan independence milestones.
5. Add `stimulus` objects for questions referencing historical source documents, speeches, or data on African nationalism.
6. Review `answer_lines` and correct.
7. Verify `type`, `difficulty`, `cognitive_level` — include application questions relating colonial history lessons to contemporary African governance challenges.
8. Ensure each topic has at minimum 30 questions.
9. Fix atomization errors — History enumeration rubrics must list all valid answers as separate criteria with `max_marks`.
10. Consult `../materials/form-3/history/` and KCSE History past papers for accuracy and coverage.

**Update after completion:**
- [ ] Mark this task `[x]`
- [ ] git commit: `fix(history): content improvement pass Form 3`

---

### Task 39: Form 3 Home Science — Content Improvement
**Files to modify:** `form3/home-science/*.json`
**Files in scope:**
- `advanced-garment-construction.json`
- `advanced-nutrition-and-diet-therapy.json`
- `entrepreneurship-in-home-science.json`
- `fabric-finishes-and-textile-care.json`
- `family-resource-management.json`
- `food-science-and-technology.json`
- `housing-design-and-construction.json`
- `prenatal-and-postnatal-care.json`
**Depends on:** Task 01
**Parallel group:** P2

**Specification:**
Read `AGENT.md` in full before starting. For every topic JSON file listed above:
1. Verify factual accuracy — dietary reference values (RDAs), therapeutic diet specifications (diabetic, renal, cardiac diets), fabric finish chemicals and their effects, food science principles (emulsification, gelatinisation, coagulation), prenatal/postnatal care milestones, and housing construction terminology must be accurate per the KCSE Home Science syllabus.
2. Ensure Form 3 scope only — advanced content building on Forms 1 and 2 foundations.
3. Add `stimulus` objects for questions referencing nutritional composition tables, fabric care symbols, prenatal care schedule charts, or floor plan diagrams.
4. Upgrade `example_answer.format` to `"svg"` for: advanced garment construction techniques (placket, collar, zip insertion diagrams), housing floor plan layouts, food preservation equipment diagrams, fabric weave diagrams. Create SVG files in `form3/home-science/illustrations/`.
5. Review `answer_lines` and correct.
6. Verify `type`, `difficulty`, `cognitive_level` — include application/scenario questions (e.g., planning a therapeutic diet for a given patient profile).
7. Ensure each topic has at minimum 30 questions.
8. Fix atomization errors.
9. Consult `../materials/form-3/home-science/` and KCSE Home Science past papers for accuracy and coverage.

**Update after completion:**
- [ ] Mark this task `[x]`
- [ ] git commit: `fix(home-science): content improvement pass Form 3`

---

### Task 40: Form 3 IRE — Content Improvement
**Files to modify:** `form3/ire/*.json`
**Files in scope:**
- `hadith-literature-and-scholars.json`
- `history-of-islam-in-east-africa.json`
- `islamic-ethics-and-akhlaq.json`
- `islamic-law-of-inheritance.json`
- `khulafaa-rashidun.json`
- `muamalat-islamic-commercial-law.json`
- `muslim-contributions-to-civilisation.json`
- `schools-of-islamic-jurisprudence.json`
- `selected-surahs-and-their-teachings.json`
- `ulumul-quran.json`
**Depends on:** Task 01
**Parallel group:** P2

**Specification:**
Read `AGENT.md` in full before starting. For every topic JSON file listed above:
1. Verify religious and historical accuracy — the four Khulafaa Rashidun (Abu Bakr, Umar, Uthman, Ali) and their specific contributions, the four major schools of jurisprudence (Hanafi, Maliki, Shafi'i, Hanbali) and their founders, Islamic inheritance law shares (fard shares), Hadith classification terminology (sahih, hasan, daif, mutawatir, ahad), and Islamic contributions to science and civilisation must be accurate per the KCSE IRE syllabus.
2. Ensure Form 3 scope — Khulafaa Rashidun, Islamic jurisprudence schools, Hadith science, Ulumul Quran, and Islamic history in East Africa are Form 3 content.
3. Add `stimulus` objects for questions referencing Quranic verses or Hadith texts provided for classification or analysis.
4. For Islamic law of inheritance: include `calculation` questions requiring students to calculate shares from a given estate with named heirs.
5. Review `answer_lines` and correct.
6. Verify `type`, `difficulty`, `cognitive_level` — include application questions relating Islamic commercial law principles to contemporary financial transactions faced by Kenyan Muslims.
7. Ensure each topic has at minimum 30 questions.
8. Fix atomization errors.
9. Consult `../materials/form-3/ire/` and KCSE IRE past papers for accuracy and coverage.

**Update after completion:**
- [ ] Mark this task `[x]`
- [ ] git commit: `fix(ire): content improvement pass Form 3`

---

### Task 41: Form 3 Kiswahili — Content Improvement
**Files to modify:** `form3/kiswahili/*.json`
**Files in scope:**
- `fasihi-simulizi-ngano-na-hekaya.json`
- `isimu-jamii-sajili-na-lahaja.json`
- `kumbukumbu-za-mkutano-na-insha-ya-methali.json`
- `mofimu-na-aina-zake.json`
- `ripoti.json`
- `uchanganuzi-wa-sentensi.json`
- `ufahamu.json`
- `ufupisho.json`
- `unyambulishaji.json`
- `ushairi-aina-za-mashairi.json`
- `utohozi-na-taathira-za-lugha.json`
- `visawe-kinyume-polisemia-na-homonimi.json`
**Depends on:** Task 01
**Parallel group:** P2

**Specification:**
Read `AGENT.md` in full before starting. All questions must be in correct Kiswahili sanifu.
1. Verify linguistic accuracy — morphological analysis (mofimu: kiima, kiishio, kirejeshi, mnyambuliko wa vitenzi), sentence analysis (uchanganuzi: kishazi huru, kishazi tegemezi), word derivation (unyambulishaji), loan word analysis (utohozi), sociolinguistic concepts (sajili, lahaja), and types of poetry (mashairi ya kimapokeo, mashairi huru) must be accurate per the KCSE Kiswahili syllabus.
2. Ensure Form 3 scope — advanced morphology, sociolinguistics, and complex literary analysis appropriate to Form 3.
3. For ufahamu and ufupisho: embed full passages in `stimulus`. Follow KCSE Kiswahili format precisely.
4. For ushairi (poetry) analysis: embed the complete shairi before sub-questions. Form 3 poetry questions should include deeper analysis of structure (mizani, vina, mishororo, beti) and thematic analysis.
5. For kumbukumbu za mkutano: verify that minute-writing format follows standard Kiswahili institutional conventions.
6. For insha ya methali: include proverbs and require students to write an essay inspired by the proverb's message.
7. Add `stimulus` objects for all passage-based questions.
8. Review `answer_lines` and correct.
9. Verify `type`, `difficulty`, `cognitive_level`.
10. Ensure each topic has at minimum 30 questions.
11. Fix atomization errors.
12. Consult `../materials/form-3/kiswahili/` and KCSE Kiswahili past papers for accuracy and style.

**Update after completion:**
- [ ] Mark this task `[x]`
- [ ] git commit: `fix(kiswahili): content improvement pass Form 3`

---

### Task 42: Form 3 Mathematics — Content Improvement
**Files to modify:** `form3/mathematics/*.json`
**Files in scope:**
- `approximation-and-errors.json`
- `binomial-expansion.json`
- `circles-chords-and-tangents.json`
- `commercial-arithmetic-ii.json`
- `compound-proportions-and-rates-of-work.json`
- `formulae-and-variation.json`
- `further-logarithms.json`
- `graphical-methods.json`
- `loci.json`
- `matrices.json`
- `probability.json`
- `quadratic-expressions-and-equations.json`
- `sequences-and-series.json`
- `surds.json`
- `three-dimensional-geometry.json`
- `trigonometry-ii.json`
- `vectors-ii.json`
**Depends on:** Task 01
**Parallel group:** P2

**Specification:**
Read `AGENT.md` in full before starting. For every topic JSON file listed above:
1. Verify mathematical accuracy — all calculations, proofs, matrix operations, trigonometric identities (sin²θ + cos²θ = 1, compound angle formulae), locus constructions, binomial expansions, and geometric proofs must be correct.
2. Ensure Form 3 scope — Trigonometry II at Form 3 covers sine rule, cosine rule, and area of triangle formula. Three-Dimensional Geometry covers angles between lines, planes, and diagonals. Matrices cover 2×2 determinants and inverses.
3. Upgrade `example_answer.format` to `"svg"` for: locus diagrams, three-dimensional geometry figures (cuboids, prisms with angles marked), circle theorem diagrams (chords, tangents, tangent-radius relationships), vector diagrams, trigonometry oblique triangle diagrams. Create SVG files in `form3/mathematics/illustrations/`.
4. Set `answer_space_type: "grid_box"` for all graph-plotting questions (graphical methods, probability tree diagrams on grid).
5. Set `answer_space_type: "construction_box"` for locus and geometric construction questions.
6. Upgrade `example_answer.format` to `"tiptap"` for answers with multi-step algebraic manipulation, matrix calculations, and logarithm proofs.
7. Calculation rubrics must follow formula + substitution + answer pattern with follow-through marking.
8. Include proof questions ("Show that...", "Prove that...") for circle theorems and trigonometric identities with rubrics awarding marks per logical step.
9. Review `answer_lines` and correct.
10. Verify `type`, `difficulty`, `cognitive_level` — Mathematics Form 3 is heavily `calculation` with good `diagram` and `structured` representation.
11. Ensure each topic has at minimum 30 questions with graduated difficulty from straightforward to KCSE-standard multi-step problems.
12. Fix atomization errors.
13. Consult `../materials/form-3/mathematics/` and KCSE Mathematics past papers for accuracy and style.

**Update after completion:**
- [ ] Mark this task `[x]`
- [ ] git commit: `fix(mathematics): content improvement pass Form 3`

---

### Task 43: Form 3 Physics — Content Improvement
**Files to modify:** `form3/physics/*.json`
**Files in scope:**
- `current-electricity-ii.json`
- `electrostatics-ii.json`
- `gas-laws.json`
- `heating-effect-of-electric-current.json`
- `linear-motion.json`
- `newtons-laws-of-motion.json`
- `quantity-of-heat.json`
- `refraction-of-light.json`
- `waves-ii.json`
- `work-energy-and-power.json`
**Depends on:** Task 01
**Parallel group:** P2

**Specification:**
Read `AGENT.md` in full before starting. For every topic JSON file listed above:
1. Verify factual accuracy — Ohm's law and its limitations, Boyle's and Charles' laws (with correct mathematical expressions), Newton's laws of motion (all three, with correct statements), equations of motion (v = u + at, s = ut + ½at², v² = u² + 2as), refraction laws (Snell's law n = sin i / sin r), wave properties (v = fλ), specific heat capacity and latent heat calculations, and electrical power/energy calculations must be accurate per the KCSE Physics syllabus.
2. Ensure Form 3 scope — Refraction of Light at Form 3 covers prisms, total internal reflection, and optical fibers. Electrostatics II covers capacitors and electric fields. Current Electricity II covers complex circuits with EMF and internal resistance.
3. Upgrade `example_answer.format` to `"svg"` for: refraction ray diagrams (prism, total internal reflection, critical angle), circuit diagrams with internal resistance (EMF diagrams), capacitor diagrams, velocity-time graphs, displacement-time graphs, wave diagrams (interference, diffraction), gas law experimental apparatus setups. Create SVG files in `form3/physics/illustrations/`.
4. Set `answer_space_type: "diagram_box"` for all ray diagram and circuit diagram drawing questions.
5. Set `answer_space_type: "grid_box"` for all graph-plotting questions (v-t graphs, s-t graphs, gas law graphs, cooling curves).
6. Calculation rubrics must follow formula + substitution + answer with units pattern. Allow follow-through from earlier parts.
7. Include `experiment` questions for: verifying Newton's second law, determining specific heat capacity of a metal, verifying Hooke's Law, determining the refractive index of glass.
8. Review `answer_lines` and correct.
9. Verify `type`, `difficulty`, `cognitive_level` — Physics Form 3 must have very strong `calculation`, `diagram`, and `experiment` type coverage.
10. Ensure each topic has at minimum 30 questions.
11. Fix atomization errors.
12. Consult `../materials/form-3/physics/` and KCSE Physics past papers for accuracy and coverage.

**Update after completion:**
- [ ] Mark this task `[x]`
- [ ] git commit: `fix(physics): content improvement pass Form 3`

---

### Task 44: Form 4 Agriculture — Content Improvement
**Files to modify:** `form4/agriculture/*.json`
**Files in scope:**
- `agricultural-economics-iii-production-economics.json`
- `agricultural-economics-iv-farm-accounts.json`
- `agricultural-economics-v-marketing-and-organizations.json`
- `agroforestry.json`
- `farm-power-and-machinery.json`
- `livestock-production-v-poultry.json`
- `livestock-production-vi-cattle.json`
**Depends on:** Task 01
**Parallel group:** P2

**Specification:**
Read `AGENT.md` in full before starting. For every topic JSON file listed above:
1. Verify factual accuracy — farm economic concepts (gross margin, net margin, opportunity cost, economies of scale), farm record formats (cash flow statement, farm inventory), marketing cooperative structures, agroforestry systems (alley cropping, windbreaks, silvo-pastoral), poultry and cattle management practices, and machinery maintenance procedures must be accurate per the KCSE Agriculture syllabus.
2. Ensure Form 4 scope — this is the culminating year; questions should be at the highest difficulty level and integrate knowledge from Forms 1–4.
3. Add `stimulus` objects for questions referencing farm account data tables, production economics data (cost curves), marketing scenario descriptions, or machinery specification tables.
4. Upgrade `example_answer.format` to `"svg"` for: farm machinery diagrams (tractor components, tillage implements), agroforestry system layout diagrams, poultry housing plans, cash flow statement formats. Create SVG files in `form4/agriculture/illustrations/`.
5. Upgrade `example_answer.format` to `"tiptap"` for answers presenting farm accounts in table format.
6. Include `calculation` questions for: gross margin calculation, break-even analysis, farm budget preparation from given data.
7. Review `answer_lines` and correct.
8. Verify `type`, `difficulty`, `cognitive_level` — Form 4 Agriculture should have predominantly difficulty 3–5 questions.
9. Ensure each topic has at minimum 30 questions integrating economic analysis with practical management decisions.
10. Fix atomization errors.
11. Consult `../materials/form-4/agriculture/` and `../kcse-past-papers/2016/44/` through `../kcse-past-papers/2024/44/` for accuracy and coverage.

**Update after completion:**
- [ ] Mark this task `[x]`
- [ ] git commit: `fix(agriculture): content improvement pass Form 4`

---

### Task 45: Form 4 Biology — Content Improvement
**Files to modify:** `form4/biology/*.json`
**Files in scope:**
- `evolution.json`
- `genetics.json`
- `reception-response-and-coordination.json`
- `support-and-movement.json`
**Depends on:** Task 01
**Parallel group:** P2

**Specification:**
Read `AGENT.md` in full before starting. For every topic JSON file listed above:
1. Verify factual accuracy — Mendelian genetics (monohybrid and dihybrid crosses, law of segregation, law of independent assortment), mutation types, Darwin's theory of natural selection, evidence for evolution (fossil record, comparative anatomy, comparative embryology, molecular biology), nervous system (neuron structure, synaptic transmission, reflex arc), endocrine system (hormone names, glands, target organs, effects), skeletal and muscular system must be accurate per the KCSE Biology syllabus.
2. Ensure Form 4 scope — this is the highest complexity level. Genetics includes sex-linked inheritance (haemophilia, colour blindness), codominance (ABO blood groups), and mutation.
3. Add `stimulus` objects for questions referencing genetic cross diagrams, pedigree charts, evolutionary evidence data tables, nerve impulse graphs, or hormone concentration graphs.
4. Upgrade `example_answer.format` to `"svg"` for: Punnett square diagrams, pedigree charts, neuron structure diagrams, reflex arc diagrams, synapse diagrams, skeletal diagrams (types of joints), DNA structure diagrams. Create SVG files in `form4/biology/illustrations/`.
5. Upgrade `example_answer.format` to `"image"` for: brain cross-section (regions labelled), eye and ear cross-sections, muscle ultrastructure (sarcomere). Add detailed `description` fields.
6. Include `calculation` questions for: genetic ratios from crosses, probability of offspring genotypes, and Hardy-Weinberg calculations where applicable.
7. Review `answer_lines` and correct.
8. Verify `type`, `difficulty`, `cognitive_level` — Form 4 Biology must have very strong `structured` (multi-part synthesis questions) and `calculation` (genetics) representation.
9. Ensure each topic has at minimum 30 questions; Genetics and Reception/Response can sustain 60+ questions given their breadth.
10. Fix atomization errors — particularly for "explain the mechanism of..." rubrics.
11. Consult `../materials/form-4/biology/` and KCSE Biology past papers for accuracy and coverage.

**Update after completion:**
- [ ] Mark this task `[x]`
- [ ] git commit: `fix(biology): content improvement pass Form 4`

---

### Task 46: Form 4 Business Studies — Content Improvement
**Files to modify:** `form4/business-studies/*.json`
**Files in scope:**
- `consumer-protection.json`
- `economic-development-and-planning.json`
- `emerging-trends-in-business.json`
- `financial-markets.json`
- `insurance.json`
- `international-trade.json`
- `marketing.json`
- `product-promotion.json`
- `public-finance.json`
- `the-business-plan.json`
**Depends on:** Task 01
**Parallel group:** P2

**Specification:**
Read `AGENT.md` in full before starting. For every topic JSON file listed above:
1. Verify factual accuracy — insurance principles at Form 4 level (including marine insurance, life assurance, reinsurance), international trade concepts (balance of trade, balance of payments, WTO, COMESA, EAC), financial market instruments (stocks, bonds, treasury bills, NSE), public finance concepts (taxation types, government budget, public debt), marketing mix (4Ps), and emerging ICT trends in business must be accurate per the KCSE Business Studies syllabus.
2. Note that `insurance.json` covers Form 4 insurance content (more advanced than Form 2 insurance). Ensure the questions are appropriately scoped and do not duplicate Form 2 content.
3. Add `stimulus` objects for questions referencing balance of payments tables, government budget extracts, NSE data, or business plan excerpts.
4. Upgrade `body_format` to `"tiptap"` for questions presenting financial data in tables.
5. Include `calculation` questions for: balance of payments calculations, taxation calculations (income tax, VAT), and trade balance calculations.
6. Include `data_response` questions with given economic or financial data for analysis.
7. Review `answer_lines` and correct.
8. Verify `type`, `difficulty`, `cognitive_level` — Form 4 questions should be predominantly difficulty 3–5 with strong analysis and evaluation components.
9. Ensure each topic has at minimum 30 questions.
10. Fix atomization errors.
11. Consult `../materials/form-4/business-studies/` and KCSE Business Studies past papers for accuracy and coverage.

**Update after completion:**
- [ ] Mark this task `[x]`
- [ ] git commit: `fix(business-studies): content improvement pass Form 4`

---

### Task 47: Form 4 Chemistry — Content Improvement
**Files to modify:** `form4/chemistry/*.json`
**Files in scope:**
- `acids-bases-and-salts.json`
- `electrochemistry.json`
- `energy-changes-in-chemical-and-physical-processes.json`
- `metals.json`
- `organic-chemistry-ii.json`
- `radioactivity.json`
- `reaction-rates-and-reversible-reactions.json`
**Depends on:** Task 01
**Parallel group:** P2

**Specification:**
Read `AGENT.md` in full before starting. For every topic JSON file listed above:
1. Verify factual accuracy — electrochemistry (Faraday's laws, electrode reactions, standard electrode potentials, electrochemical series), enthalpy changes (Hess's Law, bond energies), radioactivity (types of radiation, half-life, decay equations), organic chemistry II (esters, soaps and detergents, polymers, carbohydrates, proteins), metals (reactivity series, extraction processes — blast furnace, Thermite, electrolysis of alumina), and equilibrium (Le Chatelier's principle, Kc expressions) must be accurate per the KCSE Chemistry syllabus.
2. Ensure Form 4 scope — this is the most advanced Chemistry year. Include complex multi-step calculations and synthesis questions.
3. Add `stimulus` objects for questions referencing decay graphs, energy level diagrams, electrochemical cell diagrams, or organic reaction pathway data.
4. Upgrade `example_answer.format` to `"svg"` for: electrochemical cell diagrams (Daniell cell, electrolytic cell), blast furnace diagram, nuclear decay equation diagrams, organic functional group structure diagrams, energy profile diagrams (exothermic/endothermic). Create SVG files in `form4/chemistry/illustrations/`.
5. Upgrade `example_answer.format` to `"tiptap"` for answers containing balanced ionic equations, half-equations, Hess's Law energy cycles, and organic mechanism arrows.
6. Set `answer_space_type: "grid_box"` for questions asking students to plot half-life graphs or energy diagrams.
7. Calculation rubrics must follow formula + substitution + answer with units pattern; include Faraday constant (96,500 C/mol) calculations.
8. Review `answer_lines` and correct.
9. Verify `type`, `difficulty`, `cognitive_level` — Chemistry Form 4 must have the highest difficulty distribution; predominantly difficulty 3–5.
10. Ensure each topic has at minimum 30 questions; Electrochemistry and Organic Chemistry II can sustain 50+ questions.
11. Fix atomization errors.
12. Consult `../materials/form-4/chemistry/` and KCSE Chemistry past papers for accuracy and coverage.

**Update after completion:**
- [ ] Mark this task `[x]`
- [ ] git commit: `fix(chemistry): content improvement pass Form 4`

---

### Task 48: Form 4 Computer Studies — Content Improvement
**Files to modify:** `form4/computer-studies/*.json`
**Files in scope:**
- `advanced-programming.json`
- `career-opportunities-in-ict.json`
- `computer-ethics-and-legal-issues.json`
- `data-security-and-data-control.json`
- `disk-management.json`
- `emerging-trends-in-ict.json`
- `system-development-life-cycle.json`
- `web-design-and-development.json`
**Depends on:** Task 01
**Parallel group:** P2

**Specification:**
Read `AGENT.md` in full before starting. For every topic JSON file listed above:
1. Verify factual accuracy — programming constructs (variables, data types, control structures, subprograms/procedures/functions), SDLC methodologies (waterfall, agile, spiral), web design standards (HTML5, CSS basics), data security measures (encryption, firewalls, access controls), computer ethics principles, and emerging ICT trends (AI, cloud computing, IoT, blockchain) must be accurate per the KCSE Computer Studies syllabus.
2. Ensure Form 4 scope — Advanced Programming covers structured programming with subprograms, arrays, and file handling at the Form 4 level.
3. Upgrade `example_answer.format` to `"svg"` for: system development life cycle diagrams, program flowcharts for advanced algorithms, network security architecture diagrams, web page layout wireframes. Create SVG files in `form4/computer-studies/illustrations/`.
4. Upgrade `example_answer.format` to `"tiptap"` for: pseudocode answers, HTML/CSS code snippets, program trace table answers.
5. Set `answer_space_type: "plain_box"` with `answer_box_height_mm: 140` for programming questions requiring pseudocode or code writing.
6. Include `calculation` questions for: program trace tables (tracing through code step by step), file size and storage calculations.
7. Review `answer_lines` and correct.
8. Verify `type`, `difficulty`, `cognitive_level`.
9. Ensure each topic has at minimum 30 questions.
10. Fix atomization errors.
11. Consult `../materials/form-4/computer-studies/` and KCSE Computer Studies past papers for accuracy and coverage.

**Update after completion:**
- [ ] Mark this task `[x]`
- [ ] git commit: `fix(computer-studies): content improvement pass Form 4`

---

### Task 49: Form 4 CRE — Content Improvement
**Files to modify:** `form4/cre/*.json`
**Files in scope:**
- `christian-approaches-to-human-sexuality-marriage-and-family.json`
- `christian-approaches-to-law-order-and-justice.json`
- `christian-approaches-to-leisure.json`
- `christian-approaches-to-science-technology-and-environment.json`
- `christian-approaches-to-wealth-money-and-poverty.json`
- `christian-approaches-to-work.json`
- `introduction-to-christian-ethics.json`
**Depends on:** Task 01
**Parallel group:** P2

**Specification:**
Read `AGENT.md` in full before starting. For every topic JSON file listed above:
1. Verify theological and ethical accuracy — Christian ethical principles (Natural Law, Situation Ethics, Biblical ethics), biblical teachings on marriage, family, sexuality, wealth, work, leisure, justice, and environmental stewardship, and the teachings of major Christian traditions (Catholic, Protestant, African Independent Churches) on these themes must be accurate per the KCSE CRE syllabus.
2. Ensure Form 4 scope — Christian Ethics as applied to contemporary social issues is the Form 4 CRE focus. Questions must integrate biblical teaching with analysis of contemporary Kenyan social challenges.
3. Add `stimulus` objects for questions referencing case studies, news scenarios, or Bible passages provided for ethical analysis.
4. Include analysis-level questions requiring students to evaluate different Christian responses to a contemporary ethical dilemma (e.g., Christian approaches to environmental degradation, corruption, HIV/AIDS).
5. Review `answer_lines` and correct.
6. Verify `type`, `difficulty`, `cognitive_level` — Form 4 CRE must have strong `analysis` level questions; difficulty should be predominantly 3–5.
7. Ensure each topic has at minimum 30 questions, covering: biblical teaching, traditional African perspectives, contemporary Christian views, and personal application.
8. Fix atomization errors.
9. Consult `../materials/form-4/cre/` and KCSE CRE past papers for accuracy and coverage.

**Update after completion:**
- [ ] Mark this task `[x]`
- [ ] git commit: `fix(cre): content improvement pass Form 4`

---

### Task 50: Form 4 English — Content Improvement
**Files to modify:** `form4/english/*.json`
**Files in scope:**
- `cloze-tests.json`
- `comprehension-and-summary.json`
- `creative-and-expository-writing.json`
- `functional-writing.json`
- `grammar-parts-of-speech.json`
- `grammar-sentence-structure-and-transformation.json`
- `literary-appreciation-and-poetry-analysis.json`
- `oral-literature.json`
- `oral-skills-and-etiquette.json`
- `reading-skills-and-critical-analysis.json`
- `vocabulary-and-word-formation.json`
**Depends on:** Task 01
**Parallel group:** P2

**Specification:**
Read `AGENT.md` in full, especially §11.6 (English Language Question Formats), before starting.
1. Verify linguistic and literary accuracy — all grammar rules, comprehension passage sub-questions, poetry analysis questions, and functional writing format conventions must reflect the highest KCSE standard (Paper 1, Paper 2, Paper 3 conventions at Form 4 level).
2. Ensure Form 4 scope — this is the final year; questions must integrate all language skills and be at the full KCSE difficulty level.
3. For comprehension: ensure full 400–600 word passages are embedded in `stimulus`. Sub-questions must follow the complete KCSE pattern from §11.6b including summary, sentence transformation, and vocabulary in context.
4. For cloze tests: ensure each passage has exactly 10 blanks (a)–(j), reads as coherent prose, and tests a range of grammar skills. Form 4 cloze passages should be on more sophisticated topics.
5. For poetry analysis: embed complete poems. Include both traditional forms and free verse. Analysis questions must cover all KCSE poetry sub-question types from §11.6c.
6. For oral literature: embed complete oral narrative texts or full riddling dialogues. Questions must follow §11.6d–§11.6i format precisely.
7. For functional writing: include the full range of advanced document types (reports, speeches, memoranda, minutes, application letters, CVs, emails). Rubrics use holistic band descriptors.
8. For grammar: complex sentence transformations at Form 4 level including all types from §11.6f (passive voice, reported speech, inversions, concessions, conditionals).
9. Review `answer_lines` — writing tasks need `answer_space_type: "plain_box"` with `answer_box_height_mm: 180–220` for full compositions.
10. Ensure each topic has at minimum 30 questions; comprehension and grammar topics should have 50+.
11. Fix atomization errors. Writing task rubrics must NOT be atomized; use holistic descriptors.
12. Consult `../materials/form-4/english/` and all available KCSE English Paper 1, Paper 2, and Paper 3 past papers.

**Update after completion:**
- [ ] Mark this task `[x]`
- [ ] git commit: `fix(english): content improvement pass Form 4`

---

### Task 51: Form 4 Geography — Content Improvement
**Files to modify:** `form4/geography/*.json`
**Files in scope:**
- `energy.json`
- `fishing.json`
- `industry.json`
- `land-reclamation.json`
- `management-and-conservation-of-the-environment.json`
- `population.json`
- `trade.json`
- `transport-and-communication.json`
- `urbanisation.json`
- `wildlife-and-tourism.json`
**Depends on:** Task 01
**Parallel group:** P2

**Specification:**
Read `AGENT.md` in full before starting. For every topic JSON file listed above:
1. Verify factual accuracy — population statistics and demographic transition model stages, major world industrial regions (Ruhr, North-East USA, Japanese industrial belt), energy sources and production data (Kenya's energy mix, world oil reserves), fishing grounds and methods, land reclamation examples (Netherlands, Israel, Japan), trade blocs (WTO, COMESA, EU, ECOWAS), transport networks, wildlife conservation areas in Kenya and Africa, and urbanisation patterns in Africa must be accurate per the KCSE Geography syllabus.
2. Ensure Form 4 scope — Human Geography topics at Form 4 including population, industry, trade, transport, energy, wildlife, and environmental management.
3. Add `stimulus` objects for questions referencing population pyramid diagrams, demographic data tables, industrial location maps, climate data for fishing grounds, or trade statistics tables.
4. Upgrade `example_answer.format` to `"svg"` for: demographic transition model graphs, population pyramid diagrams, trade flow diagrams, energy source pie charts, transport network maps (schematic), industrial location diagrams. Create SVG files in `form4/geography/illustrations/`.
5. Set `answer_space_type: "grid_box"` for all graph-plotting questions (population pyramids, climate graphs, trade graphs).
6. Include `data_response` questions with actual/realistic data for population analysis, industrial location factor weighting, and trade balance calculations.
7. Include `calculation` questions for: population density, birth rate/death rate/natural increase calculations, population growth projections.
8. Review `answer_lines` and correct.
9. Verify `type`, `difficulty`, `cognitive_level` — Form 4 Geography must have very strong `data_response`, `diagram`, and `explanation` type coverage with difficulty predominantly 3–5.
10. Ensure each topic has at minimum 30 questions.
11. Fix atomization errors.
12. Consult `../materials/form-4/geography/` and KCSE Geography past papers for accuracy and coverage.

**Update after completion:**
- [ ] Mark this task `[x]`
- [ ] git commit: `fix(geography): content improvement pass Form 4`

---

### Task 52: Form 4 History and Government — Content Improvement
**Files to modify:** `form4/history/*.json`
**Files in scope:**
- `citizenship.json`
- `constitution-making-and-constitutional-changes-in-kenya.json`
- `devolved-government.json`
- `electoral-process-and-government-in-britain-and-usa.json`
- `electoral-process-and-government-in-india.json`
- `formation-structure-and-functions-of-the-government-of-kenya.json`
- `government-and-democracy.json`
- `land-tenure-in-kenya.json`
- `national-philosophies-in-kenya.json`
- `public-revenue-and-expenditure-in-kenya.json`
- `social-economic-and-political-developments-and-challenges-in-africa-since-independence.json`
- `social-economic-and-political-developments-and-challenges-in-kenya-since-independence.json`
**Depends on:** Task 01
**Parallel group:** P2

**Specification:**
Read `AGENT.md` in full before starting. For every topic JSON file listed above:
1. Verify historical and constitutional accuracy — Kenya's 2010 Constitution structure (Articles, Schedules), devolved government structure (County governments, Senate, National Assembly), electoral systems in Kenya/Britain/USA/India, national philosophies (Harambee, Nyayoism, African Socialism), land tenure systems, public finance (government revenue sources, Budget process), and post-independence developments in Africa must be accurate per the KCSE History and Government syllabus.
2. Ensure Form 4 scope — Comparative government, Kenyan post-independence history, and contemporary governance issues.
3. For comparative government topics (Britain, USA, India): ensure factual accuracy about each country's specific governmental structures — do not confuse the three systems.
4. Add `stimulus` objects for questions referencing constitutional articles, electoral data, or government budget extracts provided for analysis.
5. Review `answer_lines` and correct.
6. Verify `type`, `difficulty`, `cognitive_level` — Form 4 History should have strong `analysis` level questions comparing governance systems and evaluating Kenya's post-independence development.
7. Ensure each topic has at minimum 30 questions.
8. Fix atomization errors — particularly for comparison questions between different governance systems.
9. Consult `../materials/form-4/history/` and KCSE History and Government past papers for accuracy and coverage.

**Update after completion:**
- [ ] Mark this task `[x]`
- [ ] git commit: `fix(history): content improvement pass Form 4`

---

### Task 53: Form 4 Home Science — Content Improvement
**Files to modify:** `form4/home-science/*.json`
**Files in scope:**
- `adolescent-development-and-challenges.json`
- `advanced-food-preparation-and-service.json`
- `advanced-housing-and-home-environment.json`
- `advanced-laundry-and-fabric-care.json`
- `careers-in-home-science.json`
- `community-nutrition-and-food-security.json`
- `consumer-protection-and-legislation.json`
- `fashion-design-and-textile-industry.json`
**Depends on:** Task 01
**Parallel group:** P2

**Specification:**
Read `AGENT.md` in full before starting. For every topic JSON file listed above:
1. Verify factual accuracy — adolescent development stages (physical, cognitive, emotional, social), community nutrition concepts (malnutrition types, food security pillars, supplementary feeding programmes), consumer protection legislation (Kenya Bureau of Standards, Consumer Protection Act), advanced laundry techniques (dry cleaning symbols, specialist fabric care), fashion design principles, and careers in Home Science must be accurate per the KCSE Home Science syllabus.
2. Ensure Form 4 scope — this is the culminating year; questions should integrate knowledge from all four years and address real-world application.
3. Add `stimulus` objects for questions referencing case studies on nutrition status, consumer complaint scenarios, fashion design briefs, or community nutrition programme data.
4. Upgrade `example_answer.format` to `"svg"` for: fashion design sketches (basic garment illustrations), fabric construction diagrams (advanced weaves, knits), nutrition programme flowcharts, consumer protection process diagrams. Create SVG files in `form4/home-science/illustrations/`.
5. Include application questions that integrate multiple Home Science topic areas (e.g., planning a community nutrition intervention for adolescents).
6. Review `answer_lines` and correct.
7. Verify `type`, `difficulty`, `cognitive_level` — Form 4 Home Science should have difficulty predominantly 3–5.
8. Ensure each topic has at minimum 30 questions.
9. Fix atomization errors.
10. Consult `../materials/form-4/home-science/` and KCSE Home Science past papers for accuracy and coverage.

**Update after completion:**
- [ ] Mark this task `[x]`
- [ ] git commit: `fix(home-science): content improvement pass Form 4`

---

### Task 54: Form 4 IRE — Content Improvement
**Files to modify:** `form4/ire/*.json`
**Files in scope:**
- `akhlaq-moral-teachings-contemporary-issues.json`
- `fiqh-islamic-criminal-law.json`
- `fiqh-muslim-personal-law-in-kenya.json`
- `hadith-collections-and-classification.json`
- `history-abbasid-dynasty.json`
- `history-umayyad-dynasty.json`
- `islam-in-east-africa.json`
- `muamalat-islamic-finance.json`
- `pillars-of-iman-qadr.json`
- `quran-selected-surahs-and-themes.json`
- `quran-surah-al-hujurat.json`
**Depends on:** Task 01
**Parallel group:** P2

**Specification:**
Read `AGENT.md` in full before starting. For every topic JSON file listed above:
1. Verify religious and historical accuracy — Umayyad and Abbasid dynasties (key caliphs, capitals, contributions, downfall), Hadith classification systems (the six major Hadith collections, isnads, matn), Islamic criminal law (Hudud, Qisas, Tazir), Muslim personal law in Kenya (Kadhis' Courts, Muslim Marriage, Divorce and Succession), Islamic finance principles (Riba prohibition, Murabaha, Musharaka, Mudaraba, Takaful), Surah Al-Hujurat themes, and Qadr as a pillar of Iman must be accurate per the KCSE IRE syllabus.
2. Ensure Form 4 scope — Umayyad and Abbasid history, advanced Fiqh, and advanced Islamic economics are Form 4 content.
3. Add `stimulus` objects for questions referencing Quranic verses from Surah Al-Hujurat or other selected Surahs provided for analysis and application.
4. For Muslim personal law in Kenya: ensure questions reflect the actual legal framework under Kenyan law (Kadhis' Courts Act, Marriage Act).
5. Include application questions relating Islamic finance principles to contemporary financial products available in Kenya (Islamic banking products offered by Kenyan banks).
6. Review `answer_lines` and correct.
7. Verify `type`, `difficulty`, `cognitive_level` — Form 4 IRE must have difficulty predominantly 3–5 with strong analysis and application.
8. Ensure each topic has at minimum 30 questions.
9. Fix atomization errors.
10. Consult `../materials/form-4/ire/` and KCSE IRE past papers for accuracy and coverage.

**Update after completion:**
- [ ] Mark this task `[x]`
- [ ] git commit: `fix(ire): content improvement pass Form 4`

---

### Task 55: Form 4 Kiswahili — Content Improvement
**Files to modify:** `form4/kiswahili/*.json`
**Files in scope:**
- `aina-za-sentensi-na-upatanisho.json`
- `insha-mchanganyiko.json`
- `irabu-konsonanti-na-silabi.json`
- `isimu-jamii-lugha-katika-kazi.json`
- `isimu-jamii-tafsiri.json`
- `isimu-jamii-ukuzaji-wa-msamiati.json`
- `matumizi-ya-lugha-mseto.json`
- `uchanganuzi-wa-sentensi-kina.json`
- `ufahamu.json`
- `ufupisho.json`
- `ushairi-uchambuzi-kamili.json`
**Depends on:** Task 01
**Parallel group:** P2

**Specification:**
Read `AGENT.md` in full before starting. All questions must be in correct Kiswahili sanifu at the highest KCSE standard.
1. Verify linguistic accuracy — deep sentence analysis (uchanganuzi wa kina: nomino, kitenzi, kishazi, kauli, tendo na mtendwa), sentence types and agreement (upatanisho wa nomino, kitenzi, vivumishi at advanced level), phonology (irabu, konsonanti, silabi — syllable structure and phonological processes in Kiswahili), sociolinguistics (tafsiri/translation theory, lugha katika kazi/language in the workplace, ukuzaji wa msamiati/vocabulary development, matumizi ya lugha mseto/code-switching), insha mchanganyiko (mixed essay types: masimulizi, maelezo, hoja), and full poem analysis (uchambuzi kamili wa ushairi) must be accurate per the KCSE Kiswahili syllabus.
2. Ensure Form 4 scope — this is the culminating year. Questions must integrate all Kiswahili skills and be at the full KCSE Paper 1, Paper 2, and Paper 3 difficulty level.
3. For ufahamu and ufupisho: embed full passages in `stimulus`. Form 4 passages should be on sophisticated topics. Follow the exact KCSE Kiswahili format.
4. For ushairi uchambuzi kamili: embed complete poems. Analysis questions must cover all aspects: muundo (mizani, vina, mishororo, beti), maudhui, dhamira, mtazamo, tamathali za usemi, na tathmini.
5. For insha mchanganyiko: prompts must combine different writing modes (e.g., a narrative that includes argumentative elements). Rubrics use holistic band descriptors covering lugha, maudhui, mpangilio, and ubunifu.
6. Add `stimulus` objects for all passage-based questions.
7. Review `answer_lines` — Form 4 writing tasks need `answer_space_type: "plain_box"` with `answer_box_height_mm: 200–240`.
8. Ensure each topic has at minimum 30 questions; ufahamu, ufupisho, and ushairi should aim for 40+.
9. Fix atomization errors. Writing task rubrics must NOT be atomized; use holistic descriptors.
10. Consult `../materials/form-4/kiswahili/` and all available KCSE Kiswahili past papers (Paper 1, Paper 2, Paper 3) for accuracy and format.

**Update after completion:**
- [ ] Mark this task `[x]`
- [ ] git commit: `fix(kiswahili): content improvement pass Form 4`

---

### Task 56: Form 4 Mathematics — Content Improvement
**Files to modify:** `form4/mathematics/*.json`
**Files in scope:**
- `area-approximation.json`
- `differentiation.json`
- `integration.json`
- `linear-programming.json`
- `locus.json`
- `longitudes-and-latitudes.json`
- `matrices-and-transformations.json`
- `statistics-ii.json`
- `three-dimensional-geometry.json`
- `trigonometry-iii.json`
- `vectors-ii.json`
**Depends on:** Task 01
**Parallel group:** P2

**Specification:**
Read `AGENT.md` in full before starting. For every topic JSON file listed above:
1. Verify mathematical accuracy — differentiation rules (product rule, chain rule, quotient rule for KCSE scope), integration techniques (by substitution at KCSE level), area approximation (trapezium rule, mid-ordinate rule), linear programming (feasible region, objective function), longitude/latitude great circle and small circle distance calculations, combined transformations (matrices), statistics II (standard deviation, variance, cumulative frequency curves), and vector operations in 3D must be correct.
2. Ensure Form 4 scope — Differentiation and Integration at KCSE Form 4 level covers basic polynomial differentiation/integration, applications to gradients, turning points, and areas under curves. It does NOT cover advanced calculus techniques.
3. Upgrade `example_answer.format` to `"svg"` for: linear programming feasible region graphs, cumulative frequency (ogive) curves, locus diagrams, longitude/latitude sphere diagrams, 3D geometry figures, combined transformation diagrams. Create SVG files in `form4/mathematics/illustrations/`.
4. Set `answer_space_type: "grid_box"` for all graph-plotting questions (linear programming graphs, ogives, area approximation graphs).
5. Set `answer_space_type: "construction_box"` for locus construction questions.
6. Upgrade `example_answer.format` to `"tiptap"` for answers with calculus working, matrix multiplication steps, and statistical calculations.
7. Calculation rubrics must follow formula + substitution + answer with units pattern, with follow-through marking for all multi-step questions.
8. Include proof questions for trigonometric identities (Form 4 level) and differentiation from first principles.
9. Review `answer_lines` and correct.
10. Verify `type`, `difficulty`, `cognitive_level` — Form 4 Mathematics is the most demanding; predominantly difficulty 4–5.
11. Ensure each topic has at minimum 30 questions; Differentiation, Integration, and Statistics II can sustain 50+ questions.
12. Fix atomization errors.
13. Consult `../materials/form-4/mathematics/` and KCSE Mathematics past papers (especially Paper 1 and Paper 2) for accuracy and format.

**Update after completion:**
- [ ] Mark this task `[x]`
- [ ] git commit: `fix(mathematics): content improvement pass Form 4`

---

### Task 57: Form 4 Physics — Content Improvement
**Files to modify:** `form4/physics/*.json`
**Files in scope:**
- `cathode-rays.json`
- `electromagnetic-induction.json`
- `electromagnetic-spectrum.json`
- `electronics.json`
- `floating-and-sinking.json`
- `mains-electricity.json`
- `photoelectric-effect.json`
- `radioactivity.json`
- `thin-lenses.json`
- `uniform-circular-motion.json`
- `x-rays.json`
**Depends on:** Task 01
**Parallel group:** P2

**Specification:**
Read `AGENT.md` in full before starting. For every topic JSON file listed above:
1. Verify factual accuracy — thin lens formula (1/f = 1/u + 1/v) and sign conventions, electromagnetic induction laws (Faraday's and Lenz's), transformer equation (Vs/Vp = Ns/Np = Ip/Is), radioactivity (alpha/beta/gamma properties, half-life, nuclear equations, uses and hazards), photoelectric effect (Einstein's equation: hf = φ + ½mv²), cathode ray tube operation, electronics (p-n junction, rectification circuits, transistor as a switch), mains electricity (domestic wiring, safety devices, energy calculations), floating and sinking (Archimedes' principle, upthrust, relative density), uniform circular motion (centripetal force/acceleration formulae), and the electromagnetic spectrum (properties and uses of each region) must be accurate per the KCSE Physics syllabus.
2. Ensure Form 4 scope — this is the highest Physics year. Include multi-step calculations integrating multiple concepts and design-of-experiment questions.
3. Upgrade `example_answer.format` to `"svg"` for: thin lens ray diagrams (all object position cases for converging and diverging lenses), transformer diagrams, radioactive decay series diagrams, circuit diagrams for rectification (half-wave, full-wave), transistor switch circuits, cathode ray tube diagram, photoelectric effect diagram, domestic wiring circuit (ring main, radial circuit). Create SVG files in `form4/physics/illustrations/`.
4. Set `answer_space_type: "diagram_box"` for all diagram-drawing questions with appropriate `answer_box_height_mm`.
5. Set `answer_space_type: "grid_box"` for all graph-plotting questions (radioactive decay curves, lens formula graphs, I-V characteristics).
6. Upgrade `example_answer.format` to `"tiptap"` for answers with nuclear equations, Einstein photoelectric equation calculations, and multi-step circuit calculations.
7. Calculation rubrics must follow formula + substitution + answer with units pattern. Include Planck's constant (h = 6.626 × 10⁻³⁴ J·s) and electron charge (e = 1.6 × 10⁻¹⁹ C) in relevant calculation rubrics.
8. Include `experiment` questions for: determining the focal length of a converging lens, verifying Archimedes' principle, and plotting the I-V characteristic of a diode.
9. Review `answer_lines` and correct.
10. Verify `type`, `difficulty`, `cognitive_level` — Form 4 Physics is the most demanding; difficulty predominantly 4–5. Include synthesis-level structured questions that combine multiple Form 4 topics.
11. Ensure each topic has at minimum 30 questions; Radioactivity, Electromagnetic Induction, and Thin Lenses can sustain 50+ questions.
12. Fix atomization errors.
13. Consult `../materials/form-4/physics/` and KCSE Physics past papers for accuracy, style, and the highest difficulty standard.

**Update after completion:**
- [ ] Mark this task `[x]`
- [ ] git commit: `fix(physics): content improvement pass Form 4`
