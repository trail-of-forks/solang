//! This test expects you to have a devnode running:
//! <https://docs.arbitrum.io/run-arbitrum-node/run-nitro-dev-node>
//!
//! It also expects `cargo-stylus` and `cast` to be installed:
//! - <https://github.com/OffchainLabs/cargo-stylus>
//! - <https://book.getfoundry.sh/cast/>
#![warn(clippy::pedantic)]

use crate::{call, deploy, MUTEX};
use std::path::PathBuf;

#[test]
fn ecrecover() {
    let _lock = MUTEX.lock();
    let (tempdir, address) = deploy(
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("integration/stylus/ecrecover.sol"),
        "C",
        true,
    )
    .unwrap();
    let dir = &tempdir;

    let stdout = call(dir, &address, ["test_ecrecover()"]).unwrap();
    println!("{stdout}");
}
