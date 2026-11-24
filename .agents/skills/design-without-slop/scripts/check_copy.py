#!/usr/bin/env python3
"""Review an explicit UI copy catalog with small, auditable heuristics."""

import argparse
import json
import re
import sys
from pathlib import Path


KINDS = {"label", "instruction", "description", "error", "status"}
PROVENANCE = {"authored", "imported", "code", "quote"}
LIMITS = {"instruction": 20, "description": 25, "error": 25, "status": 25}
PHRASES = (
    "unlock powerful insights",
    "unlock your potential",
    "elevate your security posture",
    "harness the power of",
    "leverage the power of",
    "at your fingertips",
    "game-changing",
    "cutting-edge",
    "next-generation",
    "seamlessly integrate",
    "it is important to note",
    "please be advised",
    "in today's rapidly evolving landscape",
)
PATTERNS = {
    phrase: re.compile(r"(?<!\w)" + re.escape(phrase) + r"(?!\w)")
    for phrase in PHRASES
}


def nonblank(value):
    return isinstance(value, str) and bool(value.strip())


def scope(exception):
    rule = exception["rule"]
    return (rule, exception["phrase"] if rule == "phrase" else exception["sentence"])


def validate(catalog):
    """Validate every record before reviewing any text."""
    if not isinstance(catalog, list) or not catalog:
        return ["Catalog must be a nonempty JSON array of records."]
    errors, ids = [], set()
    required = {"id", "text", "kind", "provenance"}
    for index, item in enumerate(catalog):
        where = f"record[{index}]"
        if not isinstance(item, dict):
            errors.append(f"{where}: must be an object.")
            continue
        missing = required - item.keys()
        extra = item.keys() - required - {"exceptions"}
        if missing:
            errors.append(f"{where}: missing fields: {', '.join(sorted(missing))}.")
        if extra:
            errors.append(f"{where}: unknown fields: {', '.join(sorted(extra))}.")
        for field in ("id", "text"):
            if field in item and not nonblank(item[field]):
                errors.append(f"{where}.{field}: must be a nonblank string.")
        if nonblank(item.get("id")):
            if item["id"] != item["id"].strip():
                errors.append(f"{where}.id: remove surrounding whitespace.")
            if item["id"] in ids:
                errors.append(f"{where}.id: duplicate ID {item['id']!r}.")
            ids.add(item["id"])
        for field, choices in (("kind", KINDS), ("provenance", PROVENANCE)):
            if field in item and (
                not isinstance(item[field], str) or item[field] not in choices
            ):
                errors.append(f"{where}.{field}: choose from {', '.join(sorted(choices))}.")
        exceptions = item.get("exceptions", [])
        if not isinstance(exceptions, list):
            errors.append(f"{where}.exceptions: must be an array.")
            continue
        if exceptions and item.get("provenance") != "authored":
            errors.append(f"{where}: exceptions are only valid for authored records.")
        seen = set()
        for number, exception in enumerate(exceptions):
            at = f"{where}.exceptions[{number}]"
            if not isinstance(exception, dict):
                errors.append(f"{at}: must be an object.")
                continue
            rule = exception.get("rule")
            if rule not in ("phrase", "sentence-length"):
                errors.append(f"{at}.rule: choose phrase or sentence-length.")
                continue
            target = "phrase" if rule == "phrase" else "sentence"
            if set(exception) != {"rule", target, "reason"}:
                errors.append(f"{at}: requires exactly rule, {target}, and reason.")
            if not nonblank(exception.get("reason")):
                errors.append(f"{at}.reason: must be a nonblank string.")
            value = exception.get(target)
            valid = (
                isinstance(value, str) and value in PHRASES
                if rule == "phrase"
                else type(value) is int and value > 0
            )
            if not valid:
                expected = "a listed lowercase phrase" if rule == "phrase" else "a positive integer"
                errors.append(f"{at}.{target}: must be {expected}.")
            elif (rule, value) in seen:
                errors.append(f"{at}: duplicate exception scope.")
            else:
                seen.add((rule, value))
    return errors


def sentences(text):
    # Heuristic: a line break, or whitespace after . ! ?; decimals stay intact.
    return [part.strip() for part in re.split(r"(?<=[.!?])\s+|\n+", text.strip()) if part.strip()]


def approximate_words(text):
    # Whitespace tokens containing a letter or number; this is not STE counting.
    return sum(any(char.isalnum() for char in token) for token in text.split())


def review(catalog):
    counts = {name: sum(item["provenance"] == name for item in catalog) for name in PROVENANCE}
    excluded = len(catalog) - counts["authored"]
    print(f"Catalog: records={len(catalog)}; reviewed={counts['authored']}; excluded={excluded} "
          f"(imported={counts['imported']}, code={counts['code']}, quote={counts['quote']}).")
    print("Scope: supplied catalog only; phrase and approximate sentence-length checks.")
    print("Not a semantic, STE-compliance, or accessibility validator; coverage is not verified.")
    if not counts["authored"]:
        print("NOTICE: No authored copy was reviewed.")
    findings = open_findings = excepted = unused = exception_entries = 0
    for item in catalog:
        name = json.dumps(item["id"], ensure_ascii=False)
        if item["provenance"] != "authored":
            print(f"EXCLUDED {name}: provenance={item['provenance']}.")
            continue
        matches = []
        normalized = " ".join(item["text"].split()).casefold()
        for phrase, pattern in PATTERNS.items():
            occurrences = len(pattern.findall(normalized))
            if occurrences:
                matches.append((("phrase", phrase), f"phrase={phrase!r}; occurrences={occurrences}"))
        limit = LIMITS.get(item["kind"])
        if limit:
            for number, sentence in enumerate(sentences(item["text"]), 1):
                count = approximate_words(sentence)
                if count > limit:
                    matches.append((("sentence-length", number),
                                    f"sentence={number}; approximate_words={count}; limit={limit}; "
                                    f"text={json.dumps(sentence, ensure_ascii=False)}"))
        exceptions = {scope(entry): entry["reason"] for entry in item.get("exceptions", [])}
        exception_entries += len(exceptions)
        matched_scopes = {key for key, _ in matches}
        for key, message in matches:
            findings += 1
            if key in exceptions:
                excepted += 1
                print(f"EXCEPTION {name}: {message}; reason={json.dumps(exceptions[key], ensure_ascii=False)}")
            else:
                open_findings += 1
                print(f"WARNING {name}: {message}")
        for key, reason in exceptions.items():
            if key not in matched_scopes:
                unused += 1
                print(f"UNUSED EXCEPTION {name}: rule={key[0]}; target={key[1]!r}; "
                      f"reason={json.dumps(reason, ensure_ascii=False)}. No matching finding; review or remove it.")
    print(f"Summary: findings={findings}; open={open_findings}; excepted={excepted}; "
          f"exception_entries={exception_entries}; unused_exceptions={unused}.")
    if open_findings or unused:
        print("Result: review findings (exit 1).")
        return 1
    print("Result: no unexcepted findings in this catalog (exit 0).")
    return 0


def unique_keys(pairs):
    result = {}
    for key, value in pairs:
        if key in result:
            raise ValueError(f"Duplicate JSON key: {key!r}.")
        result[key] = value
    return result


def reject_constant(value):
    raise ValueError(f"Non-JSON numeric constant: {value}.")


def main(argv=None):
    parser = argparse.ArgumentParser(
        description="Review an explicit JSON UI copy catalog; never rewrite text or scan source files.",
        formatter_class=argparse.RawDescriptionHelpFormatter,
        epilog="""Schema: a nonempty JSON array. Every record requires:
  id: unique nonblank string; text: nonblank string
  kind: label | instruction | description | error | status
  provenance: authored | imported | code | quote
Optional exceptions: [{\"rule\":\"phrase\", \"phrase\":\"game-changing\", \"reason\":\"...\"}]
  or [{\"rule\":\"sentence-length\", \"sentence\":1, \"reason\":\"...\"}].
All records are validated. Only authored text is checked. Exclusions and
exceptions stay visible; unused exceptions are findings. Unknown fields fail.
Instructions: >20 approximate words per sentence. Descriptions, errors, and
statuses: >25. Labels have no length check. Decimal numbers do not split a
sentence; abbreviations and other punctuation can confuse the heuristic.
Exit 0: no unexcepted findings; 1: findings/unused exceptions; 2: invalid input.
Not a semantic, STE-compliance, or accessibility validator. Catalog completeness
and factual meaning require separate review. See references/copy-checker.md.
""",
    )
    parser.add_argument("catalog", help="UTF-8 JSON catalog path, or - for standard input")
    args = parser.parse_args(argv)
    try:
        raw = sys.stdin.read() if args.catalog == "-" else Path(args.catalog).read_text(encoding="utf-8")
        catalog = json.loads(raw, object_pairs_hook=unique_keys, parse_constant=reject_constant)
    except (OSError, UnicodeError, ValueError) as exc:
        print(f"INVALID INPUT: {exc}", file=sys.stderr)
        print("No copy review performed (exit 2).", file=sys.stderr)
        return 2
    errors = validate(catalog)
    if errors:
        for error in errors:
            print(f"INVALID INPUT: {error}", file=sys.stderr)
        print("No copy review performed (exit 2).", file=sys.stderr)
        return 2
    return review(catalog)


if __name__ == "__main__":
    sys.exit(main())
