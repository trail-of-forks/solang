//! This test expects you to have a devnode running:
//! <https://docs.arbitrum.io/run-arbitrum-node/run-nitro-dev-node>
//!
//! It also expects `cargo-stylus` and `cast` to be installed:
//! - <https://github.com/OffchainLabs/cargo-stylus>
//! - <https://book.getfoundry.sh/cast/>
#![warn(clippy::pedantic)]

use crate::{call, deploy, send, MUTEX};
use std::path::PathBuf;

#[test]
fn erc20() {
    let _lock = MUTEX.lock();
    let (tempdir, address) = deploy(
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("integration/stylus/Uniswap/test/ERC20.sol"),
        "ERC20",
        true,
    )
    .unwrap();
    let dir = &tempdir;

    let balance = concat!("1000000", "000000000000000000");

    let stdout = call(dir, &address, ["totalSupply()(uint256)"]).unwrap();
    println!("{}", stdout);
    assert_eq!("0\n", stdout);

    let stdout = send(dir, &address, ["initialize(uint256)", balance]).unwrap();
    println!("{}", stdout);

    // smoelius: Calling `init` a second time should revert.
    let error = send(dir, &address, ["initialize(uint256)", balance]).unwrap_err();
    println!("{:?}", error);

    let stdout = call(dir, &address, ["totalSupply()(uint256)"]).unwrap();
    println!("{}", stdout);
    assert_eq!("1000000000000000000000000 [1e24]\n", stdout);
}
