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
REAL_TOOLCHAIN="${SUNSCREEN_REAL_TOOLCHAIN:-0}"
COMPILE_TESTS="${SUNSCREEN_COMPILE_TESTS:-0}"
DIST_TESTS="${SUNSCREEN_DIST:-0}"
FLAKE_RUNS="${SUNSCREEN_FLAKE_RUNS:-0}"

mkdir -p "$LOG_DIR"
exec > >(tee -a "$LOG_FILE") 2>&1

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

section "offline deterministic gate"
run cargo fmt --all -- --check
run cargo clippy --locked --all-targets --all-features -- -D warnings
run cargo check --locked --no-default-features --all-targets
run cargo test --locked --all --all-features --no-fail-fast
run cargo test --locked --test integration_chain --test integration_scaffold --test integration_generate --test integration_onboarding --test app_lifecycle
run cargo test --locked --test compile_generated_workspace
run cargo build --locked --release --all-features

section "release binary smoke"
run ./target/release/sunscreen --help
run ./target/release/sunscreen version
run_allow_exit "0 2" ./target/release/sunscreen doctor --json
run ./target/release/sunscreen app marketplace --json

if [[ "$COMPILE_TESTS" == "1" ]]; then
    section "generated workspace compile gate"
    run_shell "SUNSCREEN_COMPILE_TESTS=1 cargo test --locked --test compile_generated -- --nocapture"
else
    section "generated workspace compile gate skipped"
    echo "Set SUNSCREEN_COMPILE_TESTS=1 to run compile_generated."
fi

if [[ "$REAL_TOOLCHAIN" == "1" ]]; then
    section "real Solana toolchain probes"
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
else
    section "real Solana toolchain gate skipped"
    echo "Set SUNSCREEN_REAL_TOOLCHAIN=1 to require real anchor/solana/solana-test-validator/pnpm/node/codama and run integration_anchor --ignored."
fi

if [[ "$DIST_TESTS" == "1" ]]; then
    section "cargo-dist release gate"
    require_tool cargo
    if ! cargo dist --version >/dev/null 2>&1; then
        echo "missing required cargo subcommand for dist validation: cargo-dist"
        echo "Install cargo-dist or run without SUNSCREEN_DIST=1."
        exit 2
    fi
    run cargo dist --version
    run cargo dist plan
else
    section "cargo-dist release gate skipped"
    echo "Set SUNSCREEN_DIST=1 to require cargo-dist and run cargo dist plan."
fi

require_number "SUNSCREEN_FLAKE_RUNS" "$FLAKE_RUNS"
if (( FLAKE_RUNS > 0 )); then
    section "flake loop"
    for i in $(seq 1 "$FLAKE_RUNS"); do
        echo "flake iteration $i/$FLAKE_RUNS"
        run cargo test --locked --test integration_chain --test integration_scaffold --test integration_generate --test integration_onboarding --test app_lifecycle
    done
else
    section "flake loop skipped"
    echo "Set SUNSCREEN_FLAKE_RUNS=N to repeat CLI smoke integration tests."
fi

section "complete"
echo "heavy integration log: $LOG_FILE"
