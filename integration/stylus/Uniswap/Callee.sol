pragma solidity =0.5.16;

import "./interfaces/IERC20.sol";
import "./interfaces/IUniswapV2Callee.sol";

contract Callee is IUniswapV2Callee {
    address token;

    function initialize(address _token) external {
        token = _token;
    }

    function uniswapV2Call(
        address,
        uint,
        uint,
        bytes calldata
    ) external {
        uint256 balance = IERC20(token).balanceOf(address(this));
        IERC20(token).transfer(msg.sender, balance);
    }
}
