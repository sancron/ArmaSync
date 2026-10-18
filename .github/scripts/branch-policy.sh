#!/usr/bin/env sh
# SPDX-License-Identifier: AGPL-3.0-only
# Branch flow check for pull requests (docs/BRANCHING.md): task branch -> development -> testing -> master,
# `legacy` is never a target. Inputs (environment): BASE_REF, HEAD_REF, BASE_REPO, HEAD_REPO (owner/name).
# Exit 0 = allowed, 1 = rejected (message on stderr), 2 = missing input.
set -eu

need() {
  eval "value=\${$1:-}"
  if [ -z "$value" ]; then
    echo "branch policy: $1 is required" >&2
    exit 2
  fi
}
need BASE_REF
need HEAD_REF
need BASE_REPO
need HEAD_REPO

reject() {
  echo "branch policy: pull request from '$HEAD_REPO:$HEAD_REF' into '$BASE_REPO:$BASE_REF' is not allowed: $1" >&2
  exit 1
}

same_repo=false
[ "$BASE_REPO" = "$HEAD_REPO" ] && same_repo=true

case "$BASE_REF" in
  legacy)
    reject "'legacy' stays at the documented baseline commit"
    ;;
  master)
    [ "$same_repo" = true ] || reject "pull requests from forks may only target 'development'"
    [ "$HEAD_REF" = "testing" ] || reject "only 'testing' may be merged into 'master'"
    ;;
  testing)
    [ "$same_repo" = true ] || reject "pull requests from forks may only target 'development'"
    case "$HEAD_REF" in
      development | rc-fix/*) ;;
      *) reject "only 'development' or an explicitly named 'rc-fix/*' branch may be merged into 'testing'" ;;
    esac
    ;;
  development)
    case "$HEAD_REF" in
      master | testing | legacy) reject "'$HEAD_REF' must not be merged back into 'development' by pull request" ;;
    esac
    ;;
  *)
    echo "branch policy: no rule for target '$BASE_REF' (only legacy, master, testing and development are governed)"
    ;;
esac
echo "branch policy: '$HEAD_REPO:$HEAD_REF' -> '$BASE_REPO:$BASE_REF' allowed"
