// SPDX-License-Identifier: UNLICENSED
pragma solidity >=0.8.0;

interface ArbWasm {
    /// @notice Activate a wasm program
    /// @param program the program to activate
    /// @return version the stylus version the program was activated against
    /// @return dataFee the data fee paid to store the activated program
    function activateProgram(
        address program
    ) external payable returns (uint16 version, uint256 dataFee);
}

contract C {
    function test_create2() public payable {
        Greeter greeter = new Greeter();
        print("greeter = {}".format(address(greeter)));

        ArbWasm arbWasm = ArbWasm(address(0x71));
        (uint16 version, uint256 dataFee) = arbWasm.activateProgram{
            value: msg.value
        }(address(greeter));
        print("version = {}".format(version));
        print("dataFee = {}".format(dataFee));

        greeter.greet();

        bytes initCode = contractDeploymentCalldata(address(greeter).code);
        print("initCode = {}".format(initCode));

        Greeter greeter0;
        assembly {
            greeter0 := create2(
                0,
                initCode,
                0, // codeLen - ignored
                0 // salt
            )
        }
        print("greeter0 = {}".format(address(greeter0)));
        greeter0.greet();

        Greeter greeter1;
        assembly {
            greeter1 := create2(
                0,
                initCode,
                0, // codeLen - ignored
                1 // salt
            )
        }
        print("greeter1 = {}".format(address(greeter1)));
        greeter1.greet();
    }
}

contract Greeter {
    function greet() public view {
        print("Greetings from 0x{}!".format(this));
    }
}

function contractDeploymentCalldata(bytes code) pure returns (bytes) {
    bytes memory deploy;
    uint256 codeLen = code.length;
    deploy.push(0x7f); // PUSH32
    deploy = pushBytes(deploy, bytes32(codeLen));
    deploy.push(0x80); // DUP1
    deploy.push(0x60); // PUSH1
    deploy.push(43); // prelude + version
    deploy.push(0x60); // PUSH1
    deploy.push(0x00);
    deploy.push(0x39); // CODECOPY
    deploy.push(0x60); // PUSH1
    deploy.push(0x00);
    deploy.push(0xf3); // RETURN
    deploy.push(0x00); // version
    deploy = pushBytes(deploy, code);
    return deploy;
}

function pushBytes(bytes memory xs, bytes memory ys) pure returns (bytes) {
    uint256 n = ys.length;
    for (uint256 i = 0; i < n; i++) {
        xs.push(ys[i]);
    }
    return xs;
}
