#! /bin/bash

set -euo pipefail

for X in UniswapV2ERC20.sol UniswapV2Factory.sol UniswapV2Pair.sol; do
  ../../../target/debug/solang compile --target stylus "$X"
done
