#!/usr/bin/env bash
# flow-token.sh — zero-to-SPL-token complete user journey test.
# Tests: quickstart token → scaffold instruction/account/event/error → verify files
# Toolchain: offline (SUNSCREEN_SKIP_PREFLIGHT=1)
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
SUNSCREEN_BIN="${SUNSCREEN_BIN:-$(cd "$SCRIPT_DIR/../../../.." && pwd)/target/release/sunscreen}"
export SUNSCREEN_SKIP_PREFLIGHT=1

if [[ ! -x "$SUNSCREEN_BIN" ]]; then
  echo "[ERROR] Binary not found: $SUNSCREEN_BIN — run: cargo build --release"
  exit 1
fi

FLOW_DIR="${FLOW_TMPDIR:-/tmp}/sunscreen-flow-token-$(date +%s)"
WORKSPACE_NAME="testtoken$(date +%s)"
trap 'if [[ ${FAIL:-0} -eq 0 ]]; then rm -rf "$FLOW_DIR"; else echo "[INFO] Preserving failed flow dir: $FLOW_DIR"; fi' EXIT

mkdir -p "$FLOW_DIR"

PASS=0
FAIL=0

assert_cmd() {
  local expected_exit="$1"
  local description="$2"
  shift 2
  local actual_exit=0
  local output
  output=$("$@" 2>&1) || actual_exit=$?
  if [[ "$actual_exit" -eq "$expected_exit" ]]; then
    echo "[PASS] $description"
    PASS=$((PASS + 1))
  else
    echo "[FAIL] $description (expected exit $expected_exit, got $actual_exit)"
    echo "       output: $(echo "$output" | head -5)"
    FAIL=$((FAIL + 1))
  fi
}

assert_file() {
  local path="$1"
  local description="$2"
  if [[ -e "$path" ]]; then
    echo "[PASS] $description"
    PASS=$((PASS + 1))
  else
    echo "[FAIL] $description — file missing: $path"
    FAIL=$((FAIL + 1))
  fi
}

echo "=== Flow: zero-to-token (SPL Token) ==="
echo "    binary: $SUNSCREEN_BIN"
echo "    workspace: $FLOW_DIR/$WORKSPACE_NAME"
echo ""

cd "$FLOW_DIR"

# Step 1: quickstart token
assert_cmd 0 "quickstart token creates workspace" \
  "$SUNSCREEN_BIN" quickstart token --name "$WORKSPACE_NAME" --non-interactive

# Step 2: workspace structure
assert_file "$FLOW_DIR/$WORKSPACE_NAME/sunscreen.yml"  "sunscreen.yml exists"
assert_file "$FLOW_DIR/$WORKSPACE_NAME/Anchor.toml"    "Anchor.toml exists"

# Step 3: scaffold instructions (no --program)
cd "$FLOW_DIR/$WORKSPACE_NAME"

assert_cmd 0 "scaffold instruction mint_to" \
  "$SUNSCREEN_BIN" scaffold instruction mint_to

assert_cmd 0 "scaffold instruction burn" \
  "$SUNSCREEN_BIN" scaffold instruction burn

assert_cmd 0 "scaffold instruction transfer_checked" \
  "$SUNSCREEN_BIN" scaffold instruction transfer_checked

# Step 4: verify instruction files
assert_file "programs/${WORKSPACE_NAME}/src/instructions/mint_to.rs"          "mint_to.rs exists"
assert_file "programs/${WORKSPACE_NAME}/src/instructions/burn.rs"             "burn.rs exists"
assert_file "programs/${WORKSPACE_NAME}/src/instructions/transfer_checked.rs" "transfer_checked.rs exists"

# Step 5: scaffold accounts
assert_cmd 0 "scaffold account TokenAccount" \
  "$SUNSCREEN_BIN" scaffold account TokenAccount

assert_cmd 0 "scaffold account MintAuthority" \
  "$SUNSCREEN_BIN" scaffold account MintAuthority

assert_file "programs/${WORKSPACE_NAME}/src/state/token_account.rs"    "token_account.rs exists"
assert_file "programs/${WORKSPACE_NAME}/src/state/mint_authority.rs"   "mint_authority.rs exists"

# Step 6: scaffold events and errors
assert_cmd 0 "scaffold event TokenMinted" \
  "$SUNSCREEN_BIN" scaffold event TokenMinted

assert_cmd 0 "scaffold error InsufficientFunds" \
  "$SUNSCREEN_BIN" scaffold error InsufficientFunds

assert_file "programs/${WORKSPACE_NAME}/src/events.rs"  "events.rs exists"
assert_file "programs/${WORKSPACE_NAME}/src/errors.rs"  "errors.rs exists"

echo ""
echo "━━━ zero-to-token: PASS=$PASS FAIL=$FAIL ━━━"
[[ $FAIL -eq 0 ]]
