/// Build Application
fn main() {
    // When build.rs changes itself, rerun it
    println!("cargo:rerun-if-changed=build.rs");
}
