pub fn manifest() -> String {
    std::env::var("CARGO_MANIFEST_DIR").unwrap_or_default()
}

pub fn resources() -> String {
    manifest() + "/resources"
}

pub fn message_source() -> String {
    resources() + "/messages"
}
