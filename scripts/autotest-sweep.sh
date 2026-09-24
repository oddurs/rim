#!/usr/bin/env sh
# Run the client autotest over many maps (the autotest itself always uses
# seed 7, so CI is stable). Usage: scripts/autotest-sweep.sh [first] [last]
# Prints one line per seed and exits non-zero if any failed.
set -u
first=${1:-1}
last=${2:-44}
bin="${CARGO_TARGET_DIR:-target}/release/rim"
[ -x "$bin" ] || cargo build --release -p rim_client
out=$(mktemp -d)
failed=""
s=$first
while [ "$s" -le "$last" ]; do
    if "$bin" --seed "$s" --autotest "$out/$s" > "$out/$s.log" 2>&1; then
        echo "seed $s: ok"
    else
        echo "seed $s: FAILED"
        grep "FAIL " "$out/$s.log" | sed 's/^/    /'
        failed="$failed $s"
    fi
    s=$((s + 1))
done
if [ -n "$failed" ]; then
    echo "failed seeds:$failed (logs and screenshots in $out)"
    exit 1
fi
echo "all seeds $first-$last pass"
