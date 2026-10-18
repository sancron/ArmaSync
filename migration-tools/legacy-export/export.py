# SPDX-License-Identifier: AGPL-3.0-only
"""MIG-021 offline exporter: legacy local files -> neutral import document (`armasync.import`, schema 1).

Usage:  python -I export.py <legacy-directory> <output.json>

What it does
  * reads the directory **read-only** (every file is opened with 'rb'; nothing is moved, changed or deleted),
  * decodes the unencrypted legacy files with the bounded, JVM-free reader of MIG-001
    (`migration-tools/fixture-generator/verify.py`, imported, not modified),
  * exports **profiles** (display name and the ordered names of the repositories they refer to),
  * lists encrypted repository files as *skipped* (their decryption is an open decision, ADR-0003 / RQ-01;
    this tool never carries or applies the legacy key),
  * never writes passwords, logins, favourite-server data, URLs with credentials or keys; settings files and
    favourite servers are only counted in notes,
  * refuses to overwrite an existing output file and refuses an output path inside the legacy directory.

The legacy class and field names below are wire data of the frozen baseline (ADR-0003); they stay inside this
tool and do not appear in the neutral document.

Exit status: 0 success; 2 usage or I/O problem (nothing written); 3 written, but some files were unreadable
(see the notes in the document and stderr).
"""

import importlib.util
import json
import os
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
READER_PATH = os.path.join(HERE, "..", "fixture-generator", "verify.py")

TOOL = "armasync-legacy-export"
TOOL_VERSION = "1"
FORMAT = "legacy-local-files"
MAX_FILES = 10_000
MAX_DEPTH = 6
MAX_FILE_BYTES = 8 * 1024 * 1024
MAX_NAME_CHARS = 120
SECRET_PARTS = ("password", "passwort", "passwd", "login", "secret", "token", "apikey", "credential")


def load_reader():
    sys.dont_write_bytecode = True  # importing the reader must not leave files in the fixture-generator directory
    spec = importlib.util.spec_from_file_location("legacy_reader", READER_PATH)
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def valid_name(value):
    return (
        isinstance(value, str)
        and 0 < len(value) <= MAX_NAME_CHARS
        and value == value.strip()
        and not any(ord(c) < 32 or 0x7F <= ord(c) < 0xA0 for c in value)
    )


def find_files(root):
    """Files by role, found below `root` without following links. Returns (dict role -> sorted paths, truncated)."""
    found = {"profile": [], "repository": [], "autoconfig-export": [], "settings": []}
    count = 0
    truncated = False
    root = os.path.abspath(root)
    for current, dirs, files in os.walk(root, followlinks=False):
        depth = current[len(root):].count(os.sep)
        dirs[:] = sorted(d for d in dirs if not os.path.islink(os.path.join(current, d)))
        if depth >= MAX_DEPTH:
            dirs[:] = []
        for name in sorted(files):
            path = os.path.join(current, name)
            if os.path.islink(path):
                continue
            count += 1
            if count > MAX_FILES:
                truncated = True
                return found, truncated
            if name.endswith(".a3s.profile"):
                found["profile"].append(path)
            elif name.endswith(".a3s.repository"):
                found["repository"].append(path)
            elif name == "export.a3s.autoconfig":
                found["autoconfig-export"].append(path)
            elif name in ("a3s.cfg", "a3s.prefs"):
                found["settings"].append(path)
    return found, truncated


# Wire descriptors of the legacy classes this tool consumes (serialVersionUID, from FORMAT_INVENTORY.md section 3).
# A stream whose descriptor differs was written by another class version; it is refused instead of guessed.
# Classes not listed here (for example the export container, whose UID is computed) are not checked.
EXPECTED_UIDS = {
    "fr.soe.a3s.domain.Profile": 0xC3017806DDD9C143,
    "fr.soe.a3s.domain.TreeDirectory": 0xC9A3A7B4C01584AF,
    "fr.soe.a3s.domain.TreeLeaf": 0x787BAC2F5273ECDA,
}


def read_object(reader, path):
    size = os.path.getsize(path)
    if size > MAX_FILE_BYTES:
        raise reader.LimitError("file too large")
    with open(path, "rb") as handle:
        raw = handle.read(MAX_FILE_BYTES + 1)
    top, state = reader.read_serialization(reader.gunzip_bounded(raw))
    for handle in state.handles:
        if isinstance(handle, reader.ClassDesc) and handle.name in EXPECTED_UIDS:
            if (handle.suid & 0xFFFFFFFFFFFFFFFF) != EXPECTED_UIDS[handle.name]:
                raise reader.FormatError("unexpected class version")
    return reader.neutral(top)


def profile_from(dump):
    """(name, ordered unique repository names) from a decoded legacy profile object."""
    if not isinstance(dump, dict) or dump.get("_type") != "Profile":
        return None
    name = dump.get("name")
    repositories = []

    def walk(node):
        if not isinstance(node, dict):
            return
        repo = node.get("modsetRepositoryName")
        if node.get("modsetType") == "REPOSITORY" and isinstance(repo, str) and repo not in repositories:
            repositories.append(repo)
        children = node.get("list")
        if isinstance(children, list):
            for child in children:
                walk(child)

    walk(dump.get("tree"))
    return name, repositories


def repository_name_from_file(path):
    base = os.path.basename(path)
    return base[: -len(".a3s.repository")]


def assert_no_secret_keys(value):
    if isinstance(value, dict):
        for key, inner in value.items():
            lowered = key.lower()
            if lowered != "needs_credentials" and any(part in lowered for part in SECRET_PARTS):
                raise AssertionError("secret-like key in export: " + key)
            assert_no_secret_keys(inner)
    elif isinstance(value, list):
        for item in value:
            assert_no_secret_keys(item)


def build_document(legacy_dir):
    reader = load_reader()
    files, truncated = find_files(legacy_dir)
    notes = []
    problems = 0
    profiles = {}

    def note(code, subject=None, count=None):
        entry = {"code": code}
        if subject is not None and valid_name(subject):
            entry["subject"] = subject
        if count is not None:
            entry["count"] = count
        notes.append(entry)

    if truncated:
        note("scan-truncated", count=MAX_FILES)

    def add_profile(dump, origin):
        nonlocal problems
        parsed = profile_from(dump)
        if parsed is None:
            problems += 1
            note("skipped-unexpected-content", os.path.basename(origin))
            return
        name, repositories = parsed
        if not valid_name(name):
            problems += 1
            note("skipped-invalid-profile-name")
            return
        if name in profiles:
            note("skipped-duplicate-profile", name)
            return
        profiles[name] = repositories

    for path in files["profile"]:
        try:
            add_profile(read_object(reader, path), path)
        except (reader.FormatError, OSError):
            problems += 1
            note("skipped-unreadable-file", os.path.basename(path))

    for path in files["autoconfig-export"]:
        try:
            dump = read_object(reader, path)
        except (reader.FormatError, OSError):
            problems += 1
            note("skipped-unreadable-file", os.path.basename(path))
            continue
        if not isinstance(dump, dict) or dump.get("_type") != "AutoConfig":
            problems += 1
            note("skipped-unexpected-content", os.path.basename(path))
            continue
        for item in dump.get("profiles") or []:
            add_profile(item, path)
        favourites = dump.get("favoriteServers") or []
        if favourites:
            note("skipped-favorite-servers", count=len(favourites))

    for path in files["repository"]:
        note("skipped-encrypted-repository", repository_name_from_file(path), 1)
    if files["settings"]:
        note("skipped-settings", count=len(files["settings"]))

    document = {
        "kind": "armasync.import",
        "schema_version": 1,
        "source": {"tool": TOOL, "tool_version": TOOL_VERSION, "format": FORMAT},
        "repositories": [],
        "profiles": [
            {"name": name, "repositories": profiles[name]} for name in sorted(profiles)
        ],
        "notes": notes,
    }
    assert_no_secret_keys(document)
    return document, problems


def inside(path, directory):
    path = os.path.realpath(path)
    directory = os.path.realpath(directory)
    try:
        return os.path.commonpath([path, directory]) == directory
    except ValueError:  # different drives
        return False


def main(argv):
    if len(argv) != 3 or argv[1] in ("-h", "--help"):
        sys.stderr.write(__doc__.split("\n\n")[0] + "\nUsage: python -I export.py <legacy-directory> <output.json>\n")
        return 2
    legacy_dir, output = argv[1], argv[2]
    if not os.path.isdir(legacy_dir):
        sys.stderr.write("error: '%s' is not a directory\n" % legacy_dir)
        return 2
    if os.path.lexists(output):
        sys.stderr.write("error: '%s' already exists; it is never overwritten\n" % output)
        return 2
    if inside(output, legacy_dir):
        sys.stderr.write("error: the output must not be inside the legacy directory\n")
        return 2
    parent = os.path.dirname(os.path.abspath(output))
    if not os.path.isdir(parent):
        sys.stderr.write("error: the output directory '%s' does not exist\n" % parent)
        return 2
    document, problems = build_document(legacy_dir)
    text = json.dumps(document, indent=2, ensure_ascii=False) + "\n"
    created = False
    try:
        with open(output, "x", encoding="utf-8", newline="\n") as handle:
            created = True
            handle.write(text)
    except OSError as error:
        sys.stderr.write("error: cannot write '%s': %s\n" % (output, error.strerror))
        if created:  # never remove a file this run did not create
            try:
                os.remove(output)
            except OSError:
                pass
        return 2
    skipped = sum(1 for n in document["notes"] if n["code"].startswith("skipped-"))
    sys.stderr.write(
        "exported %d profile(s); %d note(s) about skipped items, %d unreadable file(s)\n"
        % (len(document["profiles"]), skipped, problems)
    )
    return 3 if problems else 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
