// Copyright 2019 MaidSafe.net limited.
//
// This SAFE Network Software is licensed to you under The General Public License (GPL), version 3.
// Unless required by applicable law or agreed to in writing, the SAFE Network Software distributed
// under the GPL Licence is distributed on an "AS IS" BASIS, WITHOUT WARRANTIES OR CONDITIONS OF ANY
// KIND, either express or implied. Please review the Licences for the specific language governing
// permissions and limitations relating to use of the SAFE Network Software.

mod common;

#[macro_use]
extern crate duct;

use assert_cmd::prelude::*;
use common::{get_bin_location, CLI, SAFE_PROTOCOL};
use predicates::prelude::*;
use std::process::Command;

const PRETTY_FILES_CREATION_RESPONSE: &str = "FilesContainer created at: ";
const TEST_FILE: &str = "./tests/testfolder/test.md";
const TEST_FOLDER: &str = "./tests/testfolder/";
const TEST_FOLDER_SUBFOLDER: &str = "./tests/testfolder/subfolder/";
const TEST_EMPTY_FOLDER: &str = "./tests/testfolder/emptyfolder/";

#[test]
fn calling_safe_files_put_pretty() {
    let mut cmd = Command::cargo_bin(CLI).unwrap();
    cmd.args(&vec!["files", "put", TEST_FILE])
        .assert()
        .stdout(predicate::str::contains(PRETTY_FILES_CREATION_RESPONSE))
        .stdout(predicate::str::contains(SAFE_PROTOCOL).count(2))
        .stdout(predicate::str::contains(TEST_FILE).count(1))
        .success();
}

#[test]
fn calling_safe_files_put() {
    let mut cmd = Command::cargo_bin(CLI).unwrap();
    cmd.args(&vec!["files", "put", TEST_FILE, "--json"])
        .assert()
        .stdout(predicate::str::contains(PRETTY_FILES_CREATION_RESPONSE).count(0))
        .stdout(predicate::str::contains(SAFE_PROTOCOL).count(2))
        .stdout(predicate::str::contains(TEST_FILE).count(1))
        .success();
}

#[test]
fn calling_safe_files_put_recursive() {
    let mut cmd = Command::cargo_bin(CLI).unwrap();
    cmd.args(&vec!["files", "put", TEST_FOLDER, "--recursive", "--json"])
        .assert()
        .stdout(predicate::str::contains(SAFE_PROTOCOL).count(4))
        .stdout(predicate::str::contains("./tests/testfolder/test.md").count(1))
        .stdout(predicate::str::contains("./tests/testfolder/another.md").count(1))
        .stdout(predicate::str::contains("./tests/testfolder/subfolder/subexists.md").count(1))
        .success();
}

#[test]
fn calling_safe_files_put_recursive_and_change_root() {
    let file_container = cmd!(
        get_bin_location(),
        "files",
        "put",
        TEST_FOLDER,
        "--recursive",
        "--set-root",
        "aha",
    )
    .read()
    .unwrap();

    let mut lines = file_container.lines();
    let file_container_xor_line = lines.next().unwrap();
    let file_container_xor =
        &file_container_xor_line[PRETTY_FILES_CREATION_RESPONSE.len()..].replace("\"", "");

    let file = format!("{}/aha/test.md", file_container_xor);
    let file_cat = cmd!(get_bin_location(), "cat", &file).read().unwrap();
    assert_eq!(file_cat, "hello tests!");

    let subfile = format!("{}/aha/subfolder/subexists.md", file_container_xor);
    let subfile_cat = cmd!(get_bin_location(), "cat", &subfile).read().unwrap();
    assert_eq!(subfile_cat, "the sub");
}

#[test]
fn calling_safe_files_put_recursive_subfolder() {
    let mut cmd = Command::cargo_bin(CLI).unwrap();
    cmd.args(&vec![
        "files",
        "put",
        TEST_FOLDER_SUBFOLDER,
        "--recursive",
        "--json",
    ])
    .assert()
    .stdout(predicate::str::contains(SAFE_PROTOCOL).count(2))
    .stdout(predicate::str::contains("./tests/testfolder/test.md").count(0))
    .stdout(predicate::str::contains("./tests/testfolder/another.md").count(0))
    .stdout(predicate::str::contains("./tests/testfolder/subfolder/subexists.md").count(1))
    .success();
}

#[test]
fn calling_safe_files_put_emptyfolder() {
    let mut cmd = Command::cargo_bin(CLI).unwrap();
    cmd.args(&vec![
        "files",
        "put",
        TEST_EMPTY_FOLDER,
        "--recursive",
        "--json",
    ])
    .assert()
    .stdout(predicate::str::contains(SAFE_PROTOCOL).count(1))
    .stdout(predicate::str::contains("./tests/testfolder/emptyfolder/").count(0))
    .success();
}
