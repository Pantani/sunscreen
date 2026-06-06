#!/usr/bin/env bash
# flow-nft.sh — zero-to-NFT complete user journey test.
# Tests: quickstart nft → scaffold instruction/account/event/error → verify files
# Toolchain: offline (SUNSCREEN_SKIP_PREFLIGHT=1 — no Anchor/Solana needed)
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
SUNSCREEN_BIN="${SUNSCREEN_BIN:-$(cd "$SCRIPT_DIR/../../../.." && pwd)/target/release/sunscreen}"
export SUNSCREEN_SKIP_PREFLIGHT=1

if [[ ! -x "$SUNSCREEN_BIN" ]]; then
  echo "[ERROR] Binary not found: $SUNSCREEN_BIN — run: cargo build --release"
  exit 1
fi

# Isolated temp workspace
FLOW_DIR="${FLOW_TMPDIR:-/tmp}/sunscreen-flow-nft-$(date +%s)"
WORKSPACE_NAME="testnft$(date +%s)"
trap 'rm -rf "$FLOW_DIR"' EXIT

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

echo "=== Flow: zero-to-NFT ==="
echo "    binary: $SUNSCREEN_BIN"
echo "    workspace: $FLOW_DIR/$WORKSPACE_NAME"
echo ""

# Step 1: quickstart nft
cd "$FLOW_DIR"
assert_cmd 0 "quickstart nft creates workspace" \
  "$SUNSCREEN_BIN" quickstart nft --name "$WORKSPACE_NAME" --non-interactive

# Step 2: workspace structure
assert_file "$FLOW_DIR/$WORKSPACE_NAME/sunscreen.yml"    "sunscreen.yml exists"
assert_file "$FLOW_DIR/$WORKSPACE_NAME/Anchor.toml"      "Anchor.toml exists"
assert_file "$FLOW_DIR/$WORKSPACE_NAME/programs/"        "programs/ directory exists"

# Step 3: scaffold instruction (no --program, should auto-detect)
cd "$FLOW_DIR/$WORKSPACE_NAME"
assert_cmd 0 "scaffold instruction mint (auto-detects program)" \
  "$SUNSCREEN_BIN" scaffold instruction mint

# Step 4: verify instruction files
assert_file "programs/${WORKSPACE_NAME}/src/instructions/mint.rs" \
  "instruction file created: mint.rs"
assert_file "programs/${WORKSPACE_NAME}/src/instructions/mod.rs" \
  "mod.rs updated with mint"

# Step 5: scaffold account
assert_cmd 0 "scaffold account NftMetadata (no --program)" \
  "$SUNSCREEN_BIN" scaffold account NftMetadata

assert_file "programs/${WORKSPACE_NAME}/src/state/nft_metadata.rs" \
  "account file created: nft_metadata.rs"

# Step 6: scaffold event
assert_cmd 0 "scaffold event NftMinted (no --program)" \
  "$SUNSCREEN_BIN" scaffold event NftMinted

assert_file "programs/${WORKSPACE_NAME}/src/events.rs" \
  "events.rs created"

# Step 7: scaffold error
assert_cmd 0 "scaffold error InvalidMint (no --program)" \
  "$SUNSCREEN_BIN" scaffold error InvalidMint

assert_file "programs/${WORKSPACE_NAME}/src/errors.rs" \
  "errors.rs created"

# Step 8: idempotency — scaffolding the same name again must exit 0 (no-op), not crash
assert_cmd 0 "duplicate instruction mint → exit 0 (idempotent no-op)" \
  "$SUNSCREEN_BIN" scaffold instruction mint

# Step 9: learn (basic sanity)
assert_cmd 0 "learn (topic list)" \
  "$SUNSCREEN_BIN" learn

assert_cmd 0 "learn list" \
  "$SUNSCREEN_BIN" learn list

assert_cmd 0 "learn pda" \
  "$SUNSCREEN_BIN" learn pda

# Step 10: doctor (offline — may report missing tools, must not crash)
assert_cmd 0 "doctor exits 0 (reports tool status)" \
  "$SUNSCREEN_BIN" doctor || true  # doctor may exit non-zero if tools missing — accept both

echo ""
echo "━━━ zero-to-NFT: PASS=$PASS FAIL=$FAIL ━━━"
[[ $FAIL -eq 0 ]]
