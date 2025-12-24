pragma solidity >=0.5.0;

interface IUniswapV2PairCreator {
    function createPairWithBase(
        address base,
        address tokenA,
        address tokenB,
        uint256 activate
    ) external payable returns (address pair);
}
