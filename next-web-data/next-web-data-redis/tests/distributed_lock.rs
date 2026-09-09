#![cfg(feature = "distributed-lock")]

use std::time::Duration;
use std::{
    io::BufRead,
    process::{Command, Stdio},
};

use next_web_data_redis::{
    RedisDistributedLockService, RedisLockConfig, RedisLockMode, UnlockResult,
};

fn test_service() -> Option<RedisDistributedLockService> {
    let url = std::env::var("NEXT_WEB_TEST_REDIS_URL").ok()?;
    RedisDistributedLockService::with_direct(url, RedisLockConfig::default()).ok()
}

fn urls_from_env(name: &str) -> Vec<String> {
    std::env::var(name)
        .unwrap_or_default()
        .split(',')
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
        .collect()
}

#[tokio::test]
#[ignore = "requires NEXT_WEB_TEST_REDIS_URL"]
async fn direct_lock_is_reentrant_and_owner_safe() {
    let service = test_service().expect("NEXT_WEB_TEST_REDIS_URL must contain a Redis URL");
    let name = format!("next-web:test-lock:{}", uuid::Uuid::new_v4());
    let lock = service.get_lock(&name);
    let other_owner = service.get_lock(&name);

    let first = lock.try_acquire().await.unwrap().unwrap();
    let second = lock.clone().try_acquire().await.unwrap().unwrap();
    assert_eq!(lock.hold_count().await.unwrap(), 2);
    assert!(other_owner.try_acquire().await.unwrap().is_none());
    assert_eq!(second.unlock().await.unwrap(), UnlockResult::Reentrant);
    assert_eq!(first.unlock().await.unwrap(), UnlockResult::Released);

    let acquired = other_owner.try_acquire().await.unwrap().unwrap();
    acquired.unlock().await.unwrap();
}

#[tokio::test]
#[ignore = "requires NEXT_WEB_TEST_REDIS_URL"]
async fn explicit_lease_expires_without_watchdog() {
    let service = test_service().expect("NEXT_WEB_TEST_REDIS_URL must contain a Redis URL");
    let name = format!("next-web:test-lease:{}", uuid::Uuid::new_v4());
    let lock = service.get_lock(&name);
    let contender = service.get_lock(&name);

    let guard = lock
        .try_acquire_for_with_lease(Duration::from_millis(50), Duration::from_millis(150))
        .await
        .unwrap()
        .unwrap();
    tokio::time::sleep(Duration::from_millis(250)).await;
    let contender_guard = contender.try_acquire().await.unwrap().unwrap();
    contender_guard.unlock().await.unwrap();
    assert!(guard.unlock().await.is_err());
}

#[tokio::test(flavor = "multi_thread")]
#[ignore = "requires Java, NEXT_WEB_TEST_REDIS_URL, and NEXT_WEB_TEST_REDISSON_CLASSPATH"]
async fn java_redisson_4_7_and_rust_are_mutually_exclusive() {
    let url = std::env::var("NEXT_WEB_TEST_REDIS_URL").unwrap();
    let classpath = std::env::var("NEXT_WEB_TEST_REDISSON_CLASSPATH").unwrap();
    let probe = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/java/RedissonInteropProbe.java"
    );
    let name = format!("next-web:java-interop:{}", uuid::Uuid::new_v4());
    let mut child = Command::new("java")
        .args(["--class-path", &classpath, probe, &url, &name, "500"])
        .stdout(Stdio::piped())
        .spawn()
        .expect("failed to start the Java Redisson probe");
    let mut stdout = std::io::BufReader::new(child.stdout.take().unwrap());
    let mut ready = String::new();
    stdout.read_line(&mut ready).unwrap();
    assert_eq!(ready.trim(), "LOCKED");

    let service =
        RedisDistributedLockService::with_direct(&url, RedisLockConfig::default()).unwrap();
    let lock = service.get_lock(&name);
    assert!(lock.try_acquire().await.unwrap().is_none());
    assert!(child.wait().unwrap().success());

    let guard = lock
        .try_acquire_for(Duration::from_secs(1))
        .await
        .unwrap()
        .unwrap();
    guard.unlock().await.unwrap();
}

#[tokio::test]
#[ignore = "requires NEXT_WEB_TEST_REDIS_CLUSTER_URLS"]
async fn cluster_lock_round_trip() {
    let urls = urls_from_env("NEXT_WEB_TEST_REDIS_CLUSTER_URLS");
    assert!(!urls.is_empty(), "cluster seed URLs must be configured");
    let service = RedisDistributedLockService::with_cluster(urls, RedisLockConfig::default())
        .expect("invalid cluster configuration");
    let lock = service.get_lock(format!("next-web:cluster:{{{}}}", uuid::Uuid::new_v4()));
    let guard = lock
        .try_acquire_for(Duration::from_secs(2))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(guard.unlock().await.unwrap(), UnlockResult::Released);
}

#[tokio::test]
#[ignore = "requires at least three NEXT_WEB_TEST_REDLOCK_URLS"]
async fn redlock_reaches_quorum_and_releases_all_nodes() {
    let urls = urls_from_env("NEXT_WEB_TEST_REDLOCK_URLS");
    assert!(
        urls.len() >= 3,
        "at least three Redis URLs must be configured"
    );
    let primary_url = urls[0].clone();
    let config = RedisLockConfig {
        mode: RedisLockMode::Direct,
        redlock_urls: urls,
        ..RedisLockConfig::default()
    };
    let service = RedisDistributedLockService::with_direct(primary_url, config).unwrap();
    let lock = service
        .get_redlock(format!("next-web:redlock:{}", uuid::Uuid::new_v4()))
        .unwrap();
    let guard = lock
        .try_acquire_for(Duration::from_secs(2))
        .await
        .unwrap()
        .unwrap();
    guard.unlock().await.unwrap();
}
