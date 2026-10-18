#!/usr/bin/env sh
# SPDX-License-Identifier: AGPL-3.0-only
# Table test for branch-policy.sh. Usage: sh .github/scripts/test-branch-policy.sh
set -u
dir="$(cd "$(dirname "$0")" && pwd)"
failures=0
check() { # expected-exit base head base_repo head_repo
  expected="$1"
  BASE_REF="$2" HEAD_REF="$3" BASE_REPO="$4" HEAD_REPO="$5" sh "$dir/branch-policy.sh" >/dev/null 2>&1
  actual=$?
  if [ "$actual" -eq "$expected" ]; then
    echo "PASS  $3 ($5) -> $2 ($4): exit $actual"
  else
    echo "FAIL  $3 ($5) -> $2 ($4): expected $expected, got $actual"
    failures=$((failures + 1))
  fi
}
O=sancron/ArmaSync
F=someone/ArmaSync
# allowed
check 0 development claude/mig-020-store $O $O
check 0 development codex/mig-021-import $O $O
check 0 development feature-x $F $O
check 0 testing development $O $O
check 0 testing rc-fix/crash-on-start $O $O
check 0 master testing $O $O
# rejected
check 1 master development $O $O
check 1 master claude/mig-020-store $O $O
check 1 master testing $F $O
check 1 testing claude/mig-020-store $O $O
check 1 testing development $F $O
check 1 testing master $O $O
check 1 development master $O $O
check 1 development testing $O $O
check 1 development legacy $O $O
check 1 legacy development $O $O
check 1 legacy claude/x $O $O
# missing input
check 2 "" development $O $O
if [ "$failures" -gt 0 ]; then echo "$failures test(s) failed"; exit 1; fi
echo "all branch policy tests passed"
