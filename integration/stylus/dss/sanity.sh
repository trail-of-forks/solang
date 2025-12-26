#! /bin/bash

set -euo pipefail

for X in *.sol; do
  ../../../target/debug/solang compile --target stylus "$X" -O=less --no-constant-folding
done

for X in *.wasm; do
  brotli -f -q 11 "$X"
done

ls -lrt *.br
