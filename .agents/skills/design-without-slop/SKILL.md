---
name: design-without-slop
description: Create, revise, and review websites, dashboards, benchmark pages, reports, specifications, and technical write-ups with deliberate visual design and plain technical English. Use for anti-AI-slop requests, clean research or preprint styling, usable data tools, UX copy, STE-informed wording, or removal of generic generated layouts and inflated prose. Apply to HTML, app UI, Markdown, documents, and print outputs while preserving technical meaning.
---

# Design Without Slop

Make the content easy to read, compare, and use. Make the composition look deliberate. Write like a capable colleague.

Treat this file as a complete, portable policy. Use its optional references and checker when available. Retain the core requirements when copying this file alone.

## 1. Establish the design contract

Identify the audience, main task, available content, and output format. Inspect the existing artifact and project instructions. Treat supplied examples as evidence of taste and quality. Inspect rendered references when possible; extracted text does not establish visual quality.

Choose a profile, or combine profiles on separate views:

| Profile | Main task | Default composition |
| --- | --- | --- |
| Tool, dashboard, benchmark | Find, compare, inspect, act | Compact identity and title; scope; relevant controls; primary results or work surface; detail |
| Report, technical write-up | Understand an argument and check its support | Specific title; summary or abstract; ordered sections; figures and tables beside the discussion; references |
| Specification | Decide what to build and how to verify it | Purpose; scope; requirements; behavior; interfaces and states; acceptance criteria; open questions |
| Public project site or collection | Understand the project and choose useful material | Clear identity; short introduction; purposeful navigation; substantive previews; direct access to results or documents |

State the profile and 3–5 concrete design commitments in the work plan. For a small edit, one sentence is enough. Proceed without a new approval step.

Honor explicit user choices and documented project requirements. Apply these defaults where the brief leaves a choice open. Record exceptions only for a concrete user requirement, product constraint, or demonstrated task or accessibility need. Name the rule and reason. Do not excuse a generic layout by calling it intentional. Do not ask for permission just because an allowed exception is needed.

Keep **house style**, **task-based judgment**, and **accessibility standards** distinct. The palette, type sizes, and layout budgets here are house choices, not universal usability laws.

## 2. Design the composition before choosing components

Start with actual content and a reading order. Design with long labels, sparse data, and failure states. Identify demonstration data clearly and keep it separate from real results.

Give each section one purpose. Make headings describe its subject or conclusion. Remove duplicate titles, eyebrow labels, and introductions that repeat the same point. Keep a qualification beside the claim it changes.

Use a shared alignment grid, deliberate column widths, consistent spacing, and a small type scale. Group material through proximity and whitespace. Add rules or surfaces when they clarify boundaries. Create emphasis through placement, size, weight, and contrast before decoration.

Allocate space according to importance and content. Do not pad a short result to match a neighboring panel. Let sparse content remain sparse. Do not fill gaps with invented metrics, generic explanations, illustrations, or placeholder charts.

Preserve readable breathing room. Density means more useful information per view, not smaller text or every detail at once.

### Reject these defaults

- A slogan or oversized marketing hero above a working tool, report, or specification.
- A decorative KPI-card strip added because the task mentions a dashboard.
- A rounded panel around every section, nested panels, or identical cards for continuous prose.
- Ornamental gradients, glow, glass, blobs, floating tiles, illustrations, or purposeless animation.
- A sidebar, tab, filter, badge, or chart with no corresponding content or implemented purpose.
- An accent-colored feature card promoting an arbitrary number, especially an unavailable value.
- Tiny muted labels, excessive uppercase, widely spaced microheadings, or monospace used for all prose.
- A novelty font, dark theme, or dramatic composition chosen merely to look less AI-generated.
- “Premium,” “world-class,” “clean,” or “beautiful” substituted for observable design decisions.

Do not impose a universal ban on cards, charts, color, sidebars, serif type, or familiar component libraries. Independent document previews can use cards. Comparisons usually need aligned rows. A chart can reveal a pattern. Justify the component through its task.

Apply the **removal test**: if removing a box, icon, badge, sentence, or effect preserves meaning, navigation, state, and useful grouping, remove it. Keep visible control affordances and essential explanations.

## 3. Use restrained visual foundations

Use a light neutral canvas by default. Use one principal accent for links, selection, and commands. Reserve semantic colors for actual states. Use distinguishable, labeled series colors when the data needs them.

Reuse project tokens. Without an established palette, start here with neutral names such as `--ui-text`:

| Token | Value |
| --- | --- |
| canvas / surface | `#ffffff` / `#f5f7f5` |
| text / muted | `#17211b` / `#5d6961` |
| border / border-strong | `#d9e0db` / `#b9c4bc` |
| accent / accent-hover | `#185c3f` / `#10472f` |
| accent-subtle | `#e7f1eb` |
| warning-text / warning-surface | `#7a4c08` / `#fff4dc` |
| danger-text / danger-surface | `#8b3030` / `#fae9e9` |
| neutral-text / neutral-surface | `#536058` / `#edf1ee` |

Use the light border colors for separators. They cannot identify a control by themselves. Use a sufficiently contrasting boundary, fill, or identifying cue. Check actual color pairs and states in the rendered artifact.

- Use system sans for controls and application text. Use a readable serif or sans face for long reports. Choose type for the reading task; do not reject system fonts on principle.
- Use monospace for code, identifiers, and raw records. Use tabular numerals for comparable amounts. Keep identifiers selectable and copyable.
- Start with 16 px application body text and 16–18 px report body text. Keep ordinary dense table text at least 14 px. Use relative units and honor user font settings. These are house defaults, not WCAG minimum font sizes.
- Keep working-page titles around 24–32 px and report titles around 28–36 px. Use larger display type only when the public page's purpose warrants it.
- Use roughly 1.4–1.6 body line height. Keep prose columns about 60–80 characters wide. Give wide figures and tables a separate measure.
- Use a small spacing scale, such as 4, 8, 12, 16, 24, 32, and 48 px. Keep a consistent vertical rhythm. Avoid large empty bands between related content.
- Use square regions and small control radii, normally 0–4 px. Reserve pills for a real selection or status pattern. Use shadows to explain layering. A single paper sheet is acceptable for a document preview.
- Keep icons consistent. Label unfamiliar actions. Avoid decorative icons beside every heading and invented logos for an existing project.

Keep essential text legible, links recognizable, and controls obvious. Restraint still requires visible affordances.

## 4. Build tools around useful information

Give the primary results or work surface the space it needs. Keep the header compact, about 48 px on desktop when a top bar is needed. Use available width up to roughly 1600 px for dense work. Do not confine a wide table to a prose column.

For a focused results page at 1440 × 900 CSS px, put the first useful row or primary visualization within the upper 40% of the viewport. Show at least five rows when five exist and the task uses a table. At 360 px, expose the title, scope, essential control, and first useful result or truthful state without a decorative preamble. These are house review gates. Do not shrink text, hide critical context, or invent rows to meet them.

Use tables for repeated records with comparable fields and aligned lists for simpler records. Place controls beside the content they change. Show active filters, sort, and result count. Add pagination or virtualization only when volume warrants it; preserve accessible navigation and record identity.

### Tables and detail

- Prefer native table semantics, useful headers, and a table name. Associate complex headers explicitly. Use an ARIA grid only when the interaction requires it.
- Right-align comparable amounts. Keep units and precision consistent. Left-align names and identifiers. Do not imply unsupported precision.
- Implement sorting with labeled controls and the correct sort state. Define missing-value and tie handling. Sort values, not formatted numeric strings.
- Use subtle rules, adequate cell padding, and row heights that fit the controls. Make long identifiers readable through wrapping, expansion, or copying.
- Preserve selection, applicable filters, and scroll position on refresh. Keep text selection separate from commands.
- Choose inline detail, an inspector, or a linked page according to the task. Preserve and restore focus. Do not transplant one project's inspector placement into every tool.

On narrow screens, reflow controls and prose. Use labeled records for simple lookup lists. For comparisons requiring columns, use a named, keyboard-accessible table scroll region. Keep the rest of the page within the viewport. Do not automatically turn every comparison table into cards. Keep cell contents readable without their own two-dimensional scrolling.

Require every chart to answer a stated question: comparison, trend, distribution, relationship, or tradeoff. Choose the chart for that question. Label units, scales, groups, and uncertainty. Quantitative bars normally start at zero. Explain a necessary exception. Provide values or accessible equivalents. Do not make essential facts hover-only. Keep absent-data states compact. Use reproducible plotting or vector tools for exact data and technical figures. Do not use generated artwork to invent measurements or diagram relationships.

### Data and state integrity

Distinguish zero, pending, not measured, not applicable, withheld, unavailable, and failure. Define compact symbols. Do not use one unexplained dash for all states.

For benchmarks, expose the metric and direction, denominator or sample size, release, evaluated system/configuration, and applicable dates. Distinguish dataset date, run date, and page refresh date. Link methodology and result evidence where available. Separate measured from estimated values and submitted from independently checked results. Explain incomplete coverage and exclusions.

Show uncertainty only when supplied or calculated by a stated method. Do not fabricate intervals or infer a meaningful rank difference from a small gap. Keep incompatible protocols or releases separate. Do not inherit another product's severity or confidence label.

Implement relevant loading, empty, filtered-empty, partial, error, stale, and offline states. Retain previous data with its observation time when useful. Preserve drafts and pending actions. Explain disabled commands that need current state. Do not label data “live” without a defined freshness mechanism.

## 5. Typeset reports, specifications, and technical prose

Use a coherent reading sequence, a specific title, modest metadata, descriptive headings, and continuous prose. Add an abstract, contents, numbered sections, or appendices when length and genre warrant them. Keep short notes short.

Organize reports around the question, evidence, analysis, findings, limitations, and implications. Organize specifications around intended behavior, scope, interfaces, constraints, failure cases, acceptance criteria, and unresolved decisions. Adapt the sequence to the reader; do not force every document into an academic template.

- Keep substantive prose in a readable column. Use lists for parallel items or steps, tables for comparisons, and diagrams for relationships that prose cannot explain as clearly.
- Avoid card mosaics, promotional headings, decorative kickers, repeated pull quotes, and dashboard statistics in a continuous argument.
- Keep heading levels meaningful and emphasis restrained. Do not bold every opening phrase or whole paragraphs.
- Give figures and tables specific captions, units, legends, and sources. Explain their contribution nearby. Remove a visual that merely counts sources or decorates the document's structure.
- Put citations beside the claims they support. Use one reference identity for repeated sources. Distinguish hypotheses, observations, interpretations, and recommendations.
- Do not invent authors, affiliations, peer-review status, a DOI, or a publication venue to make a preprint look official.
- Use left-aligned web prose. Avoid forced justification and manual line breaks that balance a title at only one width.
- When print or PDF is in scope, inspect pagination, figure readability, table width, captions, links, and heading breaks. Remove interactive-only controls in print. Keep references and essential detail available without clicking.
- Allow long sections and figures to break when needed. Do not apply `break-inside: avoid` to whole multi-page sections. Inspect exported pages; CSS alone does not prove print quality.

Preserve claims during restyling. Keep numbers, citations, qualifications, and technical relationships intact. Verify any needed factual correction and identify it separately from the design change.

## 6. Write plain technical English

Use familiar, direct language. Interpret “colloquial” as natural English a colleague would say, without slang, idioms, fake friendliness, or obstructive jokes.

Use **STE-informed language**, not a claim of ASD-STE100 conformance. Combine controlled technical meaning with conversational UI copy. Formal STE adds vocabulary, grammar, and counting rules and disallows contractions. If formal conformance is explicitly requested, use the current authorized standard and appropriate review. Do not certify it through this skill or checker.

Apply the following to headings, paragraphs, captions, labels, tooltips, accessible names, errors, and status text:

1. State the point early. Prefer concrete subjects and verbs. Name the actor when known and relevant. Use passive voice when the actor is unknown or irrelevant.
2. Use one stable term for each concept. Keep different concepts distinct. Define necessary specialist terms once. Do not swap “run,” “campaign,” and “attempt” for variety.
3. Use one main idea per sentence and one topic per paragraph. Review instructions over 20 words and explanatory sentences over 25 words. Split them without losing conditions or causality. Treat these as project limits; automated counts are approximate.
4. Begin an instruction with its action or a condition the reader must know first. Use one action per step unless actions must occur together.
5. Keep natural articles and connecting words. Do not turn prose into clipped commands or remove qualifications to meet a word limit.
6. Preserve negation, scope, modality, units, denominators, uncertainty, and causal relationships. “May,” “did,” “must,” and “should” are different claims.
7. Permit familiar contractions in ordinary UI and informal explanations. Use full forms for consequential negation or requirements when clearer. Keep repeated messages consistent and follow the document's register.
8. Name the actual action: “Compare runs,” “Download CSV,” “Cancel run.” Give repeated links enough accessible context to identify their target.
9. Explain an error's observed problem and a useful next step. State a cause only when known. Preserve entered data. Do not promise retry will fix an unknown failure.
10. Remove promotional filler, throat-clearing, forced conclusions, nominalizations, and empty certainty. Preserve necessary technical terms.

Reject newly authored filler such as “unlock powerful insights,” “seamlessly,” “game-changing,” “harness the power of,” and “at your fingertips.” Also review inflated technical prose: “protected gain,” “evidence substrate,” “decision surface,” “source-backed intelligence,” and “actionable insights.” Name the actual access, evidence, decision, source, or action instead. Preserve an established, necessary domain term and define it.

Prefer “test account,” “test data,” and “test setup” in ordinary reader-facing copy when accurate. Avoid habitual “owned,” “synthetic,” “fixture,” and “receipt” wording where a clearer concrete phrase preserves the meaning. Retain a term when its precise technical distinction matters.

Do not apply blind replacements. Product names, quotations, code identifiers, imported labels, and genuine technical terms can contain flagged words. Keep exceptions scoped and visible.

| Avoid | Prefer when supported by the facts |
| --- | --- |
| “See what each run proved.” | “Benchmark results” |
| “A result has context.” | “Scores exclude runs that have not been graded.” |
| “Initiate assessment execution” | “Start run” |
| “No resolved positive runs yet” | “No completed runs have a verified finding.” |
| “No findings. Your system is secure.” | “This run found no verified vulnerabilities.” |
| “Potential protected gain” | “Data or actions the account should not be able to access” |
| “Cost of a discriminating test” | “Effort needed to distinguish a failure from expected behavior” |

Verify the facts before using these examples. Do not treat them as terminology mappings: “finding,” “effect,” “completed,” and “graded” may name different states.

## 7. Verify the delivered result

For substantial changes, inspect the rendered result and revise it. A build, lint pass, or unexamined screenshot is insufficient. Use existing project tools; do not add a framework just for review. Match checks to the output format. Do not require browser interactions for a text-only specification or a screen-reader audit of a document outline.

Inspect desktop and narrow layouts, including 1440 × 900 and 360 × 800 CSS px or the actual target sizes. Add an intermediate width where layout changes. Test native browser zoom or text resizing to 200%, and reflow at 320 CSS px or the equivalent viewport. CSS zoom does not establish native-zoom behavior. Honor reduced motion and user font settings.

Target WCAG 2.2 AA for web output. Check semantics, keyboard access, names, status announcements, focus, and contrast. Ordinary text generally needs 4.5:1 contrast; apply large-text and other exceptions accurately. Required non-text boundaries and states generally need 3:1. Keep focus visible and unobscured. A 2 px outline with offset is a house default, not a complete conformance test.

Meet the 24 × 24 CSS px pointer-target rule or its valid exceptions. Prefer 44 × 44 px controls in narrow touch layouts. Do not claim 44 px is the universal AA rule. Check screen-reader behavior for substantial interactive changes when tools are available. Identify untested checks specifically.

For tools, exercise the lookup, comparison, filters, sort, detail, and commands that exist. Check sparse and failure states, long names, large values, exports, legends, and result counts. For documents, check reading order, line length, citations, figures, and required print/export output. Compare original and revised claims after editing copy.

Review authored visible strings and accessible names. If available, use `scripts/check_copy.py` with an explicit copy catalog; read `references/copy-checker.md` for its schema. Inspect exclusions and exceptions. A clean check does not prove coverage, meaning, accessibility, or STE conformance.

Assess these gates independently. Mark each **pass**, **fail**, **not tested**, or **not applicable**, with concise evidence. Never average away failure with a taste score.

| Gate | Required evidence |
| --- | --- |
| Composition | The primary task or argument dominates; hierarchy matches the profile |
| Craft | Consistent alignment, type, spacing, contrast, and purposeful boundaries |
| Meaning | Accurate numbers, states, claims, sources, and technical distinctions |
| Language | Direct wording, stable terms, specific actions, resolved copy findings |
| Behavior | Relevant controls and states work and preserve context |
| Access | Keyboard, focus, narrow layout, zoom, and applicable accessibility checks |
| Output | The actual delivered page, document, or export was inspected |

For substantial redesigns, use a fresh reviewer when available. Supply the artifact, task, and policy; withhold the author's self-assessment. Request concrete defects and their locations. Keep justified choices that serve the task. Do not manufacture faults or weaken supported conclusions.

Fix observed failures and check the affected flow again. Stop when applicable gates pass or a concrete external limitation remains. State precisely what is untested. Do not infer full accessibility compliance, statistical validity, or publication quality from limited checks.

## 8. Deliver and maintain

Deliver the artifact and briefly explain the main changes, validation, and remaining limits. Keep review logs and design rationale outside the product's normal reading flow. Do not label the artifact “anti-slop” or add a panel describing its clean design.

For ongoing projects, point the project's agent instructions at this file and include applicable gates in review. Keep a few representative approved screenshots or pages. Recheck the affected profile when features change the composition. Do not rely only on automatic skill selection for a required standard.

Use optional resources when needed:

- `references/examples.md`: benchmark and report examples, diagnosis, and precise copy revisions.
- `references/research.md`: primary sources, evidence limits, and STE constraints.
- `references/copy-checker.md` and `scripts/check_copy.py`: bounded mechanical copy review, followed by the meaning gate.

Primary foundations: [USWDS cards](https://designsystem.digital.gov/components/card/), [USWDS tables](https://designsystem.digital.gov/components/table/), [WCAG 2.2](https://www.w3.org/TR/WCAG22/), [ASD-STE100](https://www.asd-ste100.org/), and [Google's technical-writing tone](https://developers.google.com/style/tone). Treat the visual defaults and layout budgets as this skill's house choices.
