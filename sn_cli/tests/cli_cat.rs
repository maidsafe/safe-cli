// Copyright 2020 MaidSafe.net limited.
//
// This SAFE Network Software is licensed to you under the MIT license <LICENSE-MIT
// http://opensource.org/licenses/MIT> or the Modified BSD license <LICENSE-BSD
// https://opensource.org/licenses/BSD-3-Clause>, at your option. This file may not be copied,
// modified, or distributed except according to those terms. Please review the Licences for the
// specific language governing permissions and limitations relating to use of the SAFE Network
// Software.

extern crate sn_cmd_test_utilities;

#[macro_use]
extern crate duct;

use anyhow::{anyhow, Result};
use assert_cmd::prelude::*;
use predicates::prelude::*;
use sn_api::{
    fetch::{SafeContentType, SafeDataType},
    sk_to_hex, Keypair,
};
use sn_cmd_test_utilities::{
    create_preload_and_get_keys, get_random_nrs_string, parse_cat_wallet_output,
    parse_files_container_output, parse_files_put_or_sync_output, safe_cmd_stderr, safe_cmd_stdout,
    safeurl_from, test_symlinks_are_valid, upload_test_symlinks_folder, CLI,
};
use std::process::Command;

const TEST_DATA: &str = "../testdata/";
const TEST_FILE: &str = "../testdata/test.md";
const TEST_FILE_CONTENT: &str = "hello tests!";
const ID_RELATIVE_FILE_ERROR: &str = "Cannot get relative path of Immutable Data";
const TEST_FILE_HEXDUMP_CONTENT: &str = "Length: 12 (0xc) bytes\n0000:   68 65 6c 6c  6f 20 74 65  73 74 73 21                hello tests!\n";
const ANOTHER_FILE: &str = "../testdata/another.md";
const ANOTHER_FILE_CONTENT: &str = "exists";

#[test]
fn calling_safe_cat() -> Result<()> {
    let content = cmd!(
        env!("CARGO_BIN_EXE_safe"),
        "files",
        "put",
        TEST_FILE,
        "--json"
    )
    .read()
    .map_err(|e| anyhow!(e.to_string()))?;

    let (_container_xorurl, map) = parse_files_put_or_sync_output(&content);
    let mut cmd = Command::cargo_bin(CLI).map_err(|e| anyhow!(e.to_string()))?;
    cmd.args(&vec!["cat", &map[TEST_FILE].1])
        .assert()
        .stdout(predicate::str::contains(TEST_FILE_CONTENT))
        .success();

    let safeurl = safeurl_from(&map[TEST_FILE].1)?;
    assert_eq!(
        safeurl.content_type(),
        SafeContentType::MediaType("text/markdown".to_string())
    );
    assert_eq!(safeurl.data_type(), SafeDataType::PublicBlob);
    Ok(())
}

#[test]
fn calling_safe_cat_subfolders() -> Result<()> {
    let content = cmd!(
        env!("CARGO_BIN_EXE_safe"),
        "files",
        "put",
        TEST_DATA,
        "--json",
        "--recursive",
    )
    .read()
    .map_err(|e| anyhow!(e.to_string()))?;

    let (container_xorurl, _) = parse_files_put_or_sync_output(&content);

    let content = cmd!(
        env!("CARGO_BIN_EXE_safe"),
        "cat",
        &container_xorurl,
        "--json",
    )
    .read()
    .map_err(|e| anyhow!(e.to_string()))?;

    let (_xorurl, filesmap) = parse_files_container_output(&content);

    assert_eq!(filesmap["/emptyfolder"]["type"], "inode/directory");
    assert_eq!(filesmap["/emptyfolder"]["size"], "0");
    assert_eq!(filesmap["/subfolder"]["type"], "inode/directory");
    assert_eq!(filesmap["/subfolder"]["size"], "0");
    Ok(())
}

#[test]
fn calling_safe_cat_on_relative_file_from_id_fails() -> Result<()> {
    let content = cmd!(
        env!("CARGO_BIN_EXE_safe"),
        "files",
        "put",
        TEST_FILE,
        "--json"
    )
    .read()
    .map_err(|e| anyhow!(e.to_string()))?;

    let (_container_xorurl, map) = parse_files_put_or_sync_output(&content);
    let mut cmd = Command::cargo_bin(CLI).map_err(|e| anyhow!(e.to_string()))?;

    let relative_url = format!("{}/something_relative.wasm", &map[TEST_FILE].1);
    cmd.args(&vec!["cat", &relative_url])
        .assert()
        .stderr(predicate::str::contains(ID_RELATIVE_FILE_ERROR))
        .failure();
    Ok(())
}

#[test]
fn calling_safe_cat_hexdump() -> Result<()> {
    let content = cmd!(
        env!("CARGO_BIN_EXE_safe"),
        "files",
        "put",
        TEST_FILE,
        "--json"
    )
    .read()
    .map_err(|e| anyhow!(e.to_string()))?;

    let (_container_xorurl, map) = parse_files_put_or_sync_output(&content);
    let mut cmd = Command::cargo_bin(CLI).map_err(|e| anyhow!(e.to_string()))?;
    cmd.args(&vec!["cat", "--hexdump", &map[TEST_FILE].1])
        .assert()
        .stdout(predicate::str::contains(TEST_FILE_HEXDUMP_CONTENT))
        .success();

    let safeurl = safeurl_from(&map[TEST_FILE].1)?;
    assert_eq!(
        safeurl.content_type(),
        SafeContentType::MediaType("text/markdown".to_string())
    );
    assert_eq!(safeurl.data_type(), SafeDataType::PublicBlob);
    Ok(())
}

#[test]
fn calling_safe_cat_xorurl_url_with_version() -> Result<()> {
    let content = cmd!(
        env!("CARGO_BIN_EXE_safe"),
        "files",
        "put",
        TEST_FILE,
        "--json"
    )
    .read()
    .map_err(|e| anyhow!(e.to_string()))?;
    let (container_xorurl, _files_map) = parse_files_put_or_sync_output(&content);

    // let's sync with another file so we get a new version, and a different content in the file
    let mut safeurl = safeurl_from(&container_xorurl)?;
    safeurl.set_path("/test.md");
    safeurl.set_content_hash(None);
    let mut cmd = Command::cargo_bin(CLI).map_err(|e| anyhow!(e.to_string()))?;
    cmd.args(&vec!["files", "sync", ANOTHER_FILE, &safeurl.to_string()])
        .assert()
        .success();

    safeurl.set_content_hash(None);
    let mut cmd = Command::cargo_bin(CLI).map_err(|e| anyhow!(e.to_string()))?;
    cmd.args(&vec!["cat", &safeurl.to_string()])
        .assert()
        .stdout(predicate::str::contains(ANOTHER_FILE_CONTENT))
        .success();

    safeurl.set_content_hash(Some(0));
    let mut cmd = Command::cargo_bin(CLI).map_err(|e| anyhow!(e.to_string()))?;
    cmd.args(&vec!["cat", &safeurl.to_string()])
        .assert()
        .stdout(predicate::str::contains(TEST_FILE_CONTENT))
        .success();

    safeurl.set_content_hash(Some(1));
    let mut cmd = Command::cargo_bin(CLI).map_err(|e| anyhow!(e.to_string()))?;
    cmd.args(&vec!["cat", &safeurl.to_string()])
        .assert()
        .stdout(predicate::str::contains(ANOTHER_FILE_CONTENT))
        .success();

    safeurl.set_content_hash(Some(2));
    let mut cmd = Command::cargo_bin(CLI).map_err(|e| anyhow!(e.to_string()))?;
    cmd.args(&vec!["cat", &safeurl.to_string()])
        .assert()
        .failure();
    Ok(())
}

#[test]
fn calling_safe_cat_nrsurl_with_version() -> Result<()> {
    let content = cmd!(
        env!("CARGO_BIN_EXE_safe"),
        "files",
        "put",
        TEST_FILE,
        "--json"
    )
    .read()
    .map_err(|e| anyhow!(e.to_string()))?;
    let (container_xorurl, _files_map) = parse_files_put_or_sync_output(&content);

    let nrsurl = format!("safe://{}", get_random_nrs_string());
    let _ = cmd!(
        env!("CARGO_BIN_EXE_safe"),
        "nrs",
        "create",
        &nrsurl,
        "-l",
        &container_xorurl,
    )
    .read()
    .map_err(|e| anyhow!(e.to_string()))?;

    let nrsurl_with_path = format!("{}/test.md", nrsurl);
    let mut cmd = Command::cargo_bin(CLI).map_err(|e| anyhow!(e.to_string()))?;
    cmd.args(&vec!["cat", &nrsurl_with_path])
        .assert()
        .stdout(predicate::str::contains(TEST_FILE_CONTENT))
        .success();

    // let's sync with another file so we get a new version, and a different content in the file
    let mut safeurl = safeurl_from(&container_xorurl)?;
    safeurl.set_path("/test.md");
    safeurl.set_content_hash(None);
    let mut cmd = Command::cargo_bin(CLI).map_err(|e| anyhow!(e.to_string()))?;
    cmd.args(&vec!["files", "sync", ANOTHER_FILE, &safeurl.to_string()])
        .assert()
        .success();

    // NRS name was not updated (with --updated-nrs) when doing files sync,
    // so our file should not have been updated
    let mut cmd = Command::cargo_bin(CLI).map_err(|e| anyhow!(e.to_string()))?;
    cmd.args(&vec!["cat", &nrsurl_with_path])
        .assert()
        .stdout(predicate::str::contains(TEST_FILE_CONTENT))
        .success();

    // NRS name has only one version which is 0, so using version 0 should also fetch the file
    let nrsurl_with_path_v0 = format!("{}/test.md?v=0", nrsurl);
    let mut cmd = Command::cargo_bin(CLI).map_err(|e| anyhow!(e.to_string()))?;
    cmd.args(&vec!["cat", &nrsurl_with_path_v0])
        .assert()
        .stdout(predicate::str::contains(TEST_FILE_CONTENT))
        .success();

    // there is no version 1 of NRS name
    let invalid_version_nrsurl = format!("{}/test.md?v=1", nrsurl);
    let mut cmd = Command::cargo_bin(CLI).map_err(|e| anyhow!(e.to_string()))?;
    cmd.args(&vec!["cat", &invalid_version_nrsurl])
        .assert()
        .failure();
    Ok(())
}

#[test]
fn calling_safe_cat_wallet_xorurl() -> Result<()> {
    let wallet_create = cmd!(
        env!("CARGO_BIN_EXE_safe"),
        "wallet",
        "create",
        "--test-coins",
        "--json"
    )
    .read()
    .map_err(|e| anyhow!(e.to_string()))?;
    let (wallet_xorurl, key_xorurl, key_pair): (String, String, Option<Keypair>) =
        serde_json::from_str(&wallet_create)
            .map_err(|_| anyhow!("Failed to parse output of `safe wallet create`"))?;

    let (key_pk_xor, sk) = create_preload_and_get_keys("7")?;
    let _wallet_insert_result = cmd!(
        env!("CARGO_BIN_EXE_safe"),
        "wallet",
        "insert",
        &wallet_xorurl,
        "--keyurl",
        &key_pk_xor,
        "--sk",
        &sk,
    )
    .read()
    .map_err(|e| anyhow!(e.to_string()))?;

    let key_pair = key_pair.ok_or_else(|| anyhow!("Could not fetch Keypair".to_string()))?;
    let secret_key = key_pair.secret_key().map_err(|e| anyhow!(e.to_string()))?;

    let wallet_cat = cmd!(env!("CARGO_BIN_EXE_safe"), "cat", &wallet_xorurl, "--json")
        .read()
        .map_err(|e| anyhow!(e.to_string()))?;
    let (xorurl, balances) = parse_cat_wallet_output(&wallet_cat);

    assert_eq!(wallet_xorurl, xorurl);
    assert_eq!(balances.len(), 2);

    assert_eq!(balances[&key_xorurl].0, true);
    assert_eq!(balances[&key_xorurl].1.xorurl, key_xorurl);
    assert_eq!(balances[&key_xorurl].1.sk, sk_to_hex(secret_key));

    assert_eq!(balances[&key_pk_xor].0, false);
    assert_eq!(balances[&key_pk_xor].1.xorurl, key_pk_xor);
    assert_eq!(balances[&key_pk_xor].1.sk, sk);
    Ok(())
}

#[test]
fn calling_safe_cat_safekey() -> Result<()> {
    let (safekey_xorurl, _sk) = create_preload_and_get_keys("0")?;

    let cat_output = cmd!(env!("CARGO_BIN_EXE_safe"), "cat", &safekey_xorurl,)
        .read()
        .map_err(|e| anyhow!(e.to_string()))?;

    assert_eq!(cat_output, "No content to show since the URL targets a SafeKey. Use the 'dog' command to obtain additional information about the targeted SafeKey.");
    Ok(())
}

// Test:  safe cat <src>/<path>
//    src is symlinks_test dir, put with trailing slash.
//    path references both directory and file relative symlinks
//         including parent dir and sibling dir link targets.
//         Final destination is the file sibling_dir_file.md
//         which is itself a symlink to hello.md.
//
//         realpath: /sub2/hello.md
//
//    expected result: cmd output matches contents of
//                     ../test_symlinks/sub2/hello.md
#[test]
fn calling_cat_symlinks_resolve_dir_and_file() -> Result<()> {
    // Bail if test_symlinks not valid. Typically indicates missing perms on windows.
    if !test_symlinks_are_valid()? {
        return Ok(());
    }

    let (url, ..) = upload_test_symlinks_folder(true)?;
    let mut safeurl = safeurl_from(&url)?;
    safeurl.set_path("/dir_link_link/parent_dir/dir_link/sibling_dir_file.md");

    let args = ["cat", &safeurl.to_string()];
    let output = safe_cmd_stdout(&args, Some(0))?;

    assert_eq!(output.trim(), "= Hello =");

    Ok(())
}

// Test:  safe cat <src>/<path>
//    src is symlinks_test dir, put with trailing slash.
//    path references a symlink that links to itself.
//         (infinite loop)
//
//    expected result: error, too many links.
#[test]
fn calling_cat_symlinks_resolve_infinite_loop() -> Result<()> {
    // Bail if test_symlinks not valid. Typically indicates missing perms on windows.
    if !test_symlinks_are_valid()? {
        return Ok(());
    }

    let (url, ..) = upload_test_symlinks_folder(true)?;
    let mut safeurl = safeurl_from(&url)?;

    safeurl.set_path("/sub/infinite_loop");
    let args = ["cat", &safeurl.to_string()];
    let output = safe_cmd_stderr(&args, Some(1))?;
    assert!(output.contains("ContentNotFound - Too many levels of symbolic links"));

    Ok(())
}

// Test:  safe cat <src>/dir_link_deep/../readme.md
//    src is symlinks_test dir, put with trailing slash.
//    path should resolve as follows:
//         dir_link_deep --> sub/deep
//         ../           --> sub
//         readme.md     --> readme.md
//
//         realpath: /sub/readme.md
//
//    This test verifies that "../" is being resolved
//    correctly *after* dir_link_deep resolution, not before.
//
//    On unix, this behavior can be verified with:
//       $ cat ../test_symlinks/dir_link_deep/../readme.md
//       = This is a real markdown file. =
//
//    note: This test always failed when SafeUrl
//          used rust-url for parsing path because it
//          normalizes away the "../" with no option
//          to obtain the raw path.
//          filed issue: https://github.com/servo/rust-url/issues/602
//
//    expected result: cmd output matches contents of
//                     /sub/readme.md
#[test]
fn calling_cat_symlinks_resolve_parent_dir() -> Result<()> {
    // Bail if test_symlinks not valid. Typically indicates missing perms on windows.
    if !test_symlinks_are_valid()? {
        return Ok(());
    }

    let (url, ..) = upload_test_symlinks_folder(true)?;
    let mut safeurl = safeurl_from(&url)?;

    safeurl.set_path("/dir_link_deep/../readme.md");
    let args = ["cat", &safeurl.to_string()];
    let output = safe_cmd_stdout(&args, Some(0))?;
    assert_eq!(output.trim(), "= This is a real markdown file. =");

    Ok(())
}

// Test:  safe cat <src>/dir_outside
//    src is symlinks_test dir, put with trailing slash.
//    path references a symlink with target outside the FileContainer
//
//    expected result: error, too many links.
#[test]
fn calling_cat_symlinks_resolve_dir_outside() -> Result<()> {
    // Bail if test_symlinks not valid. Typically indicates missing perms on windows.
    if !test_symlinks_are_valid()? {
        return Ok(());
    }

    let (url, ..) = upload_test_symlinks_folder(true)?;
    let mut safeurl = safeurl_from(&url)?;

    safeurl.set_path("/dir_outside");
    let args = ["cat", &safeurl.to_string()];
    let output = safe_cmd_stderr(&args, Some(1))?;
    assert!(output.contains("ContentNotFound - Cannot ascend beyond root directory"));

    Ok(())
}
