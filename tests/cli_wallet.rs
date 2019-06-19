// Copyright 2019 MaidSafe.net limited.
//
// This SAFE Network Software is licensed to you under The General Public License (GPL), version 3.
// Unless required by applicable law or agreed to in writing, the SAFE Network Software distributed
// under the GPL Licence is distributed on an "AS IS" BASIS, WITHOUT WARRANTIES OR CONDITIONS OF ANY
// KIND, either express or implied. Please review the Licences for the specific language governing
// permissions and limitations relating to use of the SAFE Network Software.

extern crate serde_json;
#[macro_use]
extern crate duct;

use assert_cmd::prelude::*;
use predicates::prelude::*;

use std::process::Command;

static CLI: &str = "safe_cli";
static PRETTY_WALLET_CREATION_RESPONSE: &str = "Wallet created at";
static SAFE_PROTOCOL: &str = "safe://";

fn get_bin_location() -> &'static str {
    let mut location = "./target/release/safe_cli";
    if cfg!(debug_assertions) {
        location = "./target/debug/safe_cli";
    }
    location
}

fn create_preload_and_get_keys(preload: &str) -> (String, String) {
    // KEY_FROM
    let pk_command_result = cmd!(
        get_bin_location(),
        "keys",
        "create",
        "--test-coins",
        "---preload",
        preload
    )
    .read()
    .unwrap();

    let mut lines = pk_command_result.lines();
    let pk_xor = lines.next().unwrap();
    let _pk = lines.next().unwrap();
    let sk_line = lines.next().unwrap();
    let sk_eq = String::from("sk=");
    let sk = &sk_line[sk_eq.chars().count()..];

    (pk_xor.to_string(), sk.to_string())
}

#[test]
fn calling_safe_wallet_transfer() {
    let mut cmd = Command::cargo_bin(CLI).unwrap();

    // FROM
    let wallet_from = cmd!(get_bin_location(), "wallet", "create").read().unwrap();
    assert!(wallet_from.contains(SAFE_PROTOCOL));

    // TO
    let wallet_to = cmd!(get_bin_location(), "wallet", "create").read().unwrap();
    assert!(wallet_to.contains(SAFE_PROTOCOL));

    let (pk_from_xorurl, from_sk) = create_preload_and_get_keys("123");

    let wallet_from_insert = cmd!(
        get_bin_location(),
        "wallet",
        "insert",
        &pk_from_xorurl,
        &wallet_from,
        &pk_from_xorurl,
        "--name",
        "our_from_wallet",
        "--default",
        "--secret-key",
        &from_sk
    )
    .read()
    .unwrap();

    assert_eq!(&wallet_from, &wallet_from_insert);

    let (pk_to_xorurl, to_sk) = create_preload_and_get_keys("3");

    let wallet_to_insert = cmd!(
        get_bin_location(),
        "wallet",
        "insert",
        &pk_to_xorurl,
        &wallet_to,
        &pk_to_xorurl,
        "--name",
        "our_to_wallet",
        "--default",
        "--secret-key",
        &to_sk
    )
    .read()
    .unwrap();

    assert_eq!(&wallet_to, &wallet_to_insert);

    cmd.args(&vec![
        "wallet",
        "transfer",
        "100",
        &wallet_to,
        &wallet_from,
        "--pretty",
    ])
    .assert()
    .stdout(predicate::str::contains("Success"))
    .stdout(predicate::str::contains("TX_ID"))
    .success();

    // To got coins?
    let to_has = cmd!(get_bin_location(), "wallet", "balance", &wallet_to)
        .read()
        .unwrap();

    assert_eq!(to_has, "103");

    // from lost coins?
    let from_has = cmd!(get_bin_location(), "wallet", "balance", &wallet_from)
        .read()
        .unwrap();

    assert_eq!(from_has, "23")
}

#[test]
fn calling_safe_wallet_balance_pretty_no_sk() {
    let mut cmd = Command::cargo_bin(CLI).unwrap();

    let wallet = cmd!(get_bin_location(), "wallet", "create").read().unwrap();
    assert!(wallet.contains(SAFE_PROTOCOL));

    let (pk_to_xorurl, to_sk) = create_preload_and_get_keys("300");

    let _wallet_to_insert = cmd!(
        get_bin_location(),
        "wallet",
        "insert",
        &pk_to_xorurl,
        &wallet,
        &pk_to_xorurl
    )
    .input(to_sk)
    .read()
    .unwrap();

    cmd.args(&vec!["wallet", "balance", &wallet])
        .assert()
        .stdout("300\n")
        .success();
}

#[test]
fn calling_safe_wallet_balance() {
    let mut cmd = Command::cargo_bin(CLI).unwrap();

    let wallet = cmd!(get_bin_location(), "wallet", "create").read().unwrap();
    assert!(wallet.contains(SAFE_PROTOCOL));

    cmd.args(&vec![
        "wallet", "balance", &wallet,
        // "--pretty",
    ])
    .assert()
    .stdout("0\n")
    .success();
}

#[test]
fn calling_safe_wallet_insert_w_preload() {
    let mut cmd = Command::cargo_bin(CLI).unwrap();

    let wallet = cmd!(get_bin_location(), "wallet", "create").read().unwrap();
    assert!(wallet.contains(SAFE_PROTOCOL));

    let (pk_pay_xor, _pay_sk) = create_preload_and_get_keys("300");

    let _wallet_to_insert = cmd!(
        get_bin_location(),
        "wallet",
        "insert",
        &pk_pay_xor,
        &wallet,
        "--test-coins",
        "--preload",
        "150",
    )
    .read()
    .unwrap();

    cmd.args(&vec!["wallet", "balance", &wallet])
        .assert()
        .stdout("150\n")
        .success();
}

#[test]
fn calling_safe_wallet_create() {
    let mut cmd = Command::cargo_bin(CLI).unwrap();
    cmd.args(&vec!["wallet", "create", "--pretty"])
        .assert()
        .stdout(predicate::str::starts_with(PRETTY_WALLET_CREATION_RESPONSE).from_utf8())
        .success();
}

#[test]
fn calling_safe_wallet_sweep() {
    let mut cmd = Command::cargo_bin(CLI).unwrap();

    // FROM
    let wallet_from = cmd!(get_bin_location(), "wallet", "create").read().unwrap();
    assert!(wallet_from.contains(SAFE_PROTOCOL));

    // TO
    let wallet_to = cmd!(get_bin_location(), "wallet", "create").read().unwrap();
    assert!(wallet_to.contains(SAFE_PROTOCOL));

    let (pk_from_xorurl, from_sk) = create_preload_and_get_keys("123");

    let wallet_from_insert = cmd!(
        get_bin_location(),
        "wallet",
        "insert",
        &pk_from_xorurl,
        &wallet_from,
        &pk_from_xorurl,
        "--name",
        "our_from_wallet",
        "--default",
        "--secret-key",
        &from_sk
    )
    .read()
    .unwrap();

    assert_eq!(&wallet_from, &wallet_from_insert);

    let (pk_to_xorurl, to_sk) = create_preload_and_get_keys("3");

    let wallet_to_insert = cmd!(
        get_bin_location(),
        "wallet",
        "insert",
        &pk_to_xorurl,
        &wallet_to,
        &pk_to_xorurl,
        "--name",
        "our_to_wallet",
        "--default",
        "--secret-key",
        &to_sk
    )
    .read()
    .unwrap();

    assert_eq!(&wallet_to, &wallet_to_insert);

    cmd.args(&vec![
        "wallet",
        "sweep",
        "--from",
        &wallet_from,
        "--to",
        &wallet_to,
        "--pretty",
    ])
    .assert()
    .stdout(predicate::str::contains("Success"))
    .stdout(predicate::str::contains("TX_ID"))
    .success();

    // To got coins?
    let to_has = cmd!(get_bin_location(), "wallet", "balance", &wallet_to)
        .read()
        .unwrap();

    assert_eq!(to_has, "126");

    // from lost coins?
    let from_has = cmd!(get_bin_location(), "wallet", "balance", &wallet_from)
        .read()
        .unwrap();

    assert_eq!(from_has, "0")
}

#[test]
fn calling_safe_wallet_get_transaction() {
    let mut cmd = Command::cargo_bin(CLI).unwrap();

    // FROM
    let wallet_from = cmd!(get_bin_location(), "wallet", "create").read().unwrap();
    assert!(wallet_from.contains(SAFE_PROTOCOL));

    // TO
    let wallet_to = cmd!(get_bin_location(), "wallet", "create").read().unwrap();
    assert!(wallet_to.contains(SAFE_PROTOCOL));

    let (pk_from_xorurl, from_sk) = create_preload_and_get_keys("123");

    let wallet_from_insert = cmd!(
        get_bin_location(),
        "wallet",
        "insert",
        &pk_from_xorurl,
        &wallet_from,
        &pk_from_xorurl,
        "--name",
        "our_from_wallet",
        "--default",
        "--secret-key",
        &from_sk
    )
    .read()
    .unwrap();

    assert_eq!(&wallet_from, &wallet_from_insert);

    let (pk_to_xorurl, to_sk) = create_preload_and_get_keys("3");

    let wallet_to_insert = cmd!(
        get_bin_location(),
        "wallet",
        "insert",
        &pk_to_xorurl,
        &wallet_to,
        &pk_to_xorurl,
        "--name",
        "our_to_wallet",
        "--default",
        "--secret-key",
        &to_sk
    )
    .read()
    .unwrap();

    assert_eq!(&wallet_to, &wallet_to_insert);

    let transaction_id = cmd!(
        get_bin_location(),
        "wallet",
        "sweep",
        "--from",
        &wallet_from,
        "--to",
        &wallet_to,
    )
    .read()
    .unwrap();

    let wallet = cmd!(get_bin_location(), "wallet", "create").read().unwrap();
    assert!(wallet.contains(SAFE_PROTOCOL));

    cmd.args(&vec![
        "wallet",
        "check-tx",
        &transaction_id,
        &pk_to_xorurl,
        "--pretty",
    ])
    .assert()
    .stdout(predicate::str::contains("Success"))
    .success();
}
