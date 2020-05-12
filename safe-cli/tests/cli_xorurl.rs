// Copyright 2020 MaidSafe.net limited.
//
// This SAFE Network Software is licensed to you under the MIT license <LICENSE-MIT
// http://opensource.org/licenses/MIT> or the Modified BSD license <LICENSE-BSD
// https://opensource.org/licenses/BSD-3-Clause>, at your option. This file may not be copied,
// modified, or distributed except according to those terms. Please review the Licences for the
// specific language governing permissions and limitations relating to use of the SAFE Network
// Software.

extern crate safe_cmd_test_utilities;

#[macro_use]
extern crate duct;

use assert_cmd::prelude::*;
use predicates::prelude::*;
use safe_api::xorurl::XorUrlEncoder;
use safe_cmd_test_utilities::{parse_xorurl_output, CLI, SAFE_PROTOCOL};
use std::process::Command;
use unwrap::unwrap;

const TEST_FILE: &str = "../testdata/test.md";
const TEST_FOLDER: &str = "../testdata/";

#[test]
fn calling_safe_xorurl() {
    let mut cmd = Command::cargo_bin(CLI).unwrap();
    cmd.args(&vec!["xorurl", TEST_FILE])
        .assert()
        .stdout(predicate::str::contains("1 file/s processed"))
        .stdout(predicate::str::contains(SAFE_PROTOCOL).count(1))
        .stdout(predicate::str::contains(TEST_FILE).count(1))
        .success();
}

#[test]
fn calling_safe_xorurl_recursive() {
    let mut cmd = Command::cargo_bin(CLI).unwrap();
    cmd.args(&vec!["xorurl", TEST_FOLDER, "--recursive"])
        .assert()
        .stdout(predicate::str::contains("5 file/s processed"))
        .stdout(predicate::str::contains(SAFE_PROTOCOL).count(5))
        .stdout(predicate::str::contains(TEST_FOLDER).count(5))
        .success();
}

#[test]
fn calling_safe_xorurl_decode() {
    let content = cmd!(env!("CARGO_BIN_EXE_safe"), "xorurl", TEST_FILE, "--json",)
        .read()
        .unwrap();

    let xorurls = parse_xorurl_output(&content);
    let file_xorurl = &xorurls[0].1;
    let xorurl_encoder = unwrap!(XorUrlEncoder::from_url(file_xorurl));

    let xorurl_decoded = cmd!(
        env!("CARGO_BIN_EXE_safe"),
        "xorurl",
        "decode",
        &file_xorurl,
        "--json",
    )
    .read()
    .unwrap();

    let decoded_obj: XorUrlEncoder = serde_json::from_str(&xorurl_decoded)
        .expect("Failed to parse output of `safe xorurl decode`");

    assert_eq!(xorurl_encoder, decoded_obj);
}
