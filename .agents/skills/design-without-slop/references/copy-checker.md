# Review an explicit copy catalog

Use `scripts/check_copy.py` to flag a small set of phrases and long sentences in authored UI text. It requires Python 3 and no additional packages. It reads a JSON catalog; it does not search application source, rewrite copy, or change files.

From this skill's directory, run:

```sh
python3 scripts/check_copy.py /path/to/copy.json
python3 scripts/check_copy.py --help
python3 scripts/test_check_copy.py
```

Use `-` instead of a path to read the catalog from standard input.

## Input schema

Supply a **nonempty JSON array**. Every record requires all four fields below. Unknown fields, duplicate IDs, duplicate JSON keys, blank text, and invalid records fail validation. Every record is validated before any copy is reviewed, including records marked for exclusion.

| Field | Value |
|---|---|
| `id` | Unique, nonblank string without surrounding whitespace. Use a stable UI location or message ID. |
| `text` | Nonblank string containing the exact copy to review. |
| `kind` | `label`, `instruction`, `description`, `error`, or `status`. |
| `provenance` | `authored`, `imported`, `code`, or `quote`. |
| `exceptions` | Optional array of scoped exceptions described below. |

```json
[
  {
    "id": "run.start",
    "text": "Start run",
    "kind": "label",
    "provenance": "authored"
  },
  {
    "id": "run.missing-benchmark",
    "text": "Select a benchmark.",
    "kind": "error",
    "provenance": "authored"
  },
  {
    "id": "result.external-title",
    "text": "Title copied from the source dataset",
    "kind": "label",
    "provenance": "imported"
  }
]
```

Use `authored` for UI copy written or adapted for the product. Use `imported` for unchanged external text, `code` for a code excerpt, and `quote` for a verbatim quotation. Excluded records remain visible in the report. These categories describe provenance, not importance or permission to ignore misleading content. The script cannot verify whether a provenance declaration is true.

## Checks and counting

The checker reports case-insensitive matches for the exact phrases below, after collapsing whitespace. It checks word boundaries but does not infer synonyms, tone, or intent. It does not prohibit individual technical words such as *exploit*, *payload*, *attack*, or *robustness*.

- unlock powerful insights
- unlock your potential
- elevate your security posture
- harness the power of
- leverage the power of
- at your fingertips
- game-changing
- cutting-edge
- next-generation
- seamlessly integrate
- it is important to note
- please be advised
- in today's rapidly evolving landscape

For authored instructions, it reports sentences over **20 approximate words**. For authored descriptions, errors, and statuses, it reports sentences over **25 approximate words**. Labels receive the phrase check but no sentence-length check. These are local review rules, not a formal implementation of ASD-STE100.

Counting is deliberately approximate. A line break, or whitespace after `.`, `!`, or `?`, starts a new sentence. Each whitespace-separated token containing a letter or number counts as one word. A decimal such as `3.14` stays together. Hyphenated words count as one token; a number and a separated unit count as two. Abbreviations, punctuation beside quotation marks, URLs, and unusual formatting can produce inaccurate sentence boundaries or counts. Read the reported sentence before changing it.

## Scoped exceptions

An exception applies only to the record containing it. It must name one exact listed phrase or one detected sentence number, and include a nonblank reason. An exception never bypasses schema validation. Excluded records cannot carry exceptions.

```json
[
  {
    "id": "required.tagline",
    "text": "A game-changing feature.",
    "kind": "description",
    "provenance": "authored",
    "exceptions": [
      {
        "rule": "phrase",
        "phrase": "game-changing",
        "reason": "This exact product tagline is required by the supplied brief."
      }
    ]
  }
]
```

A sentence exception has this shape: `{"rule":"sentence-length","sentence":1,"reason":"Explain the specific need to retain this wording."}`. Sentence numbers start at 1 within the record, using the heuristic above. Check their scope again after editing the text. A phrase exception covers all occurrences of that phrase within its record.

The report prints each applied exception, its finding, and its reason. An exception with no matching finding is printed as `UNUSED EXCEPTION` and produces exit code 1; review or remove it. Do not add broad or placeholder reasons to make a report pass.

## Report and exit codes

The report shows catalog records, authored records reviewed, exclusions by provenance, individual excluded IDs, findings, open findings, applied exceptions, exception entries, and unused exceptions. A catalog containing only exclusions explicitly says that no authored copy was reviewed.

| Exit | Meaning |
|---|---|
| `0` | No unexcepted findings in this catalog. Applied exceptions and exclusions may still exist. |
| `1` | At least one unexcepted phrase/length finding or unused exception requires review. |
| `2` | Invalid JSON, schema, file input, or command usage. No copy review is performed. |

## Limits

This is **not a semantic, STE-compliance, or accessibility validator**. It does not confirm factual accuracy, technical meaning, preserved uncertainty, valid recovery actions, truthful button effects, appropriate provenance, or complete coverage of the UI. A clean report applies only to the supplied catalog and these small checks. Manually compare the catalog with the rendered UI and review the meaning of the copy. Do not describe its result as ASD approval, STE compliance, accessibility compliance, or proof that a page contains no poor prose.
