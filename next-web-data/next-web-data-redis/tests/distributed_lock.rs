#![cfg(feature = "distributed-lock")]

use std::time::{Duration, Instant};
use std::{
    io::BufRead,
    process::{Command, Stdio},
};

use next_web_data_redis::{RedisLockConfig, RedisLockService, UnlockOutcome};

fn test_service() -> Option<RedisLockService> {
    test_service_with(RedisLockConfig::default())
}

fn test_service_with(config: RedisLockConfig) -> Option<RedisLockService> {
    let url = std::env::var("NEXT_WEB_TEST_REDIS_URL").ok()?;
    let template = next_web_data_redis::core::redis_template::RedisTemplate::with_url(url).ok()?;

    RedisLockService::from_template(&template, config).ok()
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

    let first = lock.try_lock().await.unwrap().unwrap();
    let second = lock.clone().try_lock().await.unwrap().unwrap();
    assert_eq!(lock.hold_count().await.unwrap(), 2);
    assert!(other_owner.try_lock().await.unwrap().is_none());
    assert_eq!(second.unlock().await.unwrap(), UnlockOutcome::Reentrant);
    assert_eq!(first.unlock().await.unwrap(), UnlockOutcome::Released);

    let acquired = other_owner.try_lock().await.unwrap().unwrap();
    acquired.unlock().await.unwrap();
}

#[tokio::test]
#[ignore = "requires NEXT_WEB_TEST_REDIS_URL"]
async fn a_blocking_lock_excludes_the_async_lock() {
    let service = test_service().expect("NEXT_WEB_TEST_REDIS_URL must contain a Redis URL");
    let name = format!("next-web:test-blocking:{}", uuid::Uuid::new_v4());

    let blocking = service.get_blocking_lock(&name);
    let guard = tokio::task::spawn_blocking(move || blocking.try_lock())
        .await
        .unwrap()
        .unwrap()
        .unwrap();

    assert!(service.get_lock(&name).try_lock().await.unwrap().is_none());

    tokio::task::spawn_blocking(move || guard.unlock())
        .await
        .unwrap()
        .unwrap();
}

#[tokio::test]
#[ignore = "requires NEXT_WEB_TEST_REDIS_URL"]
async fn every_script_matches_the_in_memory_double() {
    let service = test_service().expect("NEXT_WEB_TEST_REDIS_URL must contain a Redis URL");
    let name = format!("next-web:test-scripts:{}", uuid::Uuid::new_v4());
    let lock = service.get_lock(&name);
    let contender = service.get_lock(&name);
    let intruder = service.get_lock(&name);

    let guard = lock.try_lock().await.unwrap().unwrap();
    assert!(intruder.try_lock().await.unwrap().is_none());
    assert!(lock.is_held_by_owner().await.unwrap());
    assert!(lock.remaining_lease().await.unwrap().is_some());

    // force_unlock is owner-agnostic and notifies the channel.
    assert!(intruder.force_unlock().await.unwrap());
    assert!(!lock.is_locked().await.unwrap());
    // The owner's level is gone, so releasing it is refused.
    assert!(guard.unlock().await.is_err());

    let acquired = contender.try_lock().await.unwrap().unwrap();
    acquired.unlock().await.unwrap();
}

#[tokio::test]
#[ignore = "requires NEXT_WEB_TEST_REDIS_URL"]
async fn an_explicit_lease_expires_without_a_watchdog() {
    let service = test_service().expect("NEXT_WEB_TEST_REDIS_URL must contain a Redis URL");
    let name = format!("next-web:test-lease:{}", uuid::Uuid::new_v4());

    let guard = service
        .get_lock(&name)
        .try_lock_for_with_lease(Duration::from_secs(2), Duration::from_millis(150))
        .await
        .unwrap()
        .unwrap();

    tokio::time::sleep(Duration::from_millis(250)).await;

    let contender = service.get_lock(&name).try_lock().await.unwrap().unwrap();
    contender.unlock().await.unwrap();

    // A fixed lease has no watchdog, so nothing was "lost": the field is simply
    // gone by the time the guard tries to release it.
    assert!(matches!(
        guard.unlock().await,
        Err(next_web_data_redis::RedisLockError::NotOwner)
    ));
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

    let service = test_service().expect("NEXT_WEB_TEST_REDIS_URL must contain a Redis URL");
    let lock = service.get_lock(&name);
    assert!(lock.try_lock().await.unwrap().is_none());
    assert!(child.wait().unwrap().success());

    let guard = lock
        .try_lock_for(Duration::from_secs(1))
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

    let service = RedisLockService::from_cluster(urls, RedisLockConfig::default())
        .expect("invalid cluster configuration");
    let lock = service.get_lock(format!("next-web:cluster:{{{}}}", uuid::Uuid::new_v4()));
    let guard = lock
        .try_lock_for(Duration::from_secs(2))
        .await
        .unwrap()
        .unwrap();

    assert_eq!(guard.unlock().await.unwrap(), UnlockOutcome::Released);
}

#[tokio::test]
#[ignore = "requires at least three NEXT_WEB_TEST_REDLOCK_URLS"]
async fn redlock_reaches_quorum_and_releases_all_nodes() {
    let urls = urls_from_env("NEXT_WEB_TEST_REDLOCK_URLS");
    assert!(
        urls.len() >= 3,
        "at least three Redis URLs must be configured"
    );

    let nodes = urls
        .iter()
        .map(|url| {
            next_web_data_redis::core::redis_template::RedisTemplate::with_url(url)
                .map(std::sync::Arc::new)
        })
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    let service = RedisLockService::new(nodes, RedisLockConfig::default()).unwrap();

    let lock = service
        .get_red_lock(format!("next-web:redlock:{}", uuid::Uuid::new_v4()))
        .unwrap();
    assert_eq!(lock.quorum(), 2);
    assert_eq!(lock.node_count(), urls.len());

    let guard = lock
        .try_lock_for(Duration::from_secs(2))
        .await
        .unwrap()
        .unwrap();
    // Every node answered, so every node granted a level.
    assert_eq!(guard.node_count(), urls.len());
    assert!(!guard.is_lost());

    // Releasing has to reach a quorum, which means the keys are gone on a
    // majority of the deployments rather than just on the first one.
    assert_eq!(guard.unlock().await.unwrap(), UnlockOutcome::Released);

    let again = lock
        .try_lock_for(Duration::from_secs(2))
        .await
        .unwrap()
        .unwrap();
    again.unlock().await.unwrap();
}

#[tokio::test]
#[ignore = "requires NEXT_WEB_TEST_REDIS_URL"]
async fn a_watchdog_keeps_a_short_lease_alive() {
    // A lease that would be long gone after three lifetimes, on a real server:
    // only an actual renewal can keep the key alive this long.
    let config = RedisLockConfig {
        watchdog_timeout: Duration::from_secs(3),
        command_timeout: Duration::from_secs(1),
        ..RedisLockConfig::default()
    };
    let service =
        test_service_with(config).expect("NEXT_WEB_TEST_REDIS_URL must contain a Redis URL");
    let name = format!("next-web:test-watchdog:{}", uuid::Uuid::new_v4());
    let lock = service.get_lock(&name);

    let guard = lock.lock().await.unwrap();
    tokio::time::sleep(Duration::from_millis(3500)).await;

    assert!(
        !guard.is_lost(),
        "the watchdog lost a lease it should be renewing"
    );
    assert!(lock.is_held_by_owner().await.unwrap());
    assert!(lock.remaining_lease().await.unwrap().is_some());
    assert!(
        service.get_lock(&name).try_lock().await.unwrap().is_none(),
        "another owner acquired a lock the watchdog should still hold"
    );

    guard.unlock().await.unwrap();
}

#[tokio::test]
#[ignore = "requires NEXT_WEB_TEST_REDIS_URL"]
async fn a_waiter_is_woken_by_the_release_notification() {
    // A two-second retry interval makes polling unable to explain a fast
    // acquisition: only the Pub/Sub notification can.
    let config = RedisLockConfig {
        retry_interval: Duration::from_secs(2),
        ..RedisLockConfig::default()
    };
    let service =
        test_service_with(config).expect("NEXT_WEB_TEST_REDIS_URL must contain a Redis URL");
    let name = format!("next-web:test-wakeup:{}", uuid::Uuid::new_v4());
    let holder = service.get_lock(&name);
    let waiter = service.get_lock(&name);

    let guard = holder.try_lock().await.unwrap().unwrap();
    let release = tokio::spawn(async move {
        tokio::time::sleep(Duration::from_millis(300)).await;
        guard.unlock().await.unwrap();
    });

    let started = Instant::now();
    let acquired = waiter.try_lock_for(Duration::from_secs(10)).await.unwrap();
    let waited = started.elapsed();

    assert!(acquired.is_some());
    assert!(
        waited < Duration::from_millis(1500),
        "waited {waited:?} despite a release at 300ms and a 2s retry interval"
    );

    acquired.unwrap().unlock().await.unwrap();
    release.await.unwrap();
}
