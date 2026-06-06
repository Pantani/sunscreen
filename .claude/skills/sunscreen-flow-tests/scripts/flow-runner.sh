#!/usr/bin/env bash
# flow-runner.sh — runs all offline sunscreen flow tests and prints a summary.
# Usage: bash .claude/skills/sunscreen-flow-tests/scripts/flow-runner.sh
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
export SUNSCREEN_BIN="${SUNSCREEN_BIN:-$(cd "$SCRIPT_DIR/../../../.." && pwd)/target/release/sunscreen}"

if [[ ! -x "$SUNSCREEN_BIN" ]]; then
  echo "[ERROR] Binary not found at $SUNSCREEN_BIN — run: cargo build --release"
  exit 1
fi

FLOWS=(
  "flow-nft.sh:zero-to-NFT"
  "flow-token.sh:zero-to-token"
  "flow-smart-contract.sh:zero-to-smart-contract"
)

PASS=0
FAIL=0
RESULTS=()

for entry in "${FLOWS[@]}"; do
  script="${entry%%:*}"
  label="${entry#*:}"
  echo ""
  echo "━━━ $label ━━━"
  if bash "$SCRIPT_DIR/$script"; then
    RESULTS+=("  [PASS] $label")
    PASS=$((PASS + 1))
  else
    RESULTS+=("  [FAIL] $label")
    FAIL=$((FAIL + 1))
  fi
done

echo ""
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo " Flow Test Summary"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
for r in "${RESULTS[@]}"; do echo "$r"; done
echo ""
echo " Passed: $PASS  Failed: $FAIL"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"

[[ $FAIL -eq 0 ]]
