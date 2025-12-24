The files in this directory are a slight modification of those from:
https://github.com/Uniswap/v2-core/tree/master/contracts

Specifically:

1. Contracts `UniswapV2PairBase` and `UniswapV2PairCreator` were added. The purposes of these contracts is to get `UniswapV2Pair`'s WASM file to compress to under 24KB.
2. The assembly code was removed.
3. Dead code that resulted from 2 was removed or commented out.

Sam Moelius (2025-12-24)
