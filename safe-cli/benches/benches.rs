#![feature(test)]


extern crate safe_utilities;
extern crate test;
#[macro_use]
extern crate duct;

use safe_utilities::{
    get_bin_location
    // create_preload_and_get_keys, get_bin_location, get_random_nrs_string, parse_cat_wallet_output,
    // parse_files_put_or_sync_output, CLI,
};
use assert_cmd::prelude::*;
const TEST_FILE: &str = "../testdata/test.md";

pub fn calling_safe_cat() -> () {
    let content = cmd!(get_bin_location(), "files", "put", TEST_FILE, "--json")
        .read()
        .unwrap();

    // safe_utilities::shared_code();
}

#[cfg(test)]
mod tests {
        use super::*;
        use test::Bencher;
        // use tests::cli_cat::calling_safe_cat;
        // use tests::calling_safe_cat;
        
        #[bench]
        fn bench_safe_cat(b: &mut Bencher) {
            b.iter(|| calling_safe_cat());
        }
    
}