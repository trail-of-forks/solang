pragma solidity =0.5.16;

import './interfaces/IUniswapV2Factory.sol';
import './interfaces/IUniswapV2PairCreator.sol';
import './UniswapV2Pair.sol';

contract UniswapV2Factory is IUniswapV2Factory {
    address public feeTo;
    address public feeToSetter;
    address public pairBase;
    address public pairCreator;

    mapping(address => mapping(address => address)) public getPair;
    address[] public allPairs;

    event PairCreated(address indexed token0, address indexed token1, address pair, uint);

    constructor(address _feeToSetter) public {
        // feeToSetter = _feeToSetter;
    }

    function initialize(
        address _feeToSetter,
        address _pairBase,
        address _pairCreator
    ) public {
        require(feeToSetter == address(0), "already initialized");
        feeToSetter = _feeToSetter;
        pairBase = _pairBase;
        pairCreator = _pairCreator;
    }

    function allPairsLength() external view returns (uint) {
        return allPairs.length;
    }

    function createPair(address tokenA, address tokenB) external returns (address pair) {
        require(getPair[tokenA][tokenB] == address(0), 'UniswapV2: PAIR_EXISTS');
        require(getPair[tokenB][tokenA] == address(0), 'UniswapV2: PAIR_EXISTS');
        // bytes memory bytecode = type(UniswapV2Pair).creationCode;
        // bytes32 salt = keccak256(abi.encodePacked(token0, token1));
        // assembly {
        //     pair := create2(0, add(bytecode, 32), mload(bytecode), salt)
        // }
        pair = IUniswapV2PairCreator(pairCreator).createPairWithBase(
            pairBase,
            tokenA,
            tokenB,
            0
        );
        getPair[tokenA][tokenB] = pair;
        getPair[tokenB][tokenA] = pair; // populate mapping in the reverse direction
        allPairs.push(pair);
        emit PairCreated(tokenA, tokenB, pair, allPairs.length);
    }

    function setFeeTo(address _feeTo) external {
        require(msg.sender == feeToSetter, 'UniswapV2: FORBIDDEN');
        feeTo = _feeTo;
    }

    function setFeeToSetter(address _feeToSetter) external {
        require(msg.sender == feeToSetter, 'UniswapV2: FORBIDDEN');
        feeToSetter = _feeToSetter;
    }
}
