#!/usr/bin/env sh
# Determinism guard: the Luau VM must not contain fused multiply-add
# instructions. A fused a*b+c is rounded once, an unfused one twice, so the
# same script could give different numbers on an arm64 Mac and an x86 PC and
# desync lockstep. .cargo/config.toml builds Luau with -ffp-contract=off; this
# checks the result. Run after a release build.
#
# Allowed: math_noise's `fmod(x, 256.0)`, which clang expands to
# x - trunc(x/256)*256 with one fmadd. Multiplying by 256 is exact, so fused
# and unfused give the same result.
set -eu
dir="${CARGO_TARGET_DIR:-target}/release/build"
# The newest build only: stale ones from older flags or versions linger in
# target/ and in CI caches.
libs=$(find -L "$dir" -name libluauvm.a -exec ls -t {} + 2>/dev/null | head -n 1)
if [ -z "$libs" ]; then
    echo "check-no-fma: no libluauvm.a under $dir (build --release first)" >&2
    exit 1
fi
status=0
for lib in $libs; do
    hits=$(objdump -d "$lib" | awk '
        /^[0-9a-f]+ <.*>:$/ { fn = $2 }
        /[ \t](fn?m(add|sub)|vfn?m(add|sub)[0-9a-z]*)[ \t]/ { print fn }
    ' | sort | uniq -c | grep -v "math_noise" || true)
    if [ -n "$hits" ]; then
        echo "check-no-fma: fused multiply-add in $lib:" >&2
        echo "$hits" >&2
        status=1
    else
        echo "check-no-fma: $lib is clean"
    fi
done
exit $status
