#!/usr/bin/env bash
# Heavy integration runner for sunscreen.
#
# Default mode runs deterministic offline gates. Opt into real Solana tooling,
# cargo-dist, generated compile tests, and flake loops with env vars.

set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

LOG_DIR="${SUNSCREEN_HEAVY_LOG_DIR:-_workspace/test-harness}"
STAMP="$(date -u +%Y%m%dT%H%M%SZ)"
LOG_FILE="$LOG_DIR/heavy-$STAMP.log"
SUMMARY_FILE="$LOG_DIR/heavy-$STAMP.summary.json"
SUMMARY_TSV="$LOG_DIR/heavy-$STAMP.summary.tsv"
REAL_TOOLCHAIN="${SUNSCREEN_REAL_TOOLCHAIN:-0}"
COMPILE_TESTS="${SUNSCREEN_COMPILE_TESTS:-0}"
DIST_TESTS="${SUNSCREEN_DIST:-0}"
FLAKE_RUNS="${SUNSCREEN_FLAKE_RUNS:-0}"
PINOCCHIO_SBF="${SUNSCREEN_PINOCCHIO_SBF:-0}"
CURRENT_TIER=""

mkdir -p "$LOG_DIR"
: > "$SUMMARY_TSV"
exec > >(tee -a "$LOG_FILE") 2>&1

record_tier() {
    local tier="$1"
    local status="$2"
    local note="${3:-}"
    printf '%s\t%s\t%s\n' "$tier" "$status" "$note" >> "$SUMMARY_TSV"
}

finalize() {
    local code="$1"
    trap - EXIT
    local status="passed"
    if [[ "$code" -ne 0 ]]; then
        status="failed"
    fi

    python3 - "$SUMMARY_TSV" "$SUMMARY_FILE" "$LOG_FILE" "$status" "$code" <<'PY'
import json
import sys

summary_tsv, summary_json, log_file, status, code = sys.argv[1:6]
tiers = []
try:
    with open(summary_tsv, encoding="utf-8") as handle:
        for raw in handle:
            raw = raw.rstrip("\n")
            if not raw:
                continue
            parts = raw.split("\t", 2)
            while len(parts) < 3:
                parts.append("")
            tiers.append({"tier": parts[0], "status": parts[1], "note": parts[2]})
except FileNotFoundError:
    pass

payload = {
    "status": status,
    "exit_code": int(code),
    "log": log_file,
    "tiers": tiers,
}
with open(summary_json, "w", encoding="utf-8") as handle:
    json.dump(payload, handle, indent=2, sort_keys=True)
    handle.write("\n")
PY
    echo "heavy integration summary: $SUMMARY_FILE"
    exit "$code"
}

trap 'finalize $?' EXIT

section() {
    printf '\n==> %s\n' "$1"
}

run() {
    printf '+'
    printf ' %q' "$@"
    printf '\n'
    "$@"
}

run_shell() {
    printf '+ %s\n' "$*"
    bash -lc "$*"
}

run_allow_exit() {
    local allowed="$1"
    shift
    printf '+'
    printf ' %q' "$@"
    printf ' # allowed exits: %s\n' "$allowed"

    set +e
    "$@"
    local code=$?
    set -e

    case " $allowed " in
        *" $code "*) return 0 ;;
    esac

    echo "unexpected exit code $code; allowed: $allowed"
    return "$code"
}

require_tool() {
    local tool="$1"
    if ! command -v "$tool" >/dev/null 2>&1; then
        echo "missing required tool for real validation: $tool"
        echo "This is a blocked real-toolchain run, not a passing run."
        if [[ -n "$CURRENT_TIER" ]]; then
            record_tier "$CURRENT_TIER" "blocked" "missing required tool: $tool"
        fi
        exit 2
    fi
}

require_number() {
    local name="$1"
    local value="$2"
    if [[ "$value" =~ [^0-9] ]]; then
        echo "$name must be a non-negative integer; got '$value'"
        exit 2
    fi
}

section "sunscreen heavy integration runner"
echo "root=$ROOT"
echo "log=$LOG_FILE"
echo "SUNSCREEN_COMPILE_TESTS=$COMPILE_TESTS"
echo "SUNSCREEN_REAL_TOOLCHAIN=$REAL_TOOLCHAIN"
echo "SUNSCREEN_DIST=$DIST_TESTS"
echo "SUNSCREEN_FLAKE_RUNS=$FLAKE_RUNS"
echo "SUNSCREEN_PINOCCHIO_SBF=$PINOCCHIO_SBF"

section "offline deterministic gate"
CURRENT_TIER="offline_deterministic"
run cargo fmt --all -- --check
run cargo clippy --locked --all-targets --all-features -- -D warnings
run cargo check --locked --no-default-features --all-targets
run cargo test --locked --all --all-features --no-fail-fast
run cargo test --locked --test integration_chain --test integration_scaffold --test integration_generate --test integration_onboarding --test app_lifecycle
run cargo test --locked --test compile_generated_workspace
run cargo build --locked --release --all-features
record_tier "$CURRENT_TIER" "passed" "fmt, clippy, no-default check, cargo test --all, command-group smokes, compile_generated_workspace, release build"

section "release binary smoke"
CURRENT_TIER="release_binary_smoke"
run ./target/release/sunscreen --help
run ./target/release/sunscreen version
run_allow_exit "0 2" ./target/release/sunscreen doctor --json
run ./target/release/sunscreen app marketplace --json
record_tier "$CURRENT_TIER" "passed" "release binary help/version/doctor/app marketplace"

if [[ "$COMPILE_TESTS" == "1" ]]; then
    section "generated workspace compile gate"
    CURRENT_TIER="generated_workspace_compile"
    run_shell "SUNSCREEN_COMPILE_TESTS=1 cargo test --locked --test compile_generated -- --nocapture"
    record_tier "$CURRENT_TIER" "passed" "SUNSCREEN_COMPILE_TESTS=1 compile_generated executed"
else
    section "generated workspace compile gate skipped"
    echo "Set SUNSCREEN_COMPILE_TESTS=1 to run compile_generated."
    record_tier "generated_workspace_compile" "skipped" "set SUNSCREEN_COMPILE_TESTS=1"
fi

if [[ "$REAL_TOOLCHAIN" == "1" ]]; then
    section "real Solana toolchain probes"
    CURRENT_TIER="real_anchor_codama"
    require_tool cargo
    require_tool rustc
    require_tool anchor
    require_tool solana
    require_tool solana-test-validator
    require_tool pnpm
    require_tool node
    require_tool codama

    run cargo --version
    run rustc --version
    run anchor --version
    run solana --version
    run solana-test-validator --version
    run pnpm --version
    run node --version
    run_shell "command -v codama"

    section "real Anchor/Codama integration gate"
    run cargo test --locked --test integration_anchor -- --ignored --nocapture
    record_tier "$CURRENT_TIER" "passed" "real toolchain probes passed and integration_anchor --ignored executed"
else
    section "real Solana toolchain gate skipped"
    echo "Set SUNSCREEN_REAL_TOOLCHAIN=1 to require real anchor/solana/solana-test-validator/pnpm/node/codama and run integration_anchor --ignored."
    record_tier "real_anchor_codama" "skipped" "set SUNSCREEN_REAL_TOOLCHAIN=1"
fi

if [[ "$PINOCCHIO_SBF" == "1" ]]; then
    section "real Pinocchio SBF gate"
    CURRENT_TIER="pinocchio_sbf"
    require_tool cargo
    require_tool rustc
    require_tool solana
    pin_tmp="$(mktemp -d)"
    run ./target/release/sunscreen chain new real_pin --framework pinocchio --frontend none --path "$pin_tmp/real_pin"
    run_shell "cd '$pin_tmp/real_pin' && '$ROOT/target/release/sunscreen' --json chain build --headless"
    record_tier "$CURRENT_TIER" "passed" "Pinocchio workspace built through real chain build --headless"
else
    section "real Pinocchio SBF gate skipped"
    echo "Set SUNSCREEN_PINOCCHIO_SBF=1 to require Solana/Cargo SBF and run a real Pinocchio chain build."
    record_tier "pinocchio_sbf" "skipped" "set SUNSCREEN_PINOCCHIO_SBF=1"
fi

if [[ "$DIST_TESTS" == "1" ]]; then
    section "cargo-dist release gate"
    CURRENT_TIER="cargo_dist"
    require_tool cargo
    if ! cargo dist --version >/dev/null 2>&1; then
        echo "missing required cargo subcommand for dist validation: cargo-dist"
        echo "Install cargo-dist or run without SUNSCREEN_DIST=1."
        record_tier "$CURRENT_TIER" "blocked" "missing cargo-dist"
        exit 2
    fi
    run cargo dist --version
    run cargo dist plan
    record_tier "$CURRENT_TIER" "passed" "cargo dist plan executed"
else
    section "cargo-dist release gate skipped"
    echo "Set SUNSCREEN_DIST=1 to require cargo-dist and run cargo dist plan."
    record_tier "cargo_dist" "skipped" "set SUNSCREEN_DIST=1"
fi

require_number "SUNSCREEN_FLAKE_RUNS" "$FLAKE_RUNS"
if (( FLAKE_RUNS > 0 )); then
    section "flake loop"
    CURRENT_TIER="flake_loop"
    for i in $(seq 1 "$FLAKE_RUNS"); do
        echo "flake iteration $i/$FLAKE_RUNS"
        run cargo test --locked --test integration_chain --test integration_scaffold --test integration_generate --test integration_onboarding --test app_lifecycle
    done
    record_tier "$CURRENT_TIER" "passed" "command-group smoke repeated $FLAKE_RUNS times"
else
    section "flake loop skipped"
    echo "Set SUNSCREEN_FLAKE_RUNS=N to repeat CLI smoke integration tests."
    record_tier "flake_loop" "skipped" "set SUNSCREEN_FLAKE_RUNS=N"
fi

section "complete"
echo "heavy integration log: $LOG_FILE"
