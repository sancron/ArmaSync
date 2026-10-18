# SPDX-License-Identifier: AGPL-3.0-only
"""Tests of the MIG-021 offline exporter against the synthetic MIG-001 fixtures.

Run: python -I -m unittest discover -s migration-tools/legacy-export -p "test_*.py"
"""

import hashlib
import json
import os
import shutil
import subprocess
import sys
import tempfile
import unittest

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.abspath(os.path.join(HERE, "..", ".."))
EXPORT = os.path.join(HERE, "export.py")
LEGACY = os.path.join(ROOT, "fixtures", "legacy-formats", "generated")
LOCAL = os.path.join(LEGACY, "valid", "local")
GOLDEN = os.path.join(ROOT, "fixtures", "neutral-import", "from-legacy-local.json")
SECRETS = [
    "fixture-password-not-a-secret",
    "fixture-server-password-not-a-secret",
    "fixture-user",
    "192.0.2.",
    "repo.example.invalid",
    "upload.example.invalid",
]


def run(*args):
    return subprocess.run([sys.executable, "-I", EXPORT, *args], capture_output=True, text=True, timeout=120)


def tree_hashes(directory):
    out = {}
    for current, _dirs, files in os.walk(directory):
        for name in files:
            path = os.path.join(current, name)
            with open(path, "rb") as handle:
                out[os.path.relpath(path, directory)] = hashlib.sha256(handle.read()).hexdigest()
    return out


class ExporterTest(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.mkdtemp(prefix="armasync-export-test-")
        self.addCleanup(shutil.rmtree, self.tmp, ignore_errors=True)

    def out(self, name="export.json"):
        return os.path.join(self.tmp, name)

    def test_matches_the_golden_file_and_leaves_the_input_untouched(self):
        before = tree_hashes(LOCAL)
        result = run(LOCAL, self.out())
        # Exit 3 = written, but one fixture (a profile with a >120 character name) is skipped on purpose.
        self.assertEqual(result.returncode, 3, result.stderr)
        self.assertEqual(tree_hashes(LOCAL), before, "the legacy directory must not change")
        with open(self.out(), encoding="utf-8") as handle:
            produced = json.load(handle)
        with open(GOLDEN, encoding="utf-8") as handle:
            golden = json.load(handle)
        self.assertEqual(produced, golden)

    def test_no_secrets_or_addresses_in_the_export(self):
        run(LOCAL, self.out())
        with open(self.out(), encoding="utf-8") as handle:
            text = handle.read()
        for secret in SECRETS:
            self.assertNotIn(secret, text)
        lowered = text.lower()
        for word in ("password", "login", "token", "secret"):
            self.assertNotIn(word, lowered)

    def test_encrypted_repositories_are_listed_as_skipped_not_exported(self):
        run(LOCAL, self.out())
        with open(self.out(), encoding="utf-8") as handle:
            document = json.load(handle)
        self.assertEqual(document["repositories"], [])
        skipped = sorted(n["subject"] for n in document["notes"] if n["code"] == "skipped-encrypted-repository")
        self.assertEqual(skipped, ["Example-Http", "Example-Sftp"])

    def test_output_is_deterministic(self):
        run(LOCAL, self.out("a.json"))
        run(LOCAL, self.out("b.json"))
        with open(self.out("a.json"), "rb") as a, open(self.out("b.json"), "rb") as b:
            self.assertEqual(a.read(), b.read())

    def test_existing_output_is_never_overwritten(self):
        target = self.out()
        with open(target, "w", encoding="utf-8") as handle:
            handle.write("precious")
        result = run(LOCAL, target)
        self.assertEqual(result.returncode, 2)
        with open(target, encoding="utf-8") as handle:
            self.assertEqual(handle.read(), "precious")

    def test_output_inside_the_legacy_directory_is_refused(self):
        legacy = os.path.join(self.tmp, "legacy")
        shutil.copytree(LOCAL, legacy)
        before = tree_hashes(legacy)
        result = run(legacy, os.path.join(legacy, "export.json"))
        self.assertEqual(result.returncode, 2)
        self.assertEqual(tree_hashes(legacy), before)

    def test_bad_arguments_and_missing_directories(self):
        self.assertEqual(run().returncode, 2)
        self.assertEqual(run(os.path.join(self.tmp, "missing"), self.out()).returncode, 2)
        self.assertEqual(run(LOCAL, os.path.join(self.tmp, "no-dir", "x.json")).returncode, 2)
        self.assertFalse(os.path.exists(self.out()))

    def test_damaged_files_are_skipped_with_notes_and_never_crash(self):
        legacy = os.path.join(self.tmp, "legacy")
        os.makedirs(os.path.join(legacy, "profiles"))
        invalid = os.path.join(LEGACY, "invalid")
        samples = [name for name in sorted(os.listdir(invalid)) if name.endswith(".bin")]
        self.assertGreaterEqual(len(samples), 10)
        for index, name in enumerate(samples):
            shutil.copy(os.path.join(invalid, name), os.path.join(legacy, "profiles", "Bad%d.a3s.profile" % index))
        with open(os.path.join(legacy, "profiles", "Empty.a3s.profile"), "wb"):
            pass
        with open(os.path.join(legacy, "profiles", "Random.a3s.profile"), "wb") as handle:
            handle.write(bytes(range(256)) * 8)
        result = run(legacy, self.out())
        self.assertEqual(result.returncode, 3, result.stderr)
        with open(self.out(), encoding="utf-8") as handle:
            document = json.load(handle)
        self.assertEqual(document["profiles"], [])
        codes = {n["code"] for n in document["notes"]}
        self.assertTrue(codes <= {"skipped-unreadable-file", "skipped-unexpected-content"}, codes)

    def test_wrong_object_type_is_skipped(self):
        legacy = os.path.join(self.tmp, "legacy")
        os.makedirs(legacy)
        # a valid serialisation of a different type (the preferences file) under a profile file name
        shutil.copy(os.path.join(LOCAL, "a3s.prefs"), os.path.join(legacy, "Fake.a3s.profile"))
        result = run(legacy, self.out())
        self.assertEqual(result.returncode, 3)
        with open(self.out(), encoding="utf-8") as handle:
            notes = json.load(handle)["notes"]
        self.assertEqual(notes[0]["code"], "skipped-unexpected-content")


if __name__ == "__main__":
    unittest.main()
