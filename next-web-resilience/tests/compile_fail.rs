//! The macros must refuse what they cannot guard, and say why.
//!
//! `trybuild` is the usual tool for this, but it is not vendored in this
//! environment and there is no network access to fetch it, so the fixtures are
//! compiled by calling `cargo check` on [`compile_fail`], a crate that is
//! deliberately not a workspace member. The assertions are on the compiler's own
//! output, so a diagnostic that regresses into something unhelpful fails here.
//!
//! Each fixture is one binary under `compile_fail/src/bin`, compiled on its own.
//! They are expected **not** to compile; what is checked is the reason.

use std::{path::PathBuf, process::Command};

/// Compiles one fixture binary, asserting that it fails, and returns what the
/// compiler said.
///
/// `--offline` keeps the check from reaching for the network: every dependency
/// of the fixture is a path into this workspace or a crate already in the local
/// registry cache.
fn check(binary: &str) -> String {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/compile_fail/Cargo.toml");

    let output = Command::new(env!("CARGO"))
        .arg("check")
        .arg("--offline")
        .arg("--quiet")
        .arg("--manifest-path")
        .arg(&manifest)
        .arg("--bin")
        .arg(binary)
        .output()
        .expect("cargo should be runnable");

    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    assert!(
        !output.status.success(),
        "`{binary}` was expected not to compile, but it did"
    );

    stderr
}

#[test]
fn an_async_trait_method_is_diagnosed_by_name() {
    let stderr = check("async_trait_method");

    // The generic "must return Result" message would be baffling here, because
    // the method does return one. The macro recognises the shape `#[async_trait]`
    // leaves behind and explains it instead.
    assert!(stderr.contains("boxed future"), "{stderr}");
    assert!(stderr.contains("async_trait"), "{stderr}");
    assert!(
        stderr.contains("an `async fn` in a trait is native"),
        "{stderr}"
    );
    assert!(
        !stderr.contains("needs the guarded function to return"),
        "the misleading message came back: {stderr}"
    );
}

#[test]
fn a_synchronous_function_is_refused_with_a_reason() {
    let stderr = check("sync_rate_limiter");

    assert!(
        stderr.contains("cannot guard a synchronous function"),
        "{stderr}"
    );
    assert!(stderr.contains("has to be awaited"), "{stderr}");
    // The suggestion has to be there too, not just the refusal.
    assert!(stderr.contains("async fn"), "{stderr}");
}

#[test]
fn a_function_that_does_not_return_a_result_is_refused() {
    let stderr = check("not_a_result");

    assert!(
        stderr.contains("needs the guarded function to return"),
        "{stderr}"
    );
}

#[test]
fn a_missing_conversion_points_at_the_rejection_type() {
    let stderr = check("missing_from");

    // Without a `fallback` the rejection is converted into the function's error,
    // so the compiler names the implementation that is missing.
    assert!(stderr.contains("From"), "{stderr}");
    assert!(stderr.contains("CallNotPermittedError"), "{stderr}");
    assert!(stderr.contains("MyError"), "{stderr}");
}
