// SPDX-License-Identifier: UNLICENSED
pragma solidity >=0.8.0;

// smoelius: The hash and signature were generated with the default private and public key at:
// https://8gwifi.org/ecsignverify.jsp
//
// The message was the four byte string "1234".
//
// For completeness, here are the private and public keys:
//
// -----BEGIN EC PRIVATE KEY-----
// MHQCAQEEIJF1nrba+yqP4mT1I1B3ov1/Mx2ZGT7hLJJaeutk6+lxoAcGBSuBBAAK
// oUQDQgAEQb1xJnh9HhX7pYbh9pwAVcVEPlhHRwxkAnI8db53xOV39WHzidZ3n9+o
// yIfQIPWBkAbzwTcB0Ntj+Q4XrcPc5A==
// -----END EC PRIVATE KEY-----
//
// -----BEGIN PUBLIC KEY-----
// MFYwEAYHKoZIzj0CAQYFK4EEAAoDQgAEQb1xJnh9HhX7pYbh9pwAVcVEPlhHRwxk
// AnI8db53xOV39WHzidZ3n9+oyIfQIPWBkAbzwTcB0Ntj+Q4XrcPc5A==
// -----END PUBLIC KEY-----

contract C {
    function test_ecrecover() public pure {
        bytes32 hash = 0x03ac674216f3e15c761ee1a5e255f067953623c8b388b4459e13f978d7c846f4;
        bytes32 r = 0x46a24eba93079bc1442a5f0d6aa74025305faf5e04f58f35ae4961b5b268aef5;
        bytes32 s = 0x23bf6d6a32fd70b66e1ff51edfd511cc018e8e7f4c8b3ddd4082c597b458a73c;
        bytes1 v = 27;

        address actual = ecrecover(hash, v, r, s);

        print("actual = {}".format(actual));

        assert(address(0x6913bA9D8b921aa6702ecB2AEE31Bf41747d84f9) == actual);
    }
}
