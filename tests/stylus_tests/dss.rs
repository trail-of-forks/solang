//! This test expects you to have a devnode running:
//! <https://docs.arbitrum.io/run-arbitrum-node/run-nitro-dev-node>
//!
//! It also expects `cargo-stylus` and `cast` to be installed:
//! - <https://github.com/OffchainLabs/cargo-stylus>
//! - <https://book.getfoundry.sh/cast/>
#![warn(clippy::pedantic)]

// smoelius: This test mimics the initial steps of the interaction described here:
// https://github.com/sky-ecosystem/sai?tab=readme-ov-file#sample-interaction-using-sai

use crate::{call, deploy, send, ADDRESS, MUTEX};
use std::path::{Path, PathBuf};
use tempfile::TempDir;

const CHAIN_ID: u64 = 42161;

const GOLD: &str = "0x676f6c6400000000000000000000000000000000000000000000000000000000"; // "gold"
const SPOT: &str = "0x73706f7400000000000000000000000000000000000000000000000000000000"; // "spot"
const LINE_LOWER: &str = "0x6c696e6500000000000000000000000000000000000000000000000000000000"; // "line"
const LINE_UPPER: &str = "0x4c696e6500000000000000000000000000000000000000000000000000000000"; // "Line"

#[test]
fn dss() {
    let _lock = MUTEX.lock();
    let (dirs, contracts) = deploy_contracts();
    let dir = dirs.first().unwrap();
    let &[erc20, vat, dai, gem_join, dai_join] =
        contracts.iter().by_ref().collect::<Vec<_>>().as_slice()
    else {
        panic!();
    };

    configure_contracts(dir.path(), vat, dai, gem_join, dai_join);

    // smoelius: Join the system by exchanging some ERC20.
    let stdout = send(
        dir,
        erc20,
        ["approve(address,uint256)", gem_join, "2200000000000000000"],
    )
    .unwrap();
    println!("{stdout}");

    let stdout = send(
        dir,
        gem_join,
        ["join(address,uint256)", ADDRESS, "2200000000000000000"],
    )
    .unwrap();
    println!("{stdout}");

    // smoelius: Check that our ERC20 balance went down.
    let stdout = call(dir, erc20, ["balanceOf(address)(uint256)", ADDRESS]).unwrap();
    assert_eq!("999997800000000000000000 [9.999e23]\n", stdout);

    // smoelius: Check that the ERC20 is now held by the `Vat`.
    let stdout = call(dir, vat, ["gem(bytes32,address)(uint256)", GOLD, ADDRESS]).unwrap();
    assert_eq!("2200000000000000000 [2.2e18]\n", stdout);

    // smoelius: Lock the ERC20.
    let stdout = send(
        dir,
        vat,
        [
            "frob(bytes32,address,address,address,int256,int256)",
            GOLD,
            ADDRESS,
            ADDRESS,
            ADDRESS,
            "1500000000000000000",
            "89000000000000000000",
        ],
    )
    .unwrap();
    println!("{stdout}");

    // smoelius: Check our internal DAI balance.
    let stdout = call(dir, vat, ["dai(address)(uint256)", ADDRESS]).unwrap();
    assert_eq!(
        "89000000000000000000000000000000000000000000000 [8.9e46]\n",
        stdout
    );

    // smoelius: Convert the ERC20 to DAI.
    let stdout = send(
        dir,
        dai_join,
        ["exit(address,uint256)", ADDRESS, "89000000000000000000"],
    )
    .unwrap();
    println!("{stdout}");

    // smoelius: Check our external DAI balance.
    let stdout = call(dir, dai, ["balanceOf(address)(uint256)", ADDRESS]).unwrap();
    assert_eq!("89000000000000000000 [8.9e19]\n", stdout);
}

fn deploy_contracts() -> (Vec<TempDir>, Vec<String>) {
    let mut dirs_and_contracts = Vec::new();

    dirs_and_contracts.push(deploy_erc20());

    let erc20 = dirs_and_contracts.last().unwrap().1.clone();

    dirs_and_contracts.push(
        deploy(
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("integration/stylus/dss/vat.sol"),
            "Vat",
            true,
        )
        .unwrap(),
    );

    let _stdout = send(
        &dirs_and_contracts.last().unwrap().0,
        &dirs_and_contracts.last().unwrap().1,
        ["initialize()"],
    )
    .unwrap();
    // println!("{}", stdout);

    let vat = dirs_and_contracts.last().unwrap().1.clone();

    dirs_and_contracts.push(
        deploy(
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("integration/stylus/dss/dai.sol"),
            "Dai",
            true,
        )
        .unwrap(),
    );

    let _stdout = send(
        &dirs_and_contracts.last().unwrap().0,
        &dirs_and_contracts.last().unwrap().1,
        ["initialize(uint256)", &CHAIN_ID.to_string()],
    )
    .unwrap();
    // println!("{}", stdout);

    let dai = dirs_and_contracts.last().unwrap().1.clone();

    dirs_and_contracts.push(
        deploy(
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("integration/stylus/dss/join.sol"),
            "GemJoin",
            true,
        )
        .unwrap(),
    );

    let _stdout = send(
        &dirs_and_contracts.last().unwrap().0,
        &dirs_and_contracts.last().unwrap().1,
        ["initialize(address,bytes32,address)", &vat, GOLD, &erc20],
    )
    .unwrap();
    // println!("{}", stdout);

    dirs_and_contracts.push(
        deploy(
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("integration/stylus/dss/join.sol"),
            "DaiJoin",
            true,
        )
        .unwrap(),
    );

    let _stdout = send(
        &dirs_and_contracts.last().unwrap().0,
        &dirs_and_contracts.last().unwrap().1,
        ["initialize(address,address)", &vat, &dai],
    )
    .unwrap();
    // println!("{}", stdout);

    let contracts = dirs_and_contracts
        .iter()
        .map(|(_, contract)| contract.clone())
        .collect();
    let dirs = dirs_and_contracts.into_iter().map(|(dir, _)| dir).collect();

    (dirs, contracts)
}

const INITIAL_BALANCE: &str = concat!("1000000", "000000000000000000");

fn deploy_erc20() -> (TempDir, String) {
    let (tempdir, address) = deploy(
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("integration/stylus/Uniswap/test/ERC20.sol"),
        "ERC20",
        true,
    )
    .unwrap();
    let dir = &tempdir;

    let _stdout = send(dir, &address, ["initialize(uint256)", INITIAL_BALANCE]).unwrap();
    // println!("{}", stdout);

    let stdout = call(dir, &address, ["totalSupply()(uint256)"]).unwrap();
    assert_eq!("1000000000000000000000000 [1e24]\n", stdout);

    (tempdir, address)
}

fn configure_contracts(dir: &Path, vat: &str, dai: &str, gem_join: &str, dai_join: &str) {
    // smoelius: Allow `GOLD` to be an ilk.
    let _stdout = send(dir, vat, ["init(bytes32)", GOLD]).unwrap();
    // println!("{stdout}");

    // smoelius: Set `GOLD`'s spot price.
    let stdout = send(
        dir,
        vat,
        [
            "file(bytes32,bytes32,uint256)",
            GOLD,
            SPOT,
            "1000000000000000000000000000000000000000000000",
        ],
    )
    .unwrap();
    println!("{stdout}");

    // smoelius: Set `GOLD`'s debt ceiling.
    let _stdout = send(
        dir,
        vat,
        [
            "file(bytes32,bytes32,uint256)",
            GOLD,
            LINE_LOWER,
            "100000000000000000000000000000000000000000000000000000",
        ],
    )
    .unwrap();

    // smoelius: Set the global debt ceiling.
    let _stdout = send(
        dir,
        vat,
        [
            "file(bytes32,uint256)",
            LINE_UPPER,
            "100000000000000000000000000000000000000000000000000000",
        ],
    )
    .unwrap();

    // smoelius: Authorize `gem_join` to call `slip` on the `Vat`.
    let _stdout = send(dir, vat, ["rely(address)", gem_join]).unwrap();
    // println!("{stdout}");

    // smoelius: Authorize `dai_join` to `mint` DAI tokens.
    let _stdout = send(dir, dai, ["rely(address)", dai_join]).unwrap();
    // println!("{stdout}");

    // smoelius: Authorize `dai_join` to call `move` on the `Vat`.
    let _stdout = send(dir, vat, ["hope(address)", dai_join]).unwrap();
    // println!("{stdout}");
}
