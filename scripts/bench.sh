#!/usr/bin/env bash
# Cold-start benchmark for `sunscreen --help`.
# Reports mean and p95 across N runs. Non-blocking: always exits 0.

set -uo pipefail

BIN="${SUNSCREEN_BIN:-./target/release/sunscreen}"
RUNS="${RUNS:-10}"

if [[ ! -x "$BIN" ]]; then
    echo "bench: binary not found at $BIN; building..."
    cargo build --release >/dev/null
fi

if [[ ! -x "$BIN" ]]; then
    echo "bench: still no binary, aborting (non-blocking)."
    exit 0
fi

echo "bench: running '$BIN --help' x $RUNS"

samples=()
for i in $(seq 1 "$RUNS"); do
    # nanoseconds via python (portable across macOS/Linux without GNU date)
    t=$(python3 -c "
import subprocess, time
start = time.perf_counter_ns()
subprocess.run(['$BIN', '--help'], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
print(time.perf_counter_ns() - start)
")
    samples+=("$t")
    echo "  run $i: $((t / 1000000)) ms"
done

joined=$(IFS=,; echo "${samples[*]}")
python3 - <<EOF
samples = sorted([${joined}])
n = len(samples)
mean = sum(samples) / n / 1e6
p95 = samples[max(0, int(round(0.95 * n)) - 1)] / 1e6
mn = samples[0] / 1e6
mx = samples[-1] / 1e6
print(f"bench: mean={mean:.2f}ms p95={p95:.2f}ms min={mn:.2f}ms max={mx:.2f}ms (n={n})")
if p95 > 50:
    print(f"bench: WARN p95 {p95:.2f}ms exceeds 50ms target (non-blocking)")
EOF

exit 0
