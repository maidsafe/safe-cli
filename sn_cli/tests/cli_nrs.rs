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
    safeurl::{SafeUrl, XorUrlBase},
};
use sn_cmd_test_utilities::{
    get_random_nrs_string, parse_nrs_create_output, safeurl_from, upload_test_folder, CLI,
    SAFE_PROTOCOL,
};
use std::process::Command;
use xor_name::XorName;

const PRETTY_NRS_CREATION_RESPONSE: &str = "New NRS Map";

fn gen_fake_target() -> Result<String> {
    let xorname = XorName(*b"12345678901234567890123456789012");
    SafeUrl::encode(
        xorname,
        None,
        0x00a5_3cde,
        SafeDataType::PublicBlob,
        SafeContentType::Raw,
        None,
        None,
        None,
        None,
        Some(5),
        XorUrlBase::Base32,
    )
    .map_err(|e| anyhow!("Failed to encode URL: {}", e))
}

#[test]
fn calling_safe_nrs_create_pretty() -> Result<()> {
    let test_name = format!("safe://{}", get_random_nrs_string());
    let fake_target = gen_fake_target()?;
    let mut cmd = Command::cargo_bin(CLI).map_err(|e| anyhow!(e.to_string()))?;
    cmd.args(&vec!["nrs", "create", &test_name, "-l", &fake_target])
        .assert()
        .stdout(predicate::str::contains(PRETTY_NRS_CREATION_RESPONSE))
        .stdout(predicate::str::contains(SAFE_PROTOCOL).count(4))
        .stdout(predicate::str::contains(fake_target).count(1))
        .stdout(predicate::str::contains("+").count(1))
        .success();
    Ok(())
}

#[test]
fn calling_safe_nrs_twice_w_name_fails() -> Result<()> {
    let test_name = format!("safe://{}", get_random_nrs_string());
    let fake_target = gen_fake_target()?;

    let _nrs_creation = cmd!(
        env!("CARGO_BIN_EXE_safe"),
        "nrs",
        "create",
        &test_name,
        "-l",
        &fake_target,
        "--json"
    )
    .read()
    .map_err(|e| anyhow!(e.to_string()))?;

    let mut cmd = Command::cargo_bin(CLI).map_err(|e| anyhow!(e.to_string()))?;
    cmd.args(&vec!["nrs", "create", &test_name, "-l", &fake_target])
        .assert()
        .stderr(predicate::str::contains(
            "NRS name already exists. Please use 'nrs add' command to add sub names to it",
        ))
        .failure();
    Ok(())
}

#[test]
fn calling_safe_nrs_put_folder_and_fetch() -> Result<()> {
    let test_name = format!("safe://{}", get_random_nrs_string());

    let (container_xorurl, _map) = upload_test_folder(true)?;

    let cat_of_filesmap = cmd!(env!("CARGO_BIN_EXE_safe"), "cat", &container_xorurl)
        .read()
        .map_err(|e| anyhow!(e.to_string()))?;

    assert!(cat_of_filesmap.contains("safe://"));

    let nrs_creation = cmd!(
        env!("CARGO_BIN_EXE_safe"),
        "nrs",
        "create",
        &test_name,
        "-l",
        &container_xorurl,
        "--json"
    )
    .read()
    .map_err(|e| anyhow!(e.to_string()))?;

    let (nrs_map_xorurl, _change_map) = parse_nrs_create_output(&nrs_creation);

    assert!(nrs_map_xorurl.contains("safe://"));
    let cat_of_nrs_map_url = cmd!(env!("CARGO_BIN_EXE_safe"), "cat", &nrs_map_xorurl)
        .read()
        .map_err(|e| anyhow!(e.to_string()))?;

    // does our resolvable map exist?
    assert!(cat_of_nrs_map_url.contains("safe://"));
    assert!(cat_of_nrs_map_url.contains("another.md"));
    assert!(cat_of_nrs_map_url.contains("Files of FilesContainer (version 0)"));

    assert!(nrs_creation.contains("safe://"));
    assert!(nrs_creation.contains('+'));
    assert!(nrs_creation.contains(&test_name));

    let another_file = format!("{}/another.md", &test_name);
    let cat_of_new_url = cmd!(env!("CARGO_BIN_EXE_safe"), "cat", &another_file)
        .read()
        .map_err(|e| anyhow!(e.to_string()))?;

    assert_eq!(cat_of_new_url, "exists");
    Ok(())
}

#[test]
fn calling_safe_nrs_put_no_top_default_fetch() -> Result<()> {
    let nrs_name = get_random_nrs_string();
    let test_name1 = format!("safe://a.b.c.{}", nrs_name);
    let test_name2 = format!("safe://b.c.{}", nrs_name);

    let (container_xorurl, _map) = upload_test_folder(true)?;
    let mut safeurl = safeurl_from(&container_xorurl)?;
    safeurl.set_path("/test.md");
    let link = safeurl.to_string();
    let _nrs_creation = cmd!(
        env!("CARGO_BIN_EXE_safe"),
        "nrs",
        "create",
        &test_name1,
        "-l",
        &link,
        "--json"
    )
    .read()
    .map_err(|e| anyhow!(e.to_string()))?;

    let cat_of_new_url = cmd!(env!("CARGO_BIN_EXE_safe"), "cat", &test_name1)
        .read()
        .map_err(|e| anyhow!(e.to_string()))?;
    assert_eq!(cat_of_new_url, "hello tests!");

    safeurl.set_path("/another.md");
    let link2 = safeurl.to_string();
    let _nrs_addition = cmd!(
        env!("CARGO_BIN_EXE_safe"),
        "nrs",
        "add",
        &test_name2,
        "-l",
        &link2,
        "--json"
    )
    .read()
    .map_err(|e| anyhow!(e.to_string()))?;

    let cat_of_new_url = cmd!(env!("CARGO_BIN_EXE_safe"), "cat", &test_name2)
        .read()
        .map_err(|e| anyhow!(e.to_string()))?;
    assert_eq!(cat_of_new_url, "exists");
    Ok(())
}

#[test]
fn calling_safe_nrs_put_folder_and_fetch_from_subname() -> Result<()> {
    let (container_xorurl, _map) = upload_test_folder(true)?;

    let test_name = get_random_nrs_string();
    let test_name_w_sub = format!("safe://subname.{}", &test_name);

    let cat_of_filesmap = cmd!(env!("CARGO_BIN_EXE_safe"), "cat", &container_xorurl)
        .read()
        .map_err(|e| anyhow!(e.to_string()))?;
    assert!(cat_of_filesmap.contains("safe://"));

    let nrs_creation = cmd!(
        env!("CARGO_BIN_EXE_safe"),
        "nrs",
        "create",
        &test_name_w_sub,
        "-l",
        &container_xorurl,
        "--json"
    )
    .read()
    .map_err(|e| anyhow!(e.to_string()))?;

    let (nrs_map_xorurl, _change_map) = parse_nrs_create_output(&nrs_creation);

    assert!(nrs_map_xorurl.contains("safe://"));
    let cat_of_nrs_map_url = cmd!(env!("CARGO_BIN_EXE_safe"), "cat", &nrs_map_xorurl)
        .read()
        .map_err(|e| anyhow!(e.to_string()))?;

    // does our resolvable map exist?
    assert!(cat_of_nrs_map_url.contains("safe://"));
    assert!(cat_of_nrs_map_url.contains("another.md"));
    assert!(cat_of_nrs_map_url.contains("Files of FilesContainer (version 0)"));

    assert!(nrs_creation.contains("safe://"));
    assert!(nrs_creation.contains("subname"));
    assert!(nrs_creation.contains('+'));
    assert!(nrs_creation.contains(&test_name_w_sub));

    let another_file = format!("{}/another.md", &test_name_w_sub);
    let cat_of_new_url = cmd!(env!("CARGO_BIN_EXE_safe"), "cat", &another_file)
        .read()
        .map_err(|e| anyhow!(e.to_string()))?;

    assert_eq!(cat_of_new_url, "exists");

    let via_default_also = cmd!(
        env!("CARGO_BIN_EXE_safe"),
        "cat",
        format!("safe://{}/another.md", &test_name)
    )
    .read()
    .map_err(|e| anyhow!(e.to_string()))?;

    assert_eq!(via_default_also, "exists");
    Ok(())
}

#[test]
fn calling_safe_nrs_put_and_retrieve_many_subnames() -> Result<()> {
    let (container_xorurl, _map) = upload_test_folder(true)?;

    let test_name = get_random_nrs_string();
    let test_name_w_sub = format!("safe://a.b.{}", &test_name);

    let cat_of_filesmap = cmd!(env!("CARGO_BIN_EXE_safe"), "cat", &container_xorurl)
        .read()
        .map_err(|e| anyhow!(e.to_string()))?;
    assert!(cat_of_filesmap.contains("safe://"));

    let nrs_creation = cmd!(
        env!("CARGO_BIN_EXE_safe"),
        "nrs",
        "create",
        &test_name_w_sub,
        "-l",
        &container_xorurl,
        "--json"
    )
    .read()
    .map_err(|e| anyhow!(e.to_string()))?;

    let (nrs_map_xorurl, _change_map) = parse_nrs_create_output(&nrs_creation);

    assert!(nrs_map_xorurl.contains("safe://"));
    let cat_of_nrs_map_url = cmd!(env!("CARGO_BIN_EXE_safe"), "cat", &nrs_map_xorurl)
        .read()
        .map_err(|e| anyhow!(e.to_string()))?;

    // does our resolvable map exist?
    assert!(cat_of_nrs_map_url.contains("safe://"));
    assert!(cat_of_nrs_map_url.contains("another.md"));
    assert!(cat_of_nrs_map_url.contains("Files of FilesContainer (version 0)"));

    assert!(nrs_creation.contains("safe://"));
    assert!(nrs_creation.contains("a.b"));
    assert!(nrs_creation.contains('+'));
    assert!(nrs_creation.contains(&test_name_w_sub));

    let another_file = format!("{}/another.md", &test_name_w_sub);
    let cat_of_new_url = cmd!(env!("CARGO_BIN_EXE_safe"), "cat", &another_file)
        .read()
        .map_err(|e| anyhow!(e.to_string()))?;

    assert_eq!(cat_of_new_url, "exists");

    let via_default_from_root = cmd!(
        env!("CARGO_BIN_EXE_safe"),
        "cat",
        format!("safe://{}/another.md", &test_name)
    )
    .read()
    .map_err(|e| anyhow!(e.to_string()))?;

    assert_eq!(via_default_from_root, "exists");
    Ok(())
}

#[test]
fn calling_safe_nrs_put_and_add_new_subnames_set_default_and_retrieve() -> Result<()> {
    let (_container_xorurl, file_map) = upload_test_folder(true)?;

    let test_name = get_random_nrs_string();
    let test_name_w_sub = format!("safe://a.b.{}", &test_name);
    let test_name_w_new_sub = format!("safe://x.b.{}", &test_name);

    let (_a_sign, another_md_xor) = &file_map["../testdata/another.md"];
    let (_t_sign, test_md_xor) = &file_map["../testdata/test.md"];

    let cat_of_another_raw = cmd!(env!("CARGO_BIN_EXE_safe"), "cat", &another_md_xor)
        .read()
        .map_err(|e| anyhow!(e.to_string()))?;

    assert_eq!(cat_of_another_raw, "exists");

    let _file_one_nrs_creation = cmd!(
        env!("CARGO_BIN_EXE_safe"),
        "nrs",
        "create",
        &test_name_w_sub,
        "-l",
        &another_md_xor,
        "--json"
    )
    .read()
    .map_err(|e| anyhow!(e.to_string()))?;

    let cat_of_sub_one = cmd!(env!("CARGO_BIN_EXE_safe"), "cat", &test_name_w_sub)
        .read()
        .map_err(|e| anyhow!(e.to_string()))?;

    assert_eq!(cat_of_sub_one, "exists");

    let first_default = cmd!(
        env!("CARGO_BIN_EXE_safe"),
        "cat",
        format!("safe://{}", test_name)
    )
    .read()
    .map_err(|e| anyhow!(e.to_string()))?;

    assert_eq!(first_default, "exists");

    let _new_nrs_creation = cmd!(
        env!("CARGO_BIN_EXE_safe"),
        "nrs",
        "add",
        &test_name_w_new_sub,
        "-l",
        &test_md_xor,
        "--json",
        "--default"
    )
    .read()
    .map_err(|e| anyhow!(e.to_string()))?;

    let new_nrs_creation_cat = cmd!(env!("CARGO_BIN_EXE_safe"), "cat", &test_name_w_new_sub)
        .read()
        .map_err(|e| anyhow!(e.to_string()))?;

    assert_eq!(new_nrs_creation_cat, "hello tests!");

    let new_default = cmd!(
        env!("CARGO_BIN_EXE_safe"),
        "cat",
        format!("safe://{}", test_name)
    )
    .read()
    .map_err(|e| anyhow!(e.to_string()))?;

    assert_eq!(new_default, "hello tests!");
    Ok(())
}

#[test]
fn calling_safe_nrs_put_and_add_new_subnames_remove_one_and_retrieve() -> Result<()> {
    let (_container_xorurl, file_map) = upload_test_folder(true)?;

    let test_name = get_random_nrs_string();
    let test_name_w_sub = format!("safe://a.b.{}", &test_name);
    let test_name_w_new_sub = format!("safe://x.b.{}", &test_name);

    let (_a_sign, another_md_xor) = &file_map["../testdata/another.md"];
    let (_t_sign, test_md_xor) = &file_map["../testdata/test.md"];

    let cat_of_another_raw = cmd!(env!("CARGO_BIN_EXE_safe"), "cat", &another_md_xor)
        .read()
        .map_err(|e| anyhow!(e.to_string()))?;

    assert_eq!(cat_of_another_raw, "exists");

    let _file_one_nrs_creation = cmd!(
        env!("CARGO_BIN_EXE_safe"),
        "nrs",
        "create",
        &test_name_w_sub,
        "-l",
        &another_md_xor,
        "--json"
    )
    .read()
    .map_err(|e| anyhow!(e.to_string()))?;

    let _new_nrs_creation = cmd!(
        env!("CARGO_BIN_EXE_safe"),
        "nrs",
        "add",
        &test_name_w_new_sub,
        "-l",
        &test_md_xor,
        "--json",
        "--default",
    )
    .read()
    .map_err(|e| anyhow!(e.to_string()))?;

    let _remove_one_nrs = cmd!(
        env!("CARGO_BIN_EXE_safe"),
        "nrs",
        "remove",
        &test_name_w_sub,
        "--json",
    )
    .read()
    .map_err(|e| anyhow!(e.to_string()))?;

    let new_nrs_creation_cat = cmd!(env!("CARGO_BIN_EXE_safe"), "cat", &test_name_w_new_sub)
        .read()
        .map_err(|e| anyhow!(e.to_string()))?;

    assert_eq!(new_nrs_creation_cat, "hello tests!");

    let new_default = cmd!(
        env!("CARGO_BIN_EXE_safe"),
        "cat",
        format!("safe://{}", test_name)
    )
    .read()
    .map_err(|e| anyhow!(e.to_string()))?;

    assert_eq!(new_default, "hello tests!");
    Ok(())
}

#[test]
fn calling_safe_nrs_put_and_add_new_subnames_remove_one_and_so_fail_to_retrieve() -> Result<()> {
    let (_container_xorurl, file_map) = upload_test_folder(true)?;

    let test_name = get_random_nrs_string();
    let test_name_w_sub = format!("safe://a.b.{}", &test_name);
    let test_name_w_new_sub = format!("safe://x.b.{}", &test_name);

    let (_a_sign, another_md_xor) = &file_map["../testdata/another.md"];
    let (_t_sign, test_md_xor) = &file_map["../testdata/test.md"];

    let cat_of_another_raw = cmd!(env!("CARGO_BIN_EXE_safe"), "cat", &another_md_xor)
        .read()
        .map_err(|e| anyhow!(e.to_string()))?;

    assert_eq!(cat_of_another_raw, "exists");

    let _file_one_nrs_creation = cmd!(
        env!("CARGO_BIN_EXE_safe"),
        "nrs",
        "create",
        &test_name_w_sub,
        "-l",
        &another_md_xor,
        "--json"
    )
    .read()
    .map_err(|e| anyhow!(e.to_string()))?;

    let _new_nrs_creation = cmd!(
        env!("CARGO_BIN_EXE_safe"),
        "nrs",
        "add",
        &test_name_w_new_sub,
        "-l",
        &test_md_xor,
        "--json"
    )
    .read()
    .map_err(|e| anyhow!(e.to_string()))?;

    let new_nrs_creation_cat = cmd!(env!("CARGO_BIN_EXE_safe"), "cat", &test_name_w_new_sub)
        .read()
        .map_err(|e| anyhow!(e.to_string()))?;

    assert_eq!(new_nrs_creation_cat, "hello tests!");

    let safe_default = cmd!(
        env!("CARGO_BIN_EXE_safe"),
        "cat",
        format!("safe://{}", test_name)
    )
    .read()
    .map_err(|e| anyhow!(e.to_string()))?;

    assert_eq!(safe_default, "exists");

    let remove_one_nrs = cmd!(
        env!("CARGO_BIN_EXE_safe"),
        "nrs",
        "remove",
        &test_name_w_sub,
        "--json",
    )
    .read()
    .map_err(|e| anyhow!(e.to_string()))?;

    assert!(remove_one_nrs.contains('-'));
    assert!(remove_one_nrs.contains(&test_name_w_sub));

    let mut invalid_cat = Command::cargo_bin(CLI).map_err(|e| anyhow!(e.to_string()))?;
    invalid_cat
        .args(&vec!["cat", &test_name_w_sub])
        .assert()
        .stderr(predicate::str::contains(
            "Sub name not found in NRS Map Container",
        ))
        .failure();
    Ok(())
}
