Stylus
======

The flavor of Solidity that Solang supports for Stylus tries to be compatible with conventional Solidity as much as possible.
There are a few crucial differences, however, described below.

Programs must be activated
__________________________

In conventional Solidity, one can deploy a contract ``Foo`` and call ``bar`` on it as so:

.. code-block:: solidity

    (new Foo).bar()

But in Stylus, programs must be activated before methods on them can be called.
The following is an example of contract activation:

.. code-block:: solidity

    Foo foo = new Foo();

    ArbWasm arbWasm = ArbWasm(address(0x71));
    (uint16 version, uint256 dataFee) = arbWasm.activateProgram{
        value: msg.value
    }(address(foo));

    foo.bar();

In the above `ArbWasm <https://docs.arbitrum.io/build-decentralized-apps/precompiles/reference#arbwasm>`_ is the precompile at address 0x71.
It has a Solidity interface with the same name.

To activate a program, one should only need the ``activateProgram`` function.
This function takes the address of a contract to be activated and returns two values.
The first is the Stylus version the program was activated against; the second is the data fee paid to store the activated program.

Note that if the program was already activated, the call to ``activateProgram`` will revert.
Thus, one should structure their program to account for this possibility.

Constructors
____________

For the reason just given, constructors do not work in Stylus like they do in conventional Solidity.
That is, one cannot simply create a contract and expect its constructor to be called.
Instead, one must create the contract, activate it, and then call a function to simulate the contract's construction.

The following is an example. In conventional Solidity, the following would be a perfectly reasonable program:

.. code-block:: solidity

    contract C {
        uint256 x;

        constructor(uint256 _x) {
            x = _x;
        }

        function get_the_number() public view returns (uint256) {
            return x;
        }
    }

But in Stylus, one would have to write the program something like this:

.. code-block:: solidity

    contract C {
        bool initialized;
        uint256 x;

        function initialize(uint256 _x) public {
            require(!initialized);
            x = _x;
            initialized = true;
        }

        function get_the_number() public view returns (uint256) {
            return x;
        }
    }

WASM files are size limited
___________________________

In Arbitrum Stylus, the size of a Brotli-compressed WASM file must `not exceed 24KB <https://docs.arbitrum.io/stylus/how-tos/optimizing-binaries>`_.
This can be inhibiting.
For example, one cannot simply compile and deploy the ``UniswapV2Factory`` contract because it would exceed this size limit.

The bulk of the ``UniswapV2Factory``'s size comes from the ``UniswapV2Pair`` contract.
So one way to reduce the ``UniswapV2Factory``'s size is to move the logic to create ``UniswapV2Pair``s into another contract.
The following code snippet shows one way this can be done.
Note that the new ``createPair`` function uses modern Solidity syntax, rather than assembly, to pass a salt to ``new``.

.. code-block:: solidity

    interface IUniswapV2PairCreator {
        ...
        function createPair(bytes32 salt) external returns (address pair);
    }

    import './interfaces/IUniswapV2PairCreator.sol';
    import './UniswapV2Pair.sol';

    contract UniswapV2PairCreator is IUniswapV2PairCreator {
        ...
        function createPair(bytes32 salt) external returns (address pair) {
            UniswapV2Pair pair = new UniswapV2Pair{salt: salt}();
            pair.setFactoryAndBase(msg.sender, base);
            return address(pair);
        }
    }

    ...
    // In `UniswapV2Factory`, remove all references to the `UniswapV2Pair` contract and change the creation code as follows.
    bytes32 salt = keccak256(abi.encodePacked(token0, token1));
    // assembly {
    //     pair := create2(0, add(bytecode, 32), mload(bytecode), salt)
    // }
    pair = UniswapV2PairCreator(pairCreator).createPair(salt);
    ...

``block.number``
________________

According to the Stylus docs, ``block.number`` is defined as:

    the block of the first non-Arbitrum ancestor chain

This can seem confusing at first.
For example, ``block.number`` can return the same value when read in two different L2 blocks.
Moreover, the value returned can look random because it is for the parent L1 chain, not the L2 chain on which the contract is running.

To get the block number of the L2 on which the contract is running, one can call:

.. code-block:: solidity

    ArbSys(address(0x64)).arbBlockNumber() 

In the above:

- `ArbSys <https://docs.arbitrum.io/build-decentralized-apps/precompiles/reference#arbsys>`_ is the interface of the ``ArbSys`` precompile
- 0x64 is the precompile's address
- ``arbBlockNumber`` is the function that returns the L2 block number

Dynamic byte array memory layout
________________________________

In Ethereum Solidity, dynamic byte arrays are laid out as a 32-byte length followed by the array's contents.
However, WASM Solang represents a dynamic byte array as:

- a 32-bit (not byte) length
- a second copy of the 32-bit (not byte) length
- the array's contents

Thus, code that was written for Ethereum Solidity and that relies on the Ethereum Solidity memory layout cannot be ported unchanged to WASM Solang.
The code will have to adjust for the differences in the array's memory layout.

See https://github.com/hyperledger-solang/solang/blob/45f01b471800e9d271eff4e9030897e306580ec8/stdlib/stdlib.h#L6 for more details.
