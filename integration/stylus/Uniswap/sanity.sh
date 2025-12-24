#! /bin/bash

set -euo pipefail

for X in *.sol; do
  ../../../target/debug/solang compile --target stylus "$X" -O=less --no-constant-folding
  brotli -f -q 11 "$(basename "$X" .sol).wasm"
done

ls -lrt *.br
