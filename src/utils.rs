pub fn get_path_timestamp(key_path: &str) -> i64 {
    let pos = key_path.rfind('@').unwrap();
    key_path[pos + 1..].parse().unwrap()
}

pub fn get_colink_home() -> Result<String, String> {
    let colink_home = if std::env::var("COLINK_HOME").is_ok() {
        std::env::var("COLINK_HOME").unwrap()
    } else if std::env::var("HOME").is_ok() {
        std::env::var("HOME").unwrap() + "/.colink"
    } else {
        return Err("colink home not found.".to_string());
    };
    Ok(colink_home)
}
