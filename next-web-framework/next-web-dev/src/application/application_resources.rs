

use rust_embed_for_web::RustEmbed;


/// Resource files that need to be embedded in binary files
#[derive(RustEmbed)]
#[folder = "resources/"]
#[include = "*.html"]
#[include = "*.properties"]
#[include = "*.yaml"]
#[include = "*.json"]
pub struct ApplicationResources;