#![cfg(feature = "embed-resources")]

use next_web::macros::application::next_application;

#[tokio::main]
#[next_application(resources = "tests/fixtures/embedded/")]
async fn install_resources() {}

#[test]
fn the_application_attribute_embeds_resources() {
    install_resources();

    let loader = next_web::resources::embedded_resource_loader_from::<EmbedResourceLoader>();

    let resource = loader
        .get_resource("hello.txt")
        .expect("the embedded resource is available");

    assert_eq!(
        resource.get_content_as_string().unwrap().trim(),
        "embedded hello"
    );
}
