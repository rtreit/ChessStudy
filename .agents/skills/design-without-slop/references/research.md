# Research foundations

Reviewed 9 November 2026, America/Los_Angeles. Recheck evolving standards and product guidance when needed. The house style is a synthesis for the owner's projects, not a universal standard.

## Design evidence

| Primary source | Supported principle | Limit |
| --- | --- | --- |
| [USWDS cards](https://designsystem.digital.gov/components/card/) | Cards serve independent content groups; avoid decorative cards and cards substituted for table rows | Does not prohibit cards everywhere |
| [USWDS tables](https://designsystem.digital.gov/components/table/) | Minimal styling, consistent units, aligned amounts, source/date context, and task-sensitive mobile layout | Not every dataset belongs in a table |
| [WAI tables](https://www.w3.org/WAI/tutorials/tables/) | Use semantics and header associations to expose relationships | Appearance does not establish accessibility |
| [WAI sortable table example](https://www.w3.org/WAI/ARIA/apg/patterns/table/examples/sortable-table/) | Native tables, header buttons, and an appropriate sort state | Examples need implementation testing |
| [WCAG 2.2](https://www.w3.org/TR/WCAG22/) | Normative web-accessibility target | A limited audit cannot establish complete conformance |
| [W3C reflow explanation](https://www.w3.org/WAI/WCAG22/Understanding/reflow.html) | Genuine two-dimensional tables can need local scrolling | Does not excuse whole-page overflow or unreadable cell contents |
| [W3C target-size explanation](https://www.w3.org/WAI/WCAG22/Understanding/target-size-minimum.html) | AA uses 24 × 24 CSS px with specified exceptions | 44 px touch controls are a stronger house preference |
| [Government Analysis Function charts](https://analysisfunction.civilservice.gov.uk/policy-store/data-visualisation-charts/) | Use clear presentation, sources, and suitable text alternatives | A chart needs an analytical purpose |
| [GOV.UK design principles](https://www.gov.uk/guidance/government-design-principles) | Begin with user needs, evidence, and context | References do not establish identical tasks across products |

The light palette, font defaults, title sizes, radii, and first-result budget are house choices. Accuracy, usability, and accessibility remain independent requirements.

## Guidance for generated interfaces

[OpenAI's frontend prompt guidance](https://developers.openai.com/api/docs/guides/frontend-prompt) distinguishes operational tools from expressive experiences. Its operational guidance supports organized information, predictable navigation, and restrained composition. Do not import a vendor prompt wholesale: unfamiliar icons still need accessible names, and essential explanations must remain available.

[Anthropic's frontend-skills article](https://claude.com/resources/articles/improving-frontend-design-through-skills), published 12 November 2025, discusses recurring generated aesthetics and targeted prompting. Its expressive font, background, and motion examples demonstrate steerability. They do not establish the right default for a report or expert tool.

Specify a positive composition, real content, constraints, and review process. A negative list leaves room for a different generic style. A palette alone leaves the hierarchy unchanged.

## Language evidence

[ASD-STE100](https://www.asd-ste100.org/) identifies Issue 9, dated 15 January 2025, as current at review time. Its [official overview](https://www.asd-ste100.org/about_STE.html) describes controlled vocabulary and writing rules with provision for technical terminology.

The [official Issue 9 document](https://www.asd-ste100.org/assets/files/ASD-STE100_ISSUE9.pdf) distinguishes procedural and descriptive sentence limits, uses direct instructions, and disallows contractions. Official indexed text was available; direct PDF retrieval was blocked during research. This skill uses a limited original paraphrase and does not reproduce the standard or dictionary.

[Microsoft's contraction guidance](https://learn.microsoft.com/en-us/style-guide/word-choice/use-contractions) encourages familiar conversational forms. [Google's tone guidance](https://developers.google.com/style/tone) supports direct conversational technical writing without excessive informality or filler. [Google's requirements guidance](https://developers.google.com/style/prescriptive-documentation) separates requirements, recommendations, and possible outcomes.

The skill therefore uses plain technical English informed by STE, with ordinary UI contractions allowed. It does not claim formal conformance. Keep a project glossary and preserve precise meaning. When formal STE is required, use the full current standard and appropriate review.

The [official checking-tool guidance](https://asd-ste100.org/STEsoftware.html) explains automation's limits and the need for informed review. The [official downloads page](https://asd-ste100.org/STE_downloads.html) warns that plausible AI text does not establish compliance. A phrase checker cannot certify truth, technical meaning, or complete compliance.

Do not bundle the standard or dictionary. Link to official access. Do not claim ASD endorsement or certification.

## Owner-supplied examples

The Workbench instructions supplied a restrained palette, typography, accessibility expectations, and a requirement for browser evidence. The skill retains general principles while excluding Spindle-specific schemas, routes, status dimensions, preview limits, and submission workflows.

The four benchmark sites establish visual direction. The Faraday HTML pair establishes a preferred report direction. Source inspection showed a transition from cards and filters to continuous preprint structure. See `examples.md` for observations and language edits. The original attachments were not modified.
