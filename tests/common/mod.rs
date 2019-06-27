use std::env;

pub fn get_bin_location() -> String {
    let target_dir = match env::var("CARGO_TARGET_DIR") {
        Ok(target_dir) => target_dir,
        Err(e) => "./target".to_string()
    };
    if cfg!(debug_assertions) {
        return format!("{}{}", target_dir, "/debug/safe_cli");
    }
    format!("{}{}", target_dir, "/release/safe_cli")
}

pub fn create_preload_and_get_keys(preload: &str) -> (String, String) {
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
    let pk_xor_line = lines.next().unwrap();
    let pk_xor_eq = String::from("pk xor=");
    let pk_xor = &pk_xor_line[pk_xor_eq.chars().count()..];
    let _pk = lines.next().unwrap();
    let sk_line = lines.next().unwrap();
    let sk_eq = String::from("sk=");
    let sk = &sk_line[sk_eq.chars().count()..];

    (pk_xor.to_string(), sk.to_string())
}

pub fn create_wallet_with_balance(preload: &str) -> (String, String, String) {
    let (pk, sk) = create_preload_and_get_keys(&preload);
    let wallet_create_result = cmd!(
        get_bin_location(),
        "wallet",
        "create",
        &pk,
        &pk,
        "--secret-key",
        &sk
    )
    .read()
    .unwrap();

    let mut lines = wallet_create_result.lines().rev();
    let wallet_xor = lines.next().unwrap();

    (wallet_xor.to_string(), pk, sk)
}
