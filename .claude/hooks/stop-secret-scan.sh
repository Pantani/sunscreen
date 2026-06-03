#!/usr/bin/env bash
# Stop hook: warn if staged git changes contain common secret patterns.
set -u
cd "${CLAUDE_PROJECT_DIR:-.}" || exit 0
command -v git >/dev/null || exit 0
git rev-parse --git-dir >/dev/null || exit 0

PATTERN='AKIA[0-9A-Z]{16}|-----BEGIN (RSA|OPENSSH|DSA|EC|PGP) PRIVATE KEY-----|api[_-]?key["'"'"' :=]+[A-Za-z0-9_-]{20,}|secret[_-]?key["'"'"' :=]+[A-Za-z0-9_-]{20,}'

if git diff --cached -U0 | grep -E -i "$PATTERN" >/dev/null; then
  echo "WARNING: potential secret in staged changes" >&2
fi
exit 0
