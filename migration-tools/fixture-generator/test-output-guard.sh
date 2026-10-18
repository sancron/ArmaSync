#!/usr/bin/env sh
# SPDX-License-Identifier: AGPL-3.0-only
# Rejection tests for run.sh's mountpoint guard (Linux, needs root or CAP_SYS_ADMIN and `unshare`).
# Works in a throw-away copy of the generator below a temp dir, inside a private mount namespace, so nothing real is
# mounted or deleted. Bind mounts of the SAME filesystem are covered (a device-id comparison would miss them).
# Usage: sh migration-tools/fixture-generator/test-output-guard.sh
set -eu
if [ "${MIG001_GUARD_NS:-}" != 1 ]; then
  command -v unshare >/dev/null 2>&1 || { echo "SKIP: unshare not available" >&2; exit 2; }
  # Probe first: exec would replace this shell, so a failing unshare could not be turned into a skip afterwards.
  unshare -m -- true >/dev/null 2>&1 || { echo "SKIP: cannot create a mount namespace (need root / CAP_SYS_ADMIN)" >&2; exit 2; }
  MIG001_GUARD_NS=1 exec unshare -m -- sh "$0" "$@"
fi
mount --make-rprivate / 2>/dev/null || true
SRC="$(cd "$(dirname "$0")" && pwd -P)"
T="$(mktemp -d "${TMPDIR:-/tmp}/mig001-guard.XXXXXX")"
trap 'umount -R "$T/repo/fixtures/legacy-formats" 2>/dev/null || true; umount -R "$T/repo/build" 2>/dev/null || true; rm -rf --one-file-system "$T"' EXIT
R="$T/repo"; F="$T/foreign"
mkdir -p "$R/migration-tools/fixture-generator" "$R/fixtures/legacy-formats" "$R/build"
cp "$SRC/run.sh" "$R/migration-tools/fixture-generator/"
failures=0
fresh_foreign() {
  rm -rf --one-file-system "$F"; mkdir -p "$F/generated"
  echo keep > "$F/sentinel.txt"; echo keep > "$F/generated/sentinel.txt"
  echo '{"task": "MIG-001"}' > "$F/generated/manifest.json"   # would make a bypassed guard consider it deletable
}
intact() { [ -f "$F/sentinel.txt" ] && [ -f "$F/generated/sentinel.txt" ] && [ -f "$F/generated/manifest.json" ]; }
expect_refusal() {  # name, out-arg ("" = default)
  if [ -n "$2" ]; then out="$(sh "$R/migration-tools/fixture-generator/run.sh" "$2" 2>&1)" && rc=0 || rc=$?
  else out="$(sh "$R/migration-tools/fixture-generator/run.sh" 2>&1)" && rc=0 || rc=$?; fi
  if [ "$rc" -ne 0 ] && printf '%s' "$out" | grep -q "refusing" && intact; then echo "PASS  $1"
  else echo "FAIL  $1 (exit=$rc, intact=$(intact && echo yes || echo no))"; printf '%s\n' "$out" | sed 's/^/      /'; failures=$((failures+1)); fi
}
expect_allowed() {  # name, out-arg: must pass every guard (it may only fail later, at the baseline/build steps)
  out="$(sh "$R/migration-tools/fixture-generator/run.sh" "$1" 2>&1)" && rc=0 || rc=$?
  if printf '%s' "$out" | grep -q "refusing\|could not be evaluated\|Permission denied\|not found"; then
    echo "FAIL  $2 (guard refused a mount-free target)"; printf '%s
' "$out" | sed 's/^/      /'; failures=$((failures+1))
  elif printf '%s' "$out" | grep -q "differ from the baseline commit"; then echo "PASS  $2"   # reached the step after all guards
  else echo "FAIL  $2 (did not reach the post-guard baseline check; exit=$rc)"; printf '%s
' "$out" | sed 's/^/      /'; failures=$((failures+1)); fi
}
L=fixtures/legacy-formats
# positive cases: ordinary mount-free targets must NOT be refused by any guard
expect_allowed "$L/generated" "allowed: plain mount-free default target"
mkdir -p "$R/$L/plain dir"
expect_allowed "$L/plain dir/generated" "allowed: plain mount-free target with space"
fresh_foreign
mkdir "$R/$L/bind"; mount --bind "$F" "$R/$L/bind"
expect_refusal "bind mount: existing target below mount" "$L/bind/generated"
expect_refusal "bind mount: mountpoint itself as target" "$L/bind"
umount "$R/$L/bind"

# bind mount of a directory on the SAME filesystem (st_dev identical to its parent)
fresh_foreign; mkdir -p "$R/$L/same" "$R/$L/real/generated"; echo '{"task": "MIG-001"}' > "$R/$L/real/generated/manifest.json"
mount --bind "$R/$L/real" "$R/$L/same"
expect_refusal "same-filesystem bind mount as target parent" "$L/same/generated"
umount "$R/$L/same"

# tmpfs / foreign mount nested INSIDE a deletable generated directory (rm -rf would descend into it)
fresh_foreign; mkdir -p "$R/$L/generated/inner"; echo '{"task": "MIG-001"}' > "$R/$L/generated/manifest.json"
mount --bind "$F" "$R/$L/generated/inner"
expect_refusal "mount nested inside existing target" "$L/generated"
umount "$R/$L/generated/inner"

# build/fixture-generator (cleanup target) as mountpoint, and a mount beneath it
fresh_foreign; mkdir -p "$R/build/fixture-generator"; mount --bind "$F" "$R/build/fixture-generator"
expect_refusal "build/fixture-generator is a mountpoint" ""
umount "$R/build/fixture-generator"
fresh_foreign; mkdir -p "$R/build/fixture-generator/sub"; mount --bind "$F" "$R/build/fixture-generator/sub"
expect_refusal "mount beneath build/fixture-generator" ""
umount "$R/build/fixture-generator/sub"

# mount point with a space (mountinfo encodes it as \040)
fresh_foreign; mkdir -p "$R/$L/with space"; mount --bind "$F" "$R/$L/with space"
expect_refusal "mountpoint name containing a space" "$L/with space/generated"
umount "$R/$L/with space"

# literal backslash-octal sequence in a directory name (must not be decoded or altered by the guard)
fresh_foreign; mkdir -p "$R/$L/back\040slash"; mount --bind "$F" "$R/$L/back\040slash"
expect_refusal "mountpoint name with literal backslash-octal sequence" "$L/back\040slash/generated"
umount "$R/$L/back\040slash"

# symlinks (regression for the earlier -L checks)
fresh_foreign; ln -s "$F" "$R/$L/lnk"
expect_refusal "symlink: existing target behind link" "$L/lnk/generated"
expect_refusal "symlink: link itself as target" "$L/lnk"
rm "$R/$L/lnk"
rm -rf --one-file-system "$R/build/fixture-generator"
fresh_foreign; ln -s "$F" "$R/build/fixture-generator"
expect_refusal "symlink: build/fixture-generator is a link" ""
rm "$R/build/fixture-generator"

if [ "$failures" -gt 0 ]; then echo "$failures test(s) failed"; exit 1; fi
echo "all mountpoint guard tests passed"
