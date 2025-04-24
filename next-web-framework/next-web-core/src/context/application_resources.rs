

use rust_embed_for_web::RustEmbed;


/// Resource files that need to be embedded in binary files
#[derive(RustEmbed)]
#[folder = "resources/"]
#[include = "*.html"]
#[include = "*.register"]
#[include = "*.yaml"]
#[include = "*.json"]
#[include = "*.properties"]
pub struct ApplicationResources;