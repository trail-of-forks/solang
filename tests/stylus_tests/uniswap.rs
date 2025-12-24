//! This test expects you to have a devnode running:
//! <https://docs.arbitrum.io/run-arbitrum-node/run-nitro-dev-node>
//!
//! It also expects `cargo-stylus` and `cast` to be installed:
//! - <https://github.com/OffchainLabs/cargo-stylus>
//! - <https://book.getfoundry.sh/cast/>
#![warn(clippy::pedantic)]

use crate::{call, deploy, send, ADDRESS, MUTEX};
use std::path::{Path, PathBuf};
use tempfile::TempDir;

// smoelius: In this test, the pool is populated with 10000 of each token. Then, `INCOMING` (an
// unknown) number of tokens tokens are swapped-in to get 2500 tokens out. Uniswap's equations
// require that:
//
//     10000 * 10^3 + INCOMING * 9997 + 7500 * 10^3 >= (10000 * 10^3)^2
//
// Setting `INCOMING` to 3344 makes this inequality work, but anything smaller does not.
const INCOMING: &str = "3344";
const OUTGOING: &str = "2500";

#[test]
fn uniswap() {
    let _lock = MUTEX.lock();
    let (_erc20_a_dir, erc20_a) = deploy_erc20();
    let (_erc20_b_dir, erc20_b) = deploy_erc20();

    let incoming = (&erc20_a).min(&erc20_b);
    let outgoing = (&erc20_a).max(&erc20_b);

    let (_callee_dir, callee) = deploy_callee(&incoming);

    let (_pair_base_dir, pair_base) = deploy_pair_base();

    let (_pair_creator_dir, pair_creator) = deploy_pair_creator(&pair_base);

    let (factory_dir, factory) = deploy_uniswap_factory(&pair_base, &pair_creator);

    let dir = &factory_dir;

    println!("      erc20_a: {}", erc20_a);
    println!("      erc20_b: {}", erc20_b);
    println!("    pair_base: {}", pair_base);
    println!(" pair_creator: {}", pair_creator);
    println!("      factory: {}", factory);

    let pair = create_pair(dir, &factory, &erc20_a, &erc20_b);
    println!("         pair: 0x{}", pair);

    let stdout = call(dir, &pair, ["base()(address)"]).unwrap();
    println!("    pair.base: {}", stdout);

    let stdout = call(dir, &pair, ["token0()(address)"]).unwrap();
    println!("  pair.token0: {}", stdout);

    let stdout = call(dir, &pair, ["token1()(address)"]).unwrap();
    println!("  pair.token1: {}", stdout);

    // smoelius: `MINIMUM_LIQUIDITY` is 1000. If the amount of each token transferred to the pair is
    // more than this, then `MINIMUM_LIQUIDITY` will be satisfied.
    let _stdout = send(dir, &erc20_a, ["transfer(address,uint256)", &pair, "10000"]).unwrap();
    // println!("{}", stdout);

    let _stdout = send(dir, &erc20_b, ["transfer(address,uint256)", &pair, "10000"]).unwrap();
    // println!("{}", stdout);

    // smoelius: At the time of this writing, 21000 gas is not sufficient to call `mint`. The logs
    // suggest around 36000 is needed.
    let stdout = send(
        dir,
        &pair,
        ["mint(address)", ADDRESS, "--gas-limit=50000000"],
    )
    .unwrap();
    println!("{}", stdout);

    let stdout = call(dir, &pair, ["balanceOf(address)(uint256)", ADDRESS]).unwrap();
    assert_eq!("9000\n", stdout);

    // smoelius: Transfer `INCOMING` tokens to `callee` to swap them.
    let _stdout = send(
        dir,
        incoming,
        ["transfer(address,uint256)", &callee, INCOMING],
    )
    .unwrap();
    // println!("{}", stdout);

    let stdout = call(dir, incoming, ["balanceOf(address)(uint256)", &callee]).unwrap();
    assert_eq!(INCOMING, stdout.split_ascii_whitespace().next().unwrap());

    let stdout = call(dir, &outgoing, ["balanceOf(address)(uint256)", &callee]).unwrap();
    assert_eq!("0\n", stdout);

    // smoelius: The `0x111...` is `swap`'s `data` argument. It must be non-empty for `swap` to call
    // `callee`. However, our `callee` does not use this data.
    let stdout = send(
        dir,
        &pair,
        [
            "swap(uint256,uint256,address,bytes)",
            "0",
            OUTGOING,
            &callee,
            "0x1111111111111111111111111111111111111111111111111111111111111111",
            "--gas-limit=50000000",
        ],
    )
    .unwrap();
    println!("{}", stdout);

    let stdout = call(dir, incoming, ["balanceOf(address)(uint256)", &callee]).unwrap();
    assert_eq!("0\n", stdout);

    let stdout = call(dir, outgoing, ["balanceOf(address)(uint256)", &callee]).unwrap();
    assert_eq!(OUTGOING, stdout.split_ascii_whitespace().next().unwrap());
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

fn deploy_callee(token: &str) -> (TempDir, String) {
    let (tempdir, address) = deploy(
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("integration/stylus/Uniswap/Callee.sol"),
        "Callee",
        true,
    )
    .unwrap();

    let _stdout = send(&tempdir, &address, ["initialize(address)", token]).unwrap();
    // println!("{}", stdout);

    (tempdir, address)
}

fn deploy_pair_base() -> (TempDir, String) {
    let (tempdir, address) = deploy(
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("integration/stylus/Uniswap/UniswapV2PairBase.sol"),
        "UniswapV2PairBase",
        true,
    )
    .unwrap();

    let stdout = call(&tempdir, &address, ["balanceOf(address)(uint256)", ADDRESS]).unwrap();
    assert_eq!("0\n", stdout);

    let stdout = call(&tempdir, &address, ["factory()(address)"]).unwrap();
    assert_eq!("0x0000000000000000000000000000000000000000\n", stdout);

    (tempdir, address)
}

fn deploy_pair_creator(pair_base: &str) -> (TempDir, String) {
    let (tempdir, address) = deploy(
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("integration/stylus/Uniswap/UniswapV2PairCreator.sol"),
        "UniswapV2PairCreator",
        true,
    )
    .unwrap();

    // smoelius: Create a test pair with bogus token addresses.
    let _stdout = send(
        &tempdir,
        &address,
        [
            "createPairWithBase(address,address,address,uint256)(address)",
            &pair_base,
            "0000000000000000000000000000000000000001",
            "0000000000000000000000000000000000000002",
            "1",
            "--value=1000000000000000000",
        ],
    )
    .unwrap();
    // println!("{}", stdout);

    (tempdir, address)
}

fn deploy_uniswap_factory(pair_base: &str, pair_creator: &str) -> (TempDir, String) {
    let (tempdir, address) = deploy(
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("integration/stylus/Uniswap/UniswapV2Factory.sol"),
        "UniswapV2Factory",
        true,
    )
    .unwrap();

    let _stdout = send(
        &tempdir,
        &address,
        [
            "initialize(address,address,address)",
            ADDRESS,
            pair_base,
            pair_creator,
        ],
    )
    .unwrap();
    // println!("{}", stdout);

    (tempdir, address)
}

fn create_pair(dir: impl AsRef<Path>, factory: &str, erc20_a: &str, erc20_b: &str) -> String {
    let stdout = send(
        dir,
        &factory,
        [
            "createPair(address,address)(address)",
            &erc20_a,
            &erc20_b,
            "--value=1000000000000000000",
        ],
    )
    .unwrap();
    let line = stdout
        .lines()
        .find(|line| line.starts_with("logs"))
        .unwrap();
    // smoelius: Note that `data` starts with three 20-byte addresses padded to 32 bytes, and we
    // want the third. Thus, `data` here consists of two 32-byte strings followed by 12 bytes of
    // zeroes for the third address's padding.
    const PREFIX: &str = concat!(
        r#"logs                 [{"#,
        r#""address":"0xffffffffffffffffffffffffffffffffffffffff","#,
        r#""topics":["#,
        r#""0xffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff","#,
        r#""0x000000000000000000000000ffffffffffffffffffffffffffffffffffffffff","#,
        r#""0x000000000000000000000000ffffffffffffffffffffffffffffffffffffffff""#,
        r#"],"#,
        r#""data":"0x000000000000000000000000ffffffffffffffffffffffffffffffffffffffff000000000000000000000000ffffffffffffffffffffffffffffffffffffffff000000000000000000000000"#
    );
    assert_roughly_starts_with(line, PREFIX);
    let pair = line.chars().skip(PREFIX.len()).take(40).collect::<String>();
    assert!(pair.chars().all(|c| c.is_ascii_hexdigit()));
    pair
}

fn assert_roughly_starts_with(line: &str, prefix: &str) {
    assert!(line.chars().count() >= prefix.chars().count());
    for (i, (line_char, prefix_char)) in line.chars().zip(prefix.chars()).enumerate() {
        if prefix_char == 'f' {
            continue;
        }
        assert_eq!(
            line_char, prefix_char,
            "mismatch at index {i}\n
    line: {line}
  prefix: {prefix}"
        )
    }
}
