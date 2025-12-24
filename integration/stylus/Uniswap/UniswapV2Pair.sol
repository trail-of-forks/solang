pragma solidity =0.5.16;

import './interfaces/IUniswapV2Pair.sol';
import './UniswapV2ERC20.sol';
import './libraries/Math.sol';
import './libraries/UQ112x112.sol';
import './interfaces/IERC20.sol';
import './interfaces/IUniswapV2Factory.sol';
import './interfaces/IUniswapV2Callee.sol';

contract UniswapV2Pair is IUniswapV2Pair, UniswapV2ERC20 {
    using SafeMath  for uint;
    using UQ112x112 for uint224;

    uint public constant MINIMUM_LIQUIDITY = 10**3;
    bytes4 private constant SELECTOR = bytes4(keccak256(bytes('transfer(address,uint256)')));

    address public factory;
    address public base;
    address public token0;
    address public token1;

    uint112 private reserve0;           // uses single storage slot, accessible via getReserves
    uint112 private reserve1;           // uses single storage slot, accessible via getReserves
    uint32  private blockTimestampLast; // uses single storage slot, accessible via getReserves

    uint public price0CumulativeLast;
    uint public price1CumulativeLast;
    uint public kLast; // reserve0 * reserve1, as of immediately after the most recent liquidity event

    uint private unlocked = 1;
    modifier lock() {
        require(unlocked == 1, 'UniswapV2: LOCKED');
        unlocked = 0;
        _;
        unlocked = 1;
    }

    function getReserves() public view returns (uint112 _reserve0, uint112 _reserve1, uint32 _blockTimestampLast) {
        _reserve0 = reserve0;
        _reserve1 = reserve1;
        _blockTimestampLast = blockTimestampLast;
    }

    function _safeTransfer(address token, address to, uint value) private {
        (bool success, bytes memory data) = token.call(abi.encodeWithSelector(SELECTOR, to, value));
        require(success && (data.length == 0 || abi.decode(data, (bool))), 'UniswapV2: TRANSFER_FAILED');
    }

    // event Mint(address indexed sender, uint amount0, uint amount1);
    // event Burn(address indexed sender, uint amount0, uint amount1, address indexed to);
    // event Swap(
    //     address indexed sender,
    //     uint amount0In,
    //     uint amount1In,
    //     uint amount0Out,
    //     uint amount1Out,
    //     address indexed to
    // );
    // event Sync(uint112 reserve0, uint112 reserve1);

    constructor() public {
        // factory = msg.sender;
    }

    function setFactoryAndBase(address _factory, address _base) external {
        require(factory == address(0), "factory already set");
        require(base == address(0), "base already set");
        factory = _factory;
        base = _base;
        bytes memory args = abi.encodeWithSignature(
            "setFactoryAndBase(address,address)",
            _factory,
            _base
        );
        (, bytes memory result) = base.delegatecall(args);
        return abi.decode(result, ());
    }

    // called once by the factory at time of deployment
    function initialize(address _token0, address _token1) external {
        // require(msg.sender == factory, 'UniswapV2: FORBIDDEN'); // sufficient check
        token0 = _token0;
        token1 = _token1;
        bytes memory args = abi.encodeWithSignature(
            "initialize(address,address)",
            _token0,
            _token1
        );
        (, bytes memory result) = base.delegatecall(args);
        return abi.decode(result, ());
    }

    function mint(address to) external returns (uint liquidity) {
        bytes memory args = abi.encodeWithSignature("mint(address)", to);
        (, bytes memory result) = base.delegatecall(args);
        return abi.decode(result, (uint));
    }

    function burn(address to) external returns (uint amount0, uint amount1) {
        bytes memory args = abi.encodeWithSignature("burn(address)", to);
        (, bytes memory result) = base.delegatecall(args);
        return abi.decode(result, (uint, uint));
    }

    function swap(
        uint amount0Out,
        uint amount1Out,
        address to,
        bytes calldata data
    ) external {
        bytes memory args = abi.encodeWithSignature(
            "swap(uint256,uint256,address,bytes)",
            amount0Out,
            amount1Out,
            to,
            data
        );
        (, bytes memory result) = base.delegatecall(args);
        return abi.decode(result, ());
    }

    function skim(address to) external {
        bytes memory args = abi.encodeWithSignature("skim(address)", to);
        (, bytes memory result) = base.delegatecall(args);
        return abi.decode(result, ());
    }

    function sync() external {
        bytes memory args = abi.encodeWithSignature("sync()");
        (, bytes memory result) = base.delegatecall(args);
        return abi.decode(result, ());
    }
}
