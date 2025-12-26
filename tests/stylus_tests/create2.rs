//! This test expects you to have a devnode running:
//! <https://docs.arbitrum.io/run-arbitrum-node/run-nitro-dev-node>
//!
//! It also expects `cargo-stylus` and `cast` to be installed:
//! - <https://github.com/OffchainLabs/cargo-stylus>
//! - <https://book.getfoundry.sh/cast/>
#![warn(clippy::pedantic)]

use crate::{deploy, send, MUTEX};
use std::{io::Write, path::PathBuf};

#[test]
fn create2() {
    writeln!(
        std::io::stderr(),
        "If you run the `create2` test twice, it will fail the second time because the \
         contract `Greeter` cannot be activated twice.",
    )
    .unwrap();

    let _lock = MUTEX.lock();
    let (tempdir, address) = deploy(
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("integration/stylus/create2.sol"),
        "C",
        true,
    )
    .unwrap();
    let dir = &tempdir;

    let stdout = send(
        dir,
        &address,
        ["test_create2()", "--value=1000000000000000000"],
    )
    .unwrap();
    println!("{stdout}");
}
