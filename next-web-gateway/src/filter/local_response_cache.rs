use std::{
    collections::{HashMap, VecDeque},
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

use bytes::Bytes;
use pingora::http::{HeaderNameVariant, RequestHeader, ResponseHeader};

use crate::{
    application::next_gateway_application::ApplicationContext,
    filter::gateway_filter::GatewayFilter, route::route_service_manager::UpStream,
};

const CACHE_CONTROL: &str = "Cache-Control";

#[derive(Debug, Clone)]
pub struct LocalResponseCacheFilter {
    pub ttl: Duration,
    pub max_size_bytes: usize,
}

impl Default for LocalResponseCacheFilter {
    fn default() -> Self {
        Self {
            ttl: Duration::ZERO,
            max_size_bytes: 0,
        }
    }
}

impl LocalResponseCacheFilter {
    pub fn is_valid(&self) -> bool {
        !self.ttl.is_zero() && self.max_size_bytes > 0
    }
}

impl From<&str> for LocalResponseCacheFilter {
    fn from(value: &str) -> Self {
        let parts: Vec<&str> = value
            .split(',')
            .map(str::trim)
            .filter(|part| !part.is_empty())
            .collect();

        let ttl = parts
            .first()
            .and_then(|value| parse_duration_value(value))
            .unwrap_or(Duration::ZERO);
        let max_size_bytes = parts
            .get(1)
            .and_then(|value| parse_size_value(value))
            .unwrap_or(0);

        Self {
            ttl,
            max_size_bytes,
        }
    }
}

impl GatewayFilter for LocalResponseCacheFilter {
    fn filter(
        &self,
        _ctx: &mut ApplicationContext,
        _upstream: &mut UpStream,
    ) -> pingora::Result<()> {
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct LocalResponseCacheRequestState {
    pub route_id: String,
    pub key: String,
}

#[derive(Debug, Clone)]
pub struct PendingLocalResponseCacheEntry {
    pub route_id: String,
    pub key: String,
    pub ttl: Duration,
    pub max_size_bytes: usize,
    pub status: u16,
    pub headers: Vec<(String, String)>,
}

#[derive(Debug, Clone)]
pub struct CachedResponseEntry {
    pub status: u16,
    pub headers: Vec<(String, String)>,
    pub body: Bytes,
    pub expires_at: Instant,
    pub size: usize,
}

#[derive(Debug, Default)]
pub struct LocalResponseCacheManager {
    routes: Mutex<HashMap<String, RouteResponseCache>>,
}

#[derive(Debug, Default)]
struct RouteResponseCache {
    entries: HashMap<String, CachedResponseEntry>,
    order: VecDeque<String>,
    current_size: usize,
}

impl LocalResponseCacheManager {
    pub fn shared() -> Arc<Self> {
        Arc::new(Self::default())
    }

    pub fn get(&self, route_id: &str, key: &str) -> Option<CachedResponseEntry> {
        let mut routes = self.routes.lock().ok()?;
        let now = Instant::now();
        let bucket = routes.get_mut(route_id)?;

        evict_expired_entries(bucket, now);

        let entry = bucket.entries.get(key)?.clone();
        if entry.expires_at <= now {
            remove_entry(bucket, key);
            return None;
        }

        touch_key(bucket, key);

        Some(entry)
    }

    pub fn put(
        &self,
        route_id: String,
        key: String,
        ttl: Duration,
        max_size_bytes: usize,
        status: u16,
        headers: Vec<(String, String)>,
        body: Bytes,
    ) {
        if ttl.is_zero() || max_size_bytes == 0 {
            return;
        }

        let mut routes = match self.routes.lock() {
            Ok(routes) => routes,
            Err(_) => return,
        };

        let entry = CachedResponseEntry {
            status,
            size: calculate_entry_size(&key, &headers, &body),
            headers,
            body,
            expires_at: Instant::now() + ttl,
        };

        if entry.size > max_size_bytes {
            routes.remove(&route_id);
            return;
        }

        let bucket = routes.entry(route_id.clone()).or_default();

        if bucket.entries.contains_key(&key) {
            remove_entry(bucket, &key);
        }

        bucket.current_size = bucket.current_size.saturating_add(entry.size);
        bucket.entries.insert(key.clone(), entry);
        touch_key(bucket, &key);
        evict_expired_entries(bucket, Instant::now());

        while bucket.current_size > max_size_bytes {
            let Some(oldest_key) = bucket.order.pop_front() else {
                break;
            };
            if let Some(removed) = bucket.entries.remove(&oldest_key) {
                bucket.current_size = bucket.current_size.saturating_sub(removed.size);
            }
        }
    }
}

pub fn serve_from_local_cache_if_present(
    filter: &LocalResponseCacheFilter,
    ctx: &mut ApplicationContext,
    request_header: &RequestHeader,
) -> pingora::Result<()> {
    ctx.local_response_cache_request = None;
    ctx.pending_local_response_cache = None;

    let Some(manager) = ctx.local_response_cache_manager.as_ref() else {
        return Ok(());
    };

    let Some(route_id) = ctx.route_id.clone() else {
        return Ok(());
    };

    if !filter.is_valid() || !is_cacheable_get_request(request_header) {
        return Ok(());
    }

    let request_cache_control = read_cache_control_header(&request_header.headers, "cache-control");
    if request_cache_control.no_store {
        return Ok(());
    }

    let key = build_cache_key(request_header);
    ctx.local_response_cache_request = Some(LocalResponseCacheRequestState {
        route_id: route_id.clone(),
        key: key.clone(),
    });

    let Some(entry) = manager.get(&route_id, &key) else {
        return Ok(());
    };

    let headers = materialize_cached_headers(&entry);
    if request_cache_control.no_cache {
        return ctx.respond_with_empty(304, headers);
    }

    ctx.respond_with_body(entry.status, headers, entry.body)
}

pub fn capture_response_for_local_cache(
    filter: &LocalResponseCacheFilter,
    ctx: &mut ApplicationContext,
    response_header: &mut ResponseHeader,
) {
    ctx.pending_local_response_cache = None;

    if ctx.local_response_cache_manager.is_none() || !filter.is_valid() {
        return;
    }

    let Some(request_state) = ctx.local_response_cache_request.clone() else {
        return;
    };

    if !is_cacheable_status(response_header.status.as_u16()) {
        return;
    }

    let response_cache_control =
        read_cache_control_header(&response_header.headers, "cache-control");
    if response_cache_control.no_store || response_cache_control.private {
        return;
    }

    rewrite_response_max_age(response_header, filter.ttl.as_secs());

    ctx.pending_local_response_cache = Some(PendingLocalResponseCacheEntry {
        route_id: request_state.route_id,
        key: request_state.key,
        ttl: filter.ttl,
        max_size_bytes: filter.max_size_bytes,
        status: response_header.status.as_u16(),
        headers: extract_cacheable_headers(response_header),
    });
}

pub fn store_local_cache_entry(ctx: &mut ApplicationContext, body: Option<&Bytes>) {
    let Some(pending) = ctx.pending_local_response_cache.take() else {
        return;
    };

    let Some(manager) = ctx.local_response_cache_manager.as_ref() else {
        return;
    };

    manager.put(
        pending.route_id,
        pending.key,
        pending.ttl,
        pending.max_size_bytes,
        pending.status,
        pending.headers,
        body.cloned().unwrap_or_default(),
    );
}

fn calculate_entry_size(key: &str, headers: &[(String, String)], body: &Bytes) -> usize {
    key.len()
        + body.len()
        + headers
            .iter()
            .map(|(name, value)| name.len() + value.len() + 32)
            .sum::<usize>()
}

fn remove_entry(bucket: &mut RouteResponseCache, key: &str) {
    if let Some(removed) = bucket.entries.remove(key) {
        bucket.current_size = bucket.current_size.saturating_sub(removed.size);
    }
    bucket.order.retain(|existing| existing != key);
}

fn touch_key(bucket: &mut RouteResponseCache, key: &str) {
    bucket.order.retain(|existing| existing != key);
    bucket.order.push_back(key.to_string());
}

fn evict_expired_entries(bucket: &mut RouteResponseCache, now: Instant) {
    let expired_keys = bucket
        .entries
        .iter()
        .filter(|(_, entry)| entry.expires_at <= now)
        .map(|(key, _)| key.clone())
        .collect::<Vec<_>>();

    for key in expired_keys {
        remove_entry(bucket, &key);
    }
}

fn is_cacheable_get_request(request_header: &RequestHeader) -> bool {
    if request_header.method != pingora::http::Method::GET {
        return false;
    }

    if request_header.headers.contains_key("transfer-encoding") {
        return false;
    }

    match request_header.headers.get("content-length") {
        Some(content_length) => content_length
            .to_str()
            .ok()
            .and_then(|value| value.trim().parse::<u64>().ok())
            .map(|value| value == 0)
            .unwrap_or(false),
        None => true,
    }
}

fn build_cache_key(request_header: &RequestHeader) -> String {
    let path_and_query = request_header
        .uri
        .path_and_query()
        .map(|value| value.as_str())
        .unwrap_or_else(|| request_header.uri.path());

    let range = request_header
        .headers
        .get("range")
        .and_then(|value| value.to_str().ok())
        .unwrap_or("");

    if range.is_empty() {
        path_and_query.to_string()
    } else {
        format!("{path_and_query}|range={range}")
    }
}

fn is_cacheable_status(status: u16) -> bool {
    matches!(status, 200 | 206 | 301)
}

fn extract_cacheable_headers(response_header: &ResponseHeader) -> Vec<(String, String)> {
    let mut headers = Vec::new();
    response_header
        .map(|name, value| {
            let header_name = match name {
                HeaderNameVariant::Case(case_name) => {
                    String::from_utf8_lossy(case_name.as_slice()).into_owned()
                }
                HeaderNameVariant::Titled(name) => name.to_string(),
            };

            if is_hop_by_hop_header(header_name.as_str()) {
                return Ok(());
            }

            headers.push((
                header_name,
                String::from_utf8_lossy(value.as_bytes()).into_owned(),
            ));

            Ok(())
        })
        .ok();
    headers
}

fn is_hop_by_hop_header(name: &str) -> bool {
    matches!(
        name.to_ascii_lowercase().as_str(),
        "connection"
            | "keep-alive"
            | "proxy-authenticate"
            | "proxy-authorization"
            | "te"
            | "trailer"
            | "transfer-encoding"
            | "upgrade"
            | "proxy-connection"
            | "content-length"
    )
}

fn materialize_cached_headers(entry: &CachedResponseEntry) -> Vec<(String, String)> {
    let remaining_seconds = entry
        .expires_at
        .saturating_duration_since(Instant::now())
        .as_secs();

    entry
        .headers
        .iter()
        .map(|(name, value)| {
            if name.eq_ignore_ascii_case(CACHE_CONTROL) {
                (
                    name.clone(),
                    rewrite_cache_control_value(value, remaining_seconds)
                        .unwrap_or_else(|| value.clone()),
                )
            } else {
                (name.clone(), value.clone())
            }
        })
        .collect()
}

fn rewrite_response_max_age(response_header: &mut ResponseHeader, ttl_seconds: u64) {
    let cache_control = response_header
        .headers
        .get_all("cache-control")
        .iter()
        .filter_map(|value| value.to_str().ok())
        .collect::<Vec<_>>();

    if cache_control.is_empty() {
        return;
    }

    let joined = cache_control.join(", ");
    let Some(rewritten) = rewrite_cache_control_value(&joined, ttl_seconds) else {
        return;
    };

    response_header.insert_header(CACHE_CONTROL, rewritten).ok();
}

fn rewrite_cache_control_value(value: &str, seconds: u64) -> Option<String> {
    let mut found = false;
    let directives = value
        .split(',')
        .map(str::trim)
        .filter(|directive| !directive.is_empty())
        .map(|directive| {
            if directive
                .split_once('=')
                .is_some_and(|(name, _)| name.trim().eq_ignore_ascii_case("max-age"))
            {
                found = true;
                format!("max-age={seconds}")
            } else {
                directive.to_string()
            }
        })
        .collect::<Vec<_>>();

    found.then(|| directives.join(", "))
}

#[derive(Debug, Default)]
struct CacheControlDirectives {
    no_cache: bool,
    no_store: bool,
    private: bool,
}

fn read_cache_control_header(
    headers: &pingora::http::HMap,
    header_name: &str,
) -> CacheControlDirectives {
    let combined = headers
        .get_all(header_name)
        .iter()
        .filter_map(|value| value.to_str().ok())
        .collect::<Vec<_>>()
        .join(", ");

    let mut directives = CacheControlDirectives::default();
    for directive in combined.split(',').map(str::trim) {
        if directive.eq_ignore_ascii_case("no-cache") {
            directives.no_cache = true;
        } else if directive.eq_ignore_ascii_case("no-store") {
            directives.no_store = true;
        } else if directive.eq_ignore_ascii_case("private") {
            directives.private = true;
        }
    }
    directives
}

fn parse_duration_value(value: &str) -> Option<Duration> {
    let trimmed = value.trim();
    let split_index = trimmed
        .find(|char: char| !char.is_ascii_digit())
        .unwrap_or(trimmed.len());

    let (number, unit) = trimmed.split_at(split_index);
    let amount = number.parse::<u64>().ok()?;

    match unit.trim().to_ascii_lowercase().as_str() {
        "s" => Some(Duration::from_secs(amount)),
        "m" => Some(Duration::from_secs(amount.saturating_mul(60))),
        "h" => Some(Duration::from_secs(amount.saturating_mul(60 * 60))),
        _ => None,
    }
}

fn parse_size_value(value: &str) -> Option<usize> {
    let trimmed = value.trim();
    let split_index = trimmed
        .find(|char: char| !char.is_ascii_digit())
        .unwrap_or(trimmed.len());

    let (number, unit) = trimmed.split_at(split_index);
    let amount = number.parse::<usize>().ok()?;

    match unit.trim().to_ascii_uppercase().as_str() {
        "KB" => Some(amount.saturating_mul(1024)),
        "MB" => Some(amount.saturating_mul(1024 * 1024)),
        "GB" => Some(amount.saturating_mul(1024 * 1024 * 1024)),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::{
        build_cache_key, materialize_cached_headers, parse_duration_value, parse_size_value,
        rewrite_cache_control_value, CachedResponseEntry, LocalResponseCacheFilter,
        LocalResponseCacheManager,
    };
    use bytes::Bytes;
    use pingora::http::RequestHeader;
    use std::time::{Duration, Instant};

    #[test]
    fn parses_route_cache_arguments() {
        let filter = LocalResponseCacheFilter::from("30m,500MB");

        assert_eq!(filter.ttl, Duration::from_secs(30 * 60));
        assert_eq!(filter.max_size_bytes, 500 * 1024 * 1024);
        assert!(filter.is_valid());
    }

    #[test]
    fn rewrites_max_age_directive_only_when_present() {
        assert_eq!(
            rewrite_cache_control_value("public, max-age=3600, must-revalidate", 120),
            Some("public, max-age=120, must-revalidate".to_string())
        );
        assert_eq!(rewrite_cache_control_value("public, immutable", 120), None);
    }

    #[test]
    fn materializes_remaining_max_age_for_cached_entries() {
        let entry = CachedResponseEntry {
            status: 200,
            headers: vec![
                (
                    "Cache-Control".to_string(),
                    "public, max-age=300".to_string(),
                ),
                ("Content-Type".to_string(), "application/json".to_string()),
            ],
            body: Bytes::from_static(br#"{"ok":true}"#),
            expires_at: Instant::now() + Duration::from_secs(120),
            size: 0,
        };

        let headers = materialize_cached_headers(&entry);
        let cache_control = headers
            .iter()
            .find(|(name, _)| name == "Cache-Control")
            .map(|(_, value)| value.clone())
            .unwrap();

        assert!(cache_control.starts_with("public, max-age="));
        assert_ne!(cache_control, "public, max-age=300");
    }

    #[test]
    fn evicts_oldest_entries_when_bucket_exceeds_max_size() {
        let manager = LocalResponseCacheManager::default();

        manager.put(
            "route-a".to_string(),
            "/one".to_string(),
            Duration::from_secs(60),
            180,
            200,
            vec![("Content-Type".to_string(), "text/plain".to_string())],
            Bytes::from("first-body"),
        );
        manager.put(
            "route-a".to_string(),
            "/two".to_string(),
            Duration::from_secs(60),
            180,
            200,
            vec![("Content-Type".to_string(), "text/plain".to_string())],
            Bytes::from("second-body"),
        );

        assert!(manager.get("route-a", "/one").is_none());
        assert!(manager.get("route-a", "/two").is_some());
    }

    #[test]
    fn cache_key_includes_range_header_when_present() {
        let mut request = RequestHeader::build("GET", b"/resource?x=1", None).unwrap();
        request.insert_header("Range", "bytes=0-99").unwrap();

        assert_eq!(build_cache_key(&request), "/resource?x=1|range=bytes=0-99");
    }

    #[test]
    fn parses_supported_duration_and_size_units() {
        assert_eq!(parse_duration_value("45s"), Some(Duration::from_secs(45)));
        assert_eq!(parse_duration_value("2h"), Some(Duration::from_secs(7200)));
        assert_eq!(parse_size_value("64KB"), Some(64 * 1024));
        assert_eq!(parse_size_value("1GB"), Some(1024 * 1024 * 1024));
    }
}
