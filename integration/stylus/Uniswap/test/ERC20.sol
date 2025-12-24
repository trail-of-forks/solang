pragma solidity =0.5.16;

import "../UniswapV2ERC20.sol";

contract ERC20 is UniswapV2ERC20 {
    bool initialized;

    function initialize(uint _totalSupply) public {
        require(!initialized);
        _mint(msg.sender, _totalSupply);
        initialized = true;
    }
}
