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
FRONTEND_TESTS="${SUNSCREEN_FRONTEND_COMPILE_TESTS:-0}"
DIST_TESTS="${SUNSCREEN_DIST:-0}"
FLAKE_RUNS="${SUNSCREEN_FLAKE_RUNS:-0}"
PINOCCHIO_SBF="${SUNSCREEN_PINOCCHIO_SBF:-0}"
CURRENT_TIER=""
CURRENT_OWNER=""
CURRENT_COMMAND=""

mkdir -p "$LOG_DIR"
: > "$SUMMARY_TSV"
exec > >(tee -a "$LOG_FILE") 2>&1

record_tier() {
    local tier="$1"
    local status="$2"
    local owner="$3"
    local command="$4"
    local evidence="${5:-}"
    local next_action="${6:-}"
    printf '%s\t%s\t%s\t%s\t%s\t%s\n' "$tier" "$status" "$owner" "$command" "$evidence" "$next_action" >> "$SUMMARY_TSV"
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
            parts = raw.split("\t", 5)
            while len(parts) < 6:
                parts.append("")
            tiers.append(
                {
                    "tier": parts[0],
                    "status": parts[1],
                    "owner": parts[2],
                    "command": parts[3],
                    "evidence": parts[4],
                    "next_action": parts[5],
                }
            )
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
            record_tier "$CURRENT_TIER" "blocked" "$CURRENT_OWNER" "$CURRENT_COMMAND" "missing required tool: $tool" "install/provision $tool or rerun without this tier"
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
echo "SUNSCREEN_FRONTEND_COMPILE_TESTS=$FRONTEND_TESTS"
echo "SUNSCREEN_REAL_TOOLCHAIN=$REAL_TOOLCHAIN"
echo "SUNSCREEN_DIST=$DIST_TESTS"
echo "SUNSCREEN_FLAKE_RUNS=$FLAKE_RUNS"
echo "SUNSCREEN_PINOCCHIO_SBF=$PINOCCHIO_SBF"

section "offline deterministic gate"
CURRENT_TIER="offline_deterministic"
CURRENT_OWNER="offline-ci-owner"
CURRENT_COMMAND="bash scripts/integration-heavy.sh"
run cargo fmt --all -- --check
run cargo clippy --locked --all-targets --all-features -- -D warnings
run cargo check --locked --no-default-features --all-targets
run cargo test --locked --all --all-features --no-fail-fast
run cargo test --locked --test integration_chain --test integration_scaffold --test integration_generate --test integration_onboarding --test app_lifecycle
run cargo test --locked --test compile_generated_workspace
run cargo build --locked --release --all-features
record_tier "$CURRENT_TIER" "passed" "$CURRENT_OWNER" "$CURRENT_COMMAND" "fmt, clippy, no-default check, cargo test --all, command-group smokes, compile_generated_workspace, release build" ""

section "release binary smoke"
CURRENT_TIER="release_binary_smoke"
CURRENT_OWNER="release-distribution-qa"
CURRENT_COMMAND="./target/release/sunscreen --help; version; doctor --json; app marketplace --json"
run ./target/release/sunscreen --help
run ./target/release/sunscreen version
run_allow_exit "0 2" ./target/release/sunscreen doctor --json
run ./target/release/sunscreen app marketplace --json
record_tier "$CURRENT_TIER" "passed" "$CURRENT_OWNER" "$CURRENT_COMMAND" "release binary help/version/doctor/app marketplace" ""
record_tier "plugin_runtime" "passed" "plugin-runtime-qa" "cargo test --test app_lifecycle; ./target/release/sunscreen app marketplace --json" "app lifecycle ran in offline gate and marketplace JSON executed" ""

if [[ "$COMPILE_TESTS" == "1" ]]; then
    section "generated workspace compile gate"
    CURRENT_TIER="generated_workspace_compile"
    CURRENT_OWNER="real-anchor-codama-owner"
    CURRENT_COMMAND="SUNSCREEN_COMPILE_TESTS=1 cargo test --locked --test compile_generated -- --nocapture"
    run_shell "SUNSCREEN_COMPILE_TESTS=1 cargo test --locked --test compile_generated -- --nocapture"
    record_tier "$CURRENT_TIER" "passed" "$CURRENT_OWNER" "$CURRENT_COMMAND" "SUNSCREEN_COMPILE_TESTS=1 compile_generated executed" ""
else
    section "generated workspace compile gate skipped"
    echo "Set SUNSCREEN_COMPILE_TESTS=1 to run compile_generated."
    record_tier "generated_workspace_compile" "skipped" "real-anchor-codama-owner" "SUNSCREEN_COMPILE_TESTS=1 bash scripts/integration-heavy.sh" "compile_generated gated suite not requested" "rerun with SUNSCREEN_COMPILE_TESTS=1"
fi

if [[ "$REAL_TOOLCHAIN" == "1" ]]; then
    section "real Solana toolchain probes"
    CURRENT_TIER="real_anchor_codama"
    CURRENT_OWNER="real-anchor-codama-owner"
    CURRENT_COMMAND="SUNSCREEN_REAL_TOOLCHAIN=1 bash scripts/integration-heavy.sh"
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
    record_tier "$CURRENT_TIER" "passed" "$CURRENT_OWNER" "$CURRENT_COMMAND" "real toolchain probes passed and integration_anchor --ignored executed" ""
else
    section "real Solana toolchain gate skipped"
    echo "Set SUNSCREEN_REAL_TOOLCHAIN=1 to require real anchor/solana/solana-test-validator/pnpm/node/codama and run integration_anchor --ignored."
    record_tier "real_anchor_codama" "skipped" "real-anchor-codama-owner" "SUNSCREEN_REAL_TOOLCHAIN=1 bash scripts/integration-heavy.sh" "real Anchor/Codama tier not requested" "rerun with SUNSCREEN_REAL_TOOLCHAIN=1 on a provisioned machine"
fi

if [[ "$PINOCCHIO_SBF" == "1" ]]; then
    section "real Pinocchio SBF gate"
    CURRENT_TIER="pinocchio_sbf"
    CURRENT_OWNER="pinocchio-sbf-owner"
    CURRENT_COMMAND="SUNSCREEN_PINOCCHIO_SBF=1 bash scripts/integration-heavy.sh"
    require_tool cargo
    require_tool rustc
    require_tool solana
    pin_tmp="$(mktemp -d)"
    run ./target/release/sunscreen chain new real_pin --framework pinocchio --frontend none --path "$pin_tmp/real_pin"
    run_shell "cd '$pin_tmp/real_pin' && '$ROOT/target/release/sunscreen' --json chain build --headless"
    record_tier "$CURRENT_TIER" "passed" "$CURRENT_OWNER" "$CURRENT_COMMAND" "Pinocchio workspace built through real chain build --headless" ""
else
    section "real Pinocchio SBF gate skipped"
    echo "Set SUNSCREEN_PINOCCHIO_SBF=1 to require Solana/Cargo SBF and run a real Pinocchio chain build."
    record_tier "pinocchio_sbf" "skipped" "pinocchio-sbf-owner" "SUNSCREEN_PINOCCHIO_SBF=1 bash scripts/integration-heavy.sh" "Pinocchio SBF tier not requested" "rerun with SUNSCREEN_PINOCCHIO_SBF=1 on a Solana SBF machine"
fi

if [[ "$FRONTEND_TESTS" == "1" ]]; then
    section "frontend codegen typecheck gate"
    CURRENT_TIER="frontend_codegen"
    CURRENT_OWNER="frontend-codegen-owner"
    CURRENT_COMMAND="SUNSCREEN_FRONTEND_COMPILE_TESTS=1 cargo test --locked --test generate generated_frontend_hooks_typecheck_vanilla_next_project_when_dependencies_are_installed -- --ignored --nocapture"
    require_tool node
    require_tool pnpm
    run_shell "SUNSCREEN_FRONTEND_COMPILE_TESTS=1 cargo test --locked --test generate generated_frontend_hooks_typecheck_vanilla_next_project_when_dependencies_are_installed -- --ignored --nocapture"
    record_tier "$CURRENT_TIER" "passed" "$CURRENT_OWNER" "$CURRENT_COMMAND" "generated frontend hooks typecheck executed" ""
else
    section "frontend codegen typecheck gate skipped"
    echo "Set SUNSCREEN_FRONTEND_COMPILE_TESTS=1 to run generated frontend hook typecheck."
    record_tier "frontend_codegen" "skipped" "frontend-codegen-owner" "SUNSCREEN_FRONTEND_COMPILE_TESTS=1 bash scripts/integration-heavy.sh" "frontend typecheck tier not requested" "rerun with SUNSCREEN_FRONTEND_COMPILE_TESTS=1 on a Node/pnpm machine"
fi

if [[ "$DIST_TESTS" == "1" ]]; then
    section "cargo-dist release gate"
    CURRENT_TIER="cargo_dist"
    CURRENT_OWNER="release-distribution-qa"
    CURRENT_COMMAND="SUNSCREEN_DIST=1 bash scripts/integration-heavy.sh"
    require_tool cargo
    if ! cargo dist --version >/dev/null 2>&1; then
        echo "missing required cargo subcommand for dist validation: cargo-dist"
        echo "Install cargo-dist or run without SUNSCREEN_DIST=1."
        record_tier "$CURRENT_TIER" "blocked" "$CURRENT_OWNER" "$CURRENT_COMMAND" "missing cargo-dist" "install cargo-dist 0.22.1 or run without SUNSCREEN_DIST=1"
        exit 2
    fi
    run cargo dist --version
    run cargo dist plan
    record_tier "$CURRENT_TIER" "passed" "$CURRENT_OWNER" "$CURRENT_COMMAND" "cargo dist plan executed" ""
else
    section "cargo-dist release gate skipped"
    echo "Set SUNSCREEN_DIST=1 to require cargo-dist and run cargo dist plan."
    record_tier "cargo_dist" "skipped" "release-distribution-qa" "SUNSCREEN_DIST=1 bash scripts/integration-heavy.sh" "cargo-dist tier not requested" "rerun with SUNSCREEN_DIST=1 and cargo-dist installed"
fi

require_number "SUNSCREEN_FLAKE_RUNS" "$FLAKE_RUNS"
if (( FLAKE_RUNS > 0 )); then
    section "flake loop"
    CURRENT_TIER="flake_loop"
    CURRENT_OWNER="flake-perf-auditor"
    CURRENT_COMMAND="SUNSCREEN_FLAKE_RUNS=$FLAKE_RUNS bash scripts/integration-heavy.sh"
    for i in $(seq 1 "$FLAKE_RUNS"); do
        echo "flake iteration $i/$FLAKE_RUNS"
        run cargo test --locked --test integration_chain --test integration_scaffold --test integration_generate --test integration_onboarding --test app_lifecycle
    done
    record_tier "$CURRENT_TIER" "passed" "$CURRENT_OWNER" "$CURRENT_COMMAND" "command-group smoke repeated $FLAKE_RUNS times" ""
else
    section "flake loop skipped"
    echo "Set SUNSCREEN_FLAKE_RUNS=N to repeat CLI smoke integration tests."
    record_tier "flake_loop" "skipped" "flake-perf-auditor" "SUNSCREEN_FLAKE_RUNS=N bash scripts/integration-heavy.sh" "flake loop not requested" "rerun with SUNSCREEN_FLAKE_RUNS=N"
fi

section "complete"
echo "heavy integration log: $LOG_FILE"
