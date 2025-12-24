pragma solidity =0.5.16;

import './interfaces/IUniswapV2PairCreator.sol';
import './UniswapV2Pair.sol';

interface ArbWasm {
    /// @notice Activate a wasm program
    /// @param program the program to activate
    /// @return version the stylus version the program was activated against
    /// @return dataFee the data fee paid to store the activated program
    function activateProgram(
        address program
    ) external payable returns (uint16 version, uint256 dataFee);
}

contract UniswapV2PairCreator is IUniswapV2PairCreator {
    function createPairWithBase(
        address base,
        address tokenA,
        address tokenB,
        uint256 activate
    ) external payable returns (address) {
        require(tokenA != tokenB, "UniswapV2: IDENTICAL_ADDRESSES");
        (address token0, address token1) = tokenA < tokenB
            ? (tokenA, tokenB)
            : (tokenB, tokenA);
        require(token0 != address(0), "UniswapV2: ZERO_ADDRESS");
        // bytes memory bytecode = type(UniswapV2Pair).creationCode;
        bytes32 salt = keccak256(abi.encodePacked(token0, token1));
        // assembly {
        //     pair := create2(0, add(bytecode, 32), mload(bytecode), salt)
        // }
        UniswapV2Pair pair = new UniswapV2Pair{salt: salt}();

        if (activate != 0) {
            ArbWasm arbWasm = ArbWasm(address(0x71));
            (uint16 version, uint256 dataFee) = arbWasm.activateProgram{
                value: msg.value
            }(address(pair));
        }

        pair.setFactoryAndBase(msg.sender, base);
        pair.initialize(token0, token1);
        return address(pair);
    }
}
