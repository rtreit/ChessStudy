#!/usr/bin/env python3
"""Run with Python 3; uses only its standard library and temporary fixtures."""

import json
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest


SCRIPT = Path(__file__).with_name("check_copy.py")


def record(text="View results", **changes):
    item = {"id": "results", "text": text, "kind": "description", "provenance": "authored"}
    item.update(changes)
    return item


class CopyCheckerTests(unittest.TestCase):
    def check(self, data, raw=False):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "copy.json"
            path.write_text(data if raw else json.dumps(data), encoding="utf-8")
            return subprocess.run([sys.executable, str(SCRIPT), str(path)],
                                  text=True, capture_output=True, check=False)

    def test_technical_words_are_not_a_blacklist(self):
        result = self.check([record("Inspect the exploit payload, attack log, and robustness benchmark.")])
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertIn("reviewed=1; excluded=0", result.stdout)

    def test_phrase_boundaries_case_and_whitespace(self):
        result = self.check([record("Unlock  powerful insights.")])
        self.assertEqual(result.returncode, 1)
        self.assertIn("occurrences=1", result.stdout)
        result = self.check([record("The term next-generational is a quoted label in this example.")])
        self.assertEqual(result.returncode, 0)

    def test_instruction_limit_and_label_exemption(self):
        for count, expected in ((20, 0), (21, 1)):
            with self.subTest(count=count):
                result = self.check([record(" ".join(["word"] * count), kind="instruction")])
                self.assertEqual(result.returncode, expected)
        self.assertEqual(self.check([record("word " * 30, kind="label")]).returncode, 0)

    def test_prose_limit_and_decimal_not_a_sentence_boundary(self):
        text = " ".join(["word"] * 10 + ["3.14"] + ["word"] * 15) + "."
        result = self.check([record(text)])
        self.assertEqual(result.returncode, 1)
        self.assertIn("sentence=1; approximate_words=26; limit=25", result.stdout)
        for kind in ("description", "error", "status"):
            self.assertEqual(self.check([record("word " * 25, kind=kind)]).returncode, 0)

    def test_sentence_scopes_are_one_based(self):
        text = "View results. " + "word " * 26
        result = self.check([record(text)])
        self.assertIn("sentence=2; approximate_words=26", result.stdout)

    def test_every_missing_record_is_reported_before_review(self):
        result = self.check([{"id": "one"}, {"text": "View results"}])
        self.assertEqual(result.returncode, 2)
        self.assertIn("record[0]", result.stderr)
        self.assertIn("record[1]", result.stderr)
        self.assertIn("No copy review performed", result.stderr)
        self.assertEqual(result.stdout, "")

    def test_invalid_roots_fields_and_types(self):
        bad = [[], {}, [None], [record(kind=[])], [record(provenance="generated")],
               [record(text=" ")], [record(extra=True)], [record(exceptions={})]]
        for data in bad:
            with self.subTest(data=data):
                self.assertEqual(self.check(data).returncode, 2)

    def test_duplicate_ids_keys_and_malformed_json(self):
        self.assertEqual(self.check([record(), record()]).returncode, 2)
        for raw in ('[{"id":"one","id":"two"}]', '{', '[NaN]'):
            with self.subTest(raw=raw):
                self.assertEqual(self.check(raw, raw=True).returncode, 2)

    def test_exclusions_are_visible_and_still_validated(self):
        items = [record("game-changing " + "word " * 30, id=p, provenance=p)
                 for p in ("imported", "code", "quote")]
        result = self.check(items)
        self.assertEqual(result.returncode, 0)
        self.assertIn("reviewed=0; excluded=3", result.stdout)
        self.assertIn("No authored copy was reviewed", result.stdout)
        self.assertEqual(result.stdout.count("EXCLUDED "), 3)
        del items[0]["kind"]
        self.assertEqual(self.check(items).returncode, 2)

    def test_phrase_exception_remains_visible(self):
        exception = {"rule": "phrase", "phrase": "game-changing", "reason": "Required product tagline."}
        result = self.check([record("A game-changing feature.", exceptions=[exception])])
        self.assertEqual(result.returncode, 0)
        self.assertIn("EXCEPTION ", result.stdout)
        self.assertIn("Required product tagline.", result.stdout)
        self.assertIn("excepted=1; exception_entries=1; unused_exceptions=0", result.stdout)

    def test_sentence_exception_remains_visible(self):
        exception = {"rule": "sentence-length", "sentence": 1, "reason": "Required wording."}
        result = self.check([record("word " * 26, exceptions=[exception])])
        self.assertEqual(result.returncode, 0)
        self.assertIn("EXCEPTION ", result.stdout)
        self.assertIn("approximate_words=26", result.stdout)
        self.assertIn("Required wording.", result.stdout)

    def test_invalid_exception_reasons_scopes_and_types(self):
        base = {"rule": "sentence-length", "sentence": 1, "reason": "Required wording."}
        bad = [{**base, "reason": " "}, {**base, "sentence": True}, {**base, "extra": 1},
               {"rule": "phrase", "phrase": "robust", "reason": "Technical term."}, None]
        for exception in bad:
            with self.subTest(exception=exception):
                self.assertEqual(self.check([record(exceptions=[exception])]).returncode, 2)
        self.assertEqual(self.check([record(exceptions=[base, base])]).returncode, 2)
        self.assertEqual(self.check([record(provenance="quote", exceptions=[base])]).returncode, 2)

    def test_unused_exception_is_a_visible_finding(self):
        exception = {"rule": "phrase", "phrase": "game-changing", "reason": "Old tagline."}
        result = self.check([record("View results", exceptions=[exception])])
        self.assertEqual(result.returncode, 1)
        self.assertIn("UNUSED EXCEPTION", result.stdout)
        self.assertIn("unused_exceptions=1", result.stdout)

    def test_exception_does_not_waive_other_findings(self):
        exception = {"rule": "phrase", "phrase": "game-changing", "reason": "Required tagline."}
        result = self.check([record("game-changing " + "word " * 26, exceptions=[exception])])
        self.assertEqual(result.returncode, 1)
        self.assertIn("open=1; excepted=1", result.stdout)

    def test_help_and_stdin(self):
        result = subprocess.run([sys.executable, str(SCRIPT), "--help"], text=True, capture_output=True)
        self.assertEqual(result.returncode, 0)
        self.assertIn("Not a semantic, STE-compliance, or accessibility validator", result.stdout)
        result = subprocess.run([sys.executable, str(SCRIPT), "-"], input=json.dumps([record()]),
                                text=True, capture_output=True)
        self.assertEqual(result.returncode, 0)


if __name__ == "__main__":
    unittest.main(verbosity=2)
