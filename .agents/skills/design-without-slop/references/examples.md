# Examples and review cues

Use these examples to interpret the core policy. Verify actual data and behavior before using a suggested rewrite.

## Reference sites

The owner's references were [SWE-bench](https://www.swebench.com/), [BenchLM](https://benchlm.ai/), [LiveBench](https://livebench.ai/#/), and [Scale Labs](https://labs.scale.com/leaderboard). They establish an overall standard of visual quality, including the benchmark views. They are not templates to clone.

All four home views were inspected in a browser on 9 November 2026. Their public content was also read. LiveBench required the browser because text extraction returned a JavaScript shell. Interactions and accessibility were not comprehensively tested.

| Reference | Useful direction | Do not copy automatically |
| --- | --- | --- |
| SWE-bench | Concentrated comparison surface; clear identity; adjacent controls and definitions | Mascot, colored masthead, sidebar, exact columns |
| BenchLM | Consistent alignment and typography; substantive model summaries; detailed comparison | Broad navigation, leader section, composite score |
| LiveBench | Release context; local controls; category scores and detail | Large introduction, release timeline, contamination claims |
| Scale Labs | Confident typography; restrained framing; useful benchmark previews | Institutional hero or preview grid on an operational results page |

Judge the composition, proportions, content, and task together. A familiar font, card, or large heading alone does not establish poor design.

## The Proving Ground screenshot

The supplied screenshot put a large slogan and decorative numbered tiles above the work. Four summary cards contained two unavailable values. A large empty chart and an explanatory panel preceded a small results table. Repeated section labels added little meaning.

A concrete revision would use the project identity and a direct title, compact scope and freshness information, filters beside results, and a prominent table. Put scoring explanations near their metrics or in a concise method section. Expose run detail through a clear action or disclosure.

Preserve actual states. The screenshot contains a pending result and an efficacy grade unavailable after a provider failure. It does not justify labeling either as a measured zero or completed negative finding. Read the schema before rewriting statuses: graded, resolved, completed, effect, and finding can be distinct concepts.

Keep audience or confidentiality information when it affects a decision. Avoid decorative repetition. Never copy an access token from a reference screenshot into a new artifact.

## Report versus dashboard composition

The owner supplied an original Faraday research report and a cleaner preprint. This comparison comes from HTML and CSS inspection, not a rendered-page audit. Treat the revision as a positive direction, not a validated implementation.

| Original | Cleaner preprint | General rule |
| --- | --- | --- |
| Metaphorical title; display title styled up to 74 px | Descriptive title styled at 31 px | Name the subject without overwhelming it |
| Kickers, pull quote, sidebar, colored panels | Abstract, contents, numbered sections | Give a long document one recognizable reading sequence |
| Twenty-five two-column cards with ranks, pills, and internal grids | Twenty-five numbered subsections with plain paragraphs | Preserve structure without repeated visual packaging |
| Mnemonic reference codes and collapsible source groups | Numbered citations and expanded bibliography | Make evidence easy to follow on screen and in print |
| Display serif headings and system body copy | Modest serif hierarchy and continuous prose | Select typography for sustained reading |
| Filterable worklist | Contents navigation | Let a document remain a document |

The original already had a restrained palette and serif headings. The major improvement came from reading order, proportions, and reduced packaging. Recoloring would not have solved it.

Retain the revision's methods, related findings, proposed tests, limitations, and references. Improve heavy wording, internal-process language, and long sentences. Its justified abstract and wide prose measure still need attention. A subtle page shadow is not the core issue.

### Precise language revisions

Preserve the proposed or conditional nature of the research. Keep adjacent citations attached.

| Existing wording | Clearer wording |
| --- | --- |
| “The contribution is a mechanism-based synthesis and a prioritized set of falsifiable tests.” | “The report groups related failure mechanisms and ranks tests that could support or reject each hypothesis.” |
| “Its source ledger contains 115 entries representing 112 distinct URLs.” | “The source list contains 115 entries, including 112 distinct URLs.” |
| “potential protected impact” | “possible unauthorized access or actions” |
| “cost of a discriminating test” | “effort needed to distinguish a failure from expected behavior” |
| “The primary test setting comprises two unrelated researcher-owned accounts” | “The proposed tests use two separate accounts controlled by the researcher” |
| “observable protected gain” | “evidence of access or actions beyond the account's permissions” |
| “foreign owned identifier” | “an identifier from the other test account” |
| “The relevant Faraday transfer question is whether…” | “For Faraday, the question is whether…” |
| “The ranking was formed independently of the campaign graph.” | “The campaign graph was not used to rank the hypotheses.” Omit if the reader does not need this process detail. |

Do not change “this report does not present experiments” into “no experiments were run” without evidence. Absence from a report does not prove absence of the work.

Use labels such as “Hypothesis,” “Proposed test,” and “Evidence that would support it” when accurate. Avoid statements that make a proposed outcome look like an observed finding. State overall evidence status once, then retain local qualifications that affect interpretation.

Keep precise terms such as token audience, canonicalization, and principal when necessary. Define them for the audience. Remove vague abstraction without erasing the technical model.

## Specification composition

Use purpose, requirements, states, interfaces, acceptance criteria, and open questions as needed. Do not add an academic abstract, marketing hero, or feature-count dashboard to an ordinary feature spec.

Write requirements as observable behavior. Separate requirements from recommendations. Record unresolved decisions. Do not invent behavior to complete the document.

For asynchronous cancellation, “Cancellation requested” and “Cancelled” are different states. A “Cancel run” control must not promise an immediate stop. A retry that creates a new attempt must preserve the old attempt's identity and results when that is the product contract.

## Choices that should pass

- Restrained cards for independent document previews.
- A quality-versus-cost plot that reveals a useful tradeoff.
- Local horizontal scrolling for a comparison table on mobile.
- Serif body type and a single paper-like surface for a report.
- A visible partial failure that makes a summary less tidy but more accurate.
- A small, complete result with no decorative filler.
- A necessary technical word that an indiscriminate blacklist would reject.

Apply the core gates. Do not replace judgment with blanket component bans.
