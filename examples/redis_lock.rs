//! A runnable check of the distributed lock, against a real Redis.
//!
//! Unlike the crate's `#[ignore]` integration tests, this is a program: it walks
//! through every capability of `next_web_data_redis::RedisLockService`, prints
//! what each step observed, and exits with a non-zero status on the first check
//! that does not hold. Run it when you want to see the lock behave, not just a
//! green check.
//!
//! ```text
//! REDIS_URL=redis://:password@127.0.0.1:6379/0 \
//! cargo run --manifest-path examples/Cargo.toml --bin redis_lock
//! ```
//!
//! Set `REDLOCK_URLS` to three or more comma-separated URLs to include the
//! quorum lock; it is skipped otherwise.

use std::{
    error::Error,
    process::ExitCode,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

use next_web_data_redis::{
    RedisLockConfig, RedisLockError, RedisLockService, UnlockOutcome,
    core::redis_template::RedisTemplate,
};

type Result<T> = std::result::Result<T, Box<dyn Error>>;

#[tokio::main]
async fn main() -> ExitCode {
    match run().await {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("\nredis_lock failed: {error}");
            eprintln!(
                "check that REDIS_URL points at a reachable Redis and that its password is right"
            );
            ExitCode::FAILURE
        }
    }
}

async fn run() -> Result<()> {
    let url = std::env::var("REDIS_URL").unwrap_or_else(|_| "redis://127.0.0.1:6379/0".to_string());
    let template = RedisTemplate::with_url(&url)?;
    let service = RedisLockService::from_template(&template, RedisLockConfig::default())?;

    println!("redis      : {url}");
    println!("client id  : {}", service.client_id());
    println!("nodes      : {}", service.node_count());

    mutual_exclusion(&service).await?;
    reentrancy(&service).await?;
    watchdog(&template, &service).await?;
    fixed_lease(&service).await?;
    blocking_flavour(&service).await?;
    wakeup(&template).await?;
    force_unlock(&service).await?;
    scoped_operation(&service).await?;
    quorum(&url).await?;

    println!("\nevery check passed");
    Ok(())
}

/// Two handles for one name must exclude each other.
async fn mutual_exclusion(service: &RedisLockService) -> Result<()> {
    let name = unique("mutual");
    let owner = service.get_lock(&name);
    let rival = service.get_lock(&name);

    let guard = owner
        .try_lock()
        .await?
        .expect("a fresh lock must be acquireable");
    assert!(
        rival.try_lock().await?.is_none(),
        "a second owner acquired a lock that is already held"
    );
    assert!(rival.is_locked().await?, "the lock should be visible");
    assert!(
        !rival.is_held_by_owner().await?,
        "the rival must not be recognised as the owner"
    );

    let outcome = guard.unlock().await?;
    assert_eq!(outcome, UnlockOutcome::Released);
    assert!(!owner.is_locked().await?, "the lock should be gone");

    let reacquired = rival
        .try_lock()
        .await?
        .expect("the rival must get the lock once it is released");
    reacquired.unlock().await?;

    println!("mutual     : ok, only one owner at a time");
    Ok(())
}

/// A clone of one handle shares its owner token, so it re-enters.
async fn reentrancy(service: &RedisLockService) -> Result<()> {
    let name = unique("reentrant");
    let lock = service.get_lock(&name);

    let first = lock.try_lock().await?.expect("first level");
    let second = lock
        .clone()
        .try_lock()
        .await?
        .expect("a clone must re-enter");
    assert_eq!(first.owner_id(), second.owner_id());

    let depth = lock.hold_count().await?;
    assert_eq!(depth, 2, "the hold count should be the level count");
    assert!(first.lease() > Duration::ZERO);

    assert_eq!(second.unlock().await?, UnlockOutcome::Reentrant);
    assert!(
        lock.is_locked().await?,
        "the lock must survive the inner release"
    );
    assert_eq!(lock.hold_count().await?, 1);
    assert_eq!(first.unlock().await?, UnlockOutcome::Released);
    assert!(!lock.is_locked().await?);

    println!("reentrancy : ok, depth 2 then back to 0");
    Ok(())
}

/// A watchdog lock outlives its own lease.
async fn watchdog(template: &RedisTemplate, service: &RedisLockService) -> Result<()> {
    // `command_timeout` has to fit inside one renewal interval, and the
    // interval is a third of the watchdog timeout.
    let config = RedisLockConfig {
        watchdog_timeout: Duration::from_secs(3),
        command_timeout: Duration::from_secs(1),
        ..RedisLockConfig::default()
    };
    let watchdog_service = RedisLockService::from_template(template, config)?;
    let name = unique("watchdog");
    let lock = watchdog_service.get_lock(&name);

    let guard = lock.lock().await?;
    let lease = guard.lease();
    let held = lease + lease / 2;
    tokio::time::sleep(held).await;

    assert!(!guard.is_lost(), "the watchdog lost its lease");
    assert!(lock.is_held_by_owner().await?);
    assert!(
        lock.remaining_lease().await?.is_some(),
        "a renewed lease still has a TTL"
    );
    assert!(
        service.get_lock(&name).try_lock().await?.is_none(),
        "another owner took a lock the watchdog should still hold"
    );

    guard.unlock().await?;
    println!("watchdog   : ok, held {held:?} on a {lease:?} lease without expiring");
    Ok(())
}

/// A fixed lease expires on its own, and the stale guard is then refused.
async fn fixed_lease(service: &RedisLockService) -> Result<()> {
    let name = unique("lease");
    let lock = service.get_lock(&name);

    let guard = lock
        .try_lock_for_with_lease(Duration::from_secs(2), Duration::from_millis(300))
        .await?
        .expect("the lock is free");
    tokio::time::sleep(Duration::from_millis(600)).await;

    let rival = service.get_lock(&name);
    let stolen = rival
        .try_lock()
        .await?
        .expect("an expired lease must free the lock");
    stolen.unlock().await?;

    match guard.unlock().await {
        Err(RedisLockError::NotOwner) => {}
        other => {
            return Err(format!("expected NotOwner for an expired lease, got {other:?}").into());
        }
    }

    println!("lease      : ok, 300ms lease expired and the stale guard was refused");
    Ok(())
}

/// The blocking flavour reaches the same Redis state from a blocking thread.
async fn blocking_flavour(service: &RedisLockService) -> Result<()> {
    let name = unique("blocking");
    let blocking = service.get_blocking_lock(&name);

    // The blocking API must not be called from an async task, so it runs on a
    // blocking thread, which is also what a real caller would do.
    let guard = tokio::task::spawn_blocking(move || blocking.try_lock())
        .await??
        .expect("the lock is free");

    assert!(
        service.get_lock(&name).try_lock().await?.is_none(),
        "the async flavour acquired a lock the blocking flavour holds"
    );

    let released = tokio::task::spawn_blocking(move || guard.unlock()).await??;
    assert_eq!(released, UnlockOutcome::Released);

    // Bound to a name and released: dropping a guard only stops the watchdog,
    // it leaves the lease to expire, which is the trap this example avoids.
    let after_release = service
        .get_lock(&name)
        .try_lock()
        .await?
        .expect("the lock should be free after a blocking release");
    after_release.unlock().await?;

    println!("blocking   : ok, excludes the async flavour and releases");
    Ok(())
}

/// A waiter is woken by the release, not only by its retry interval.
async fn wakeup(template: &RedisTemplate) -> Result<()> {
    // A two-second retry interval means polling cannot explain a fast wake-up.
    let config = RedisLockConfig {
        retry_interval: Duration::from_secs(2),
        ..RedisLockConfig::default()
    };
    let service = RedisLockService::from_template(template, config)?;
    let name = unique("wakeup");
    let holder = service.get_lock(&name);
    let waiter = service.get_lock(&name);

    let guard = holder.try_lock().await?.expect("the lock is free");
    let release = tokio::spawn(async move {
        tokio::time::sleep(Duration::from_millis(300)).await;
        guard.unlock().await
    });

    let started = Instant::now();
    let acquired = waiter.try_lock_for(Duration::from_secs(10)).await?;
    let waited = started.elapsed();

    let acquired = acquired.expect("the waiter should get the lock");
    assert!(
        waited < Duration::from_millis(1500),
        "waited {waited:?} despite a release at 300ms and a 2s retry interval"
    );
    acquired.unlock().await?;
    release.await??;

    println!("wakeup     : ok, woke {waited:?} after a release (retry interval 2s)");
    Ok(())
}

/// `force_unlock` is owner-agnostic, and the real owner is told.
async fn force_unlock(service: &RedisLockService) -> Result<()> {
    let name = unique("force");
    let owner = service.get_lock(&name);
    let intruder = service.get_lock(&name);

    let guard = owner.try_lock().await?.expect("the lock is free");
    assert!(!intruder.is_held_by_owner().await?);
    assert!(
        intruder.force_unlock().await?,
        "force_unlock should report that it deleted the lock"
    );
    assert!(!owner.is_locked().await?);

    match guard.unlock().await {
        Err(RedisLockError::NotOwner) => {}
        other => return Err(format!("expected NotOwner after force_unlock, got {other:?}").into()),
    }

    println!("force      : ok, deleted by a stranger and the owner was told");
    Ok(())
}

/// `with_lock` releases even when the operation fails.
async fn scoped_operation(service: &RedisLockService) -> Result<()> {
    let name = unique("scoped");
    let lock = service.get_lock(&name);

    let outcome: std::result::Result<std::result::Result<(), &'static str>, _> =
        lock.with_lock(|| async { Err("operation failed") }).await;
    let inner = outcome?;
    assert_eq!(inner, Err("operation failed"));
    assert!(
        !lock.is_locked().await?,
        "with_lock must release on the error path"
    );

    println!("with_lock  : ok, released after a failed operation");
    Ok(())
}

/// The quorum lock, when three or more independent deployments are configured.
async fn quorum(primary_url: &str) -> Result<()> {
    let raw = match std::env::var("REDLOCK_URLS") {
        Ok(value) => value,
        Err(_) => {
            println!("quorum     : skipped (set REDLOCK_URLS to three comma-separated URLs)");
            return Ok(());
        }
    };
    let urls: Vec<String> = raw
        .split(',')
        .map(str::trim)
        .filter(|url| !url.is_empty())
        .map(str::to_string)
        .collect();

    if urls.len() < 3 {
        println!(
            "quorum     : skipped, REDLOCK_URLS has {} URL(s) and a quorum needs 3",
            urls.len()
        );
        return Ok(());
    }

    let nodes = urls
        .iter()
        .map(|url| RedisTemplate::with_url(url).map(std::sync::Arc::new))
        .collect::<std::result::Result<Vec<_>, _>>()?;
    let service = RedisLockService::new(nodes, RedisLockConfig::default())?;
    let name = unique("quorum");
    let lock = service.get_red_lock(&name)?;

    assert_eq!(lock.quorum(), urls.len() / 2 + 1);
    let guard = lock
        .try_lock_for(Duration::from_secs(2))
        .await?
        .expect("a free quorum lock should be acquireable");
    assert_eq!(guard.node_count(), urls.len());
    assert_eq!(guard.unlock().await?, UnlockOutcome::Released);

    let again = lock
        .try_lock_for(Duration::from_secs(2))
        .await?
        .expect("a released quorum lock should be acquireable again");
    again.unlock().await?;

    println!(
        "quorum     : ok, {} of {} nodes, released on a majority ({primary_url} is one of them)",
        lock.quorum(),
        urls.len()
    );
    Ok(())
}

/// A lock name no other run of this program shares.
fn unique(tag: &str) -> String {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|elapsed| elapsed.as_nanos())
        .unwrap_or_default();

    format!("next-web:example:{tag}:{}:{nanos}", std::process::id())
}
