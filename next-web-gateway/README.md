# next-web-gateway

A Spring Cloud Gateway equivalent for Rust, built on [pingora](https://github.com/cloudflare/pingora).
Routes, predicates and filters are declared in YAML; no code changes are needed to add a route.

## Contents

- [Quick start](#quick-start)
- [Configuration reference](#configuration-reference)
- [Predicates](#predicates)
- [Filters](#filters)
- [Operations](#operations)
- [Differences from Spring Cloud Gateway](#differences-from-spring-cloud-gateway)
- [Roadmap](#roadmap)

## Quick start

```bash
# Validate a configuration without starting the server (exit 0 valid, 2 invalid)
cargo run -p next-web-gateway -- --check --config next-web-gateway/application.yaml

# Run it
cargo run -p next-web-gateway -- --config next-web-gateway/application.yaml
```

```bash
# Container: the build context is the workspace root, the Dockerfile lives here.
docker build -f next-web-gateway/Dockerfile -t next-web-gateway .
docker run --rm -p 8080:8080 -p 9001:9001 \
  -v "$PWD/next-web-gateway/application.yaml:/etc/next-web-gateway/application.yaml:ro" \
  next-web-gateway --config /etc/next-web-gateway/application.yaml
```

The proxy listens on `0.0.0.0:8080` and the admin listener on `127.0.0.1:9001` by default.

### Command line

Every flag has an environment variable so the container can be configured without arguments.

| Flag | Environment variable | Default |
|---|---|---|
| `--config <PATH>` | `NEXT_WEB_GATEWAY_CONFIG` | `application.yaml` |
| `--bind <ADDR>` | `NEXT_WEB_GATEWAY_BIND` | `next.gateway.server.bind` |
| `--admin-bind <ADDR>` | `NEXT_WEB_GATEWAY_ADMIN_BIND` | `next.gateway.server.admin.bind` |
| `--log-level <LEVEL>` | `NEXT_WEB_GATEWAY_LOG_LEVEL`, `RUST_LOG` | `next.gateway.logging.level` |
| `--log-format <text|json>` | `NEXT_WEB_GATEWAY_LOG_FORMAT` | `next.gateway.logging.format` |
| `--check` | — | validate and exit |
| `-d`, `--daemon` | — | run in the background |
| `-u`, `--upgrade` | — | upgrade a running instance in place |
| `-t`, `--test` | — | pingora configuration self-test |

Exit codes: `0` success, `1` runtime failure, `2` invalid configuration.

## Configuration reference

The configuration lives under `next.gateway`. A legacy top level `gateway` block is still
accepted with a deprecation warning. Unknown keys are rejected, so a typo can never silently
disable a filter.

```yaml
next:
  gateway:
    server:
      bind: "0.0.0.0:8080"
      threads: 0                  # 0 = let pingora decide
      graceful_shutdown: 30s
      # `keepalive` and `max_connection` are rejected: pingora 0.9 exposes no
      # downstream knob for them, and an accepted no-op key is worse than none.
      admin:
        enabled: true
        bind: "127.0.0.1:9001"
    logging:
      level: info
      format: text                # text | json
      access_log: true
    upstream:
      connect_timeout: 2s
      read_timeout: 30s
      write_timeout: 30s
      idle_timeout: 60s
      health_check:
        enabled: true
        interval: 5s
      tls:
        verify_cert: true
        verify_hostname: true
        # Custom CA bundles are rejected at startup in this build: use the
        # system trust store instead.
        ca_file: null
        insecure_skip_verify: false
    error:
      format: json                # json | text
      request_id_header: X-Request-Id
    trusted_proxies: []           # CIDR list; empty trusts nobody
    breakers:
      main:
        enabled: true
        failure_threshold: 10
        wait_duration_in_open_state: 5s
        half_open_permits: 1
    routes:
      - id: users
        uri: "lb://user-service"
        order: 100                # omitted = i32::MAX
        predicates:
          - Path=/api/users/**
          - Method=GET,POST
        filters:
          - StripPrefix=1
        upstreams:
          - addr: "10.0.0.11:8080"
            weight: 2
          - addr: "10.0.0.12:8080"
            weight: 1
        metadata:
          client:
            connect_timeout: 1s
            read_timeout: 10s
            write_timeout: 10s
```

Durations accept `500ms`, `30s`, `2m` and `1h`. A bare integer is read as milliseconds and logs
a deprecation warning.

`uri` accepts three schemes:

| Scheme | Meaning |
|---|---|
| `http://host:port` | Proxy to a fixed authority over plain HTTP |
| `https://host:port` | Proxy to a fixed authority over TLS |
| `lb://name` | Round robin across the route's `upstreams`, weighted by `weight` |

Validation collects every problem and reports them together with a document path, for example
`routes[1].upstreams[0].addr: 'svc.local:8080' is not a 'IP:port' address`.

## Predicates

All predicates of a route must match.

| Predicate | Arguments | Matches when |
|---|---|---|
| `Path` | one or more ANT patterns | the request path matches. `{name}` captures one segment, `{*name}` captures the rest; captures feed `SetPath` |
| `Method` | one or more HTTP methods | the request method is in the list |
| `Host` | one or more host patterns | the `Host` matches. `*` matches one label, `**` matches zero or more |
| `Header` | `name[, regex]` | any value of the header matches; without a regex the header only has to exist |
| `Cookie` | `name[, regex]` | the first cookie with that name matches |
| `Query` | `name[, regex]` | the first query parameter with that name matches |
| `RemoteAddr` | one or more IPs or CIDRs | the TCP peer is inside one of the ranges |
| `XForwardedRemoteAddr` | one or more IPs or CIDRs | the client resolved from `X-Forwarded-For` is inside one of the ranges |
| `Weight` | `group, weight` | the route wins its share of the group's bucket space |
| `After` | timestamp | now is at or after it |
| `Before` | timestamp | now is before it |
| `Between` | `start, end` or `start/end` | now is inside `[start, end)` |

Timestamps are RFC 3339 and may carry a Spring style zone suffix, for example
`2017-01-20T17:42:47[America/Denver]`. Regular expressions are anchored on both ends, so
`Header=X-Test, abc` does not match `xabc`.

## Filters

Filters run in declaration order, once, in the pass they belong to.

### Request filters

| Filter | Arguments | Effect |
|---|---|---|
| `AddRequestHeader` | `name, value, ...` | appends request headers |
| `AddRequestHeadersIfNotPresent` | `name, value, ...` | inserts request headers that are absent |
| `SetRequestHeader` | `name, value, ...` | replaces request headers |
| `RemoveRequestHeader` | `name, ...` | removes request headers |
| `AddRequestParameter` | `name, value & name, value` | appends query parameters, keeping existing ones verbatim |
| `RemoveRequestParameter` | `name, ...` | removes query parameters |
| `PrefixPath` | `path` | prepends a path prefix |
| `StripPrefix` | `segments` | drops leading path segments |
| `SetPath` | `path[?query]` | replaces the path, substituting `{captures}` |
| `RewritePath` | `regex, replacement` | replaces every match; `$1` refers to capture groups |
| `MapRequestHeader` | `from, to` | copies one header onto another |
| `SetRequestHostHeader` | `host` | replaces the upstream `Host` |
| `PreserveHostHeader` | — | keeps the downstream `Host` instead of using the upstream authority |
| `RequestHeaderSize` | `bytes[, message]` | rejects a request head larger than `bytes` with 431 |
| `CircuitBreaker` | `breaker-id` | binds the route to a breaker defined under `breakers` |
| `RedirectTo` | `status, url` | answers with a redirect instead of proxying |

### Request body filters

| Filter | Arguments | Effect |
|---|---|---|
| `RequestSize` | `bytes` | rejects a body larger than `bytes` with 413, including chunked bodies with no `Content-Length` |
| `RequestRateLimiter` | `requests-per-second` | admits at most that many requests per second per client address and answers 429 with `Retry-After` |

### Response filters

| Filter | Arguments | Effect |
|---|---|---|
| `AddResponseHeader` | `name, value, ...` | appends response headers |
| `SetResponseHeader` | `name, value, ...` | replaces response headers |
| `RemoveResponseHeader` | `name, ...` | removes response headers |
| `RewriteResponseHeader` | `name, regex, replacement` | rewrites every match inside the header value |
| `RewriteLocationResponseHeader` | `mode[, header[, host[, protocols-regex]]]` | rewrites `Location` |
| `DedupeResponseHeader` | `name, ...[, RETAIN_FIRST / RETAIN_LAST / RETAIN_UNIQUE]` | collapses duplicated header values |
| `SetStatus` | `100..=599` | replaces the response status |
| `SecureHeaders` | — | applies the hardening header set and overrides weakened upstream values |
| `LocalResponseCache` | `ttl, max-size-bytes` | serves and stores responses from the in-process cache |

### Response body filter

| Filter | Arguments | Effect |
|---|---|---|
| `RemoveJsonAttributesResponseBody` | `name, ...[, true]` | removes attributes from a JSON response body; `true` removes them at any depth |

## Operations

| Endpoint | Purpose |
|---|---|
| `GET /health/live` | process liveness, always 200 |
| `GET /health/ready` | 200 when the gateway can serve, 503 otherwise |
| `GET /metrics` | Prometheus text exposition |
| `GET /routes` | JSON snapshot of the configured routes |
| `GET /version` | crate name, version and uptime |

The admin listener never runs a route or a filter, so a broken route configuration cannot take
it down. It is bound to `127.0.0.1` by default; binding it to a public address exposes the route
table and metrics to anyone who can reach it.

Metrics:

| Metric | Type | Labels |
|---|---|---|
| `gateway_requests_total` | counter | `route`, `method`, `status` |
| `gateway_request_duration_seconds` | histogram | `route` |
| `gateway_upstream_errors_total` | counter | `route`, `kind` |
| `gateway_circuit_state` | gauge | `breaker` (`0` closed, `1` open, `2` half open) |
| `gateway_inflight_requests` | gauge | — |
| `gateway_local_cache_hits_total`, `gateway_local_cache_misses_total` | counter | — |

One access log line per request:

```
INFO next_web_gateway::proxy::pipeline: access request_id=8bc9... client=Some(203.0.113.9)
  method=GET path=/ping status=200 route=ping upstream=10.0.0.11:8080 latency_ms=6
```

With `logging.format: json` the same fields are emitted as one JSON object per line.

### Error responses

| Situation | Status |
|---|---|
| no route matched | 404 |
| circuit breaker open | 503 (with `Retry-After`) |
| no backend available | 503 |
| upstream connect failure or reset | 502 |
| connect, read, write or TLS handshake timeout | 504 |
| rejected by a filter | what the filter chose (429, 413, 431, ...) |
| internal error | 500 |

The body follows `next.gateway.error.format`:

```json
{"status":503,"error":"Service Unavailable","route":"users","requestId":"8bc9..."}
```

Gateway generated responses also pass through the response filters, so `SecureHeaders` and
`AddResponseHeader` apply to error pages too.

## Differences from Spring Cloud Gateway

- **No global filter chain yet.** Phase 1 runs the forwarding headers, hop-by-hop stripping,
  request id and metrics inside the proxy rather than as ordered global filters.
- **Weights are exact.** The load balancer repeats a backend `weight` times in its rotation, so a
  3:1 configuration really produces 3:1 rather than a statistical approximation. It also means
  there is no active health checking in Phase 1: a dead backend surfaces as 502 and as breaker
  failures instead of being drained.
- **`XForwardedRemoteAddr` arguments are the client ranges**, and the trusted proxy list comes
  from the global `trusted_proxies` setting.
- **`RequestRateLimiter` takes a bare rate.** The key is always the client address; there is no
  `KeyResolver`/`RateLimiter` bean pair to compose.
- **`SaveSession` and `TokenRelay` are rejected at startup** because there is no authentication
  backend to relay a session or a token from.
- **`LocalResponseCache` is opt-in per route** and stores bodies in process memory; it is not
  shared between worker processes. A response is only cached when it is a `200`/`206`/`301` that
  carries neither `Set-Cookie` nor a `Vary` field other than `accept-encoding`; a `no-cache` or
  `no-store` request bypasses the cache entirely.
- **`upstream.tls.ca_file` is rejected at startup.** This build cannot install a custom trust
  anchor, and accepting the key would silently fail every request to a private CA.
- **`insecure_skip_verify` does not work yet**: a self-signed backend fails the handshake even
  with the flag set, so `https://` upstreams only work when the backend chains to the system
  trust store. `tests/tls.rs` reproduces it and the case is marked `#[ignore]` rather than
  dropped.
- **`server.keepalive` and `server.max_connection` are rejected at startup** because pingora 0.9
  exposes no downstream knob for them. `server.threads` and `server.graceful_shutdown` *are*
  applied to pingora's `ServerConf`.

## Roadmap

Phase 2 adds the global filter chain (`X-Forwarded-*`, hop-by-hop, route-to-URL, metrics), `Retry`,
`ModifyRequestBody`, `ModifyResponseBody`, `CacheRequestBody`, `FallbackHeaders`, `CircuitBreaker`
fallbacks, CORS and the actuator endpoints. Phase 3 adds configuration hot reload, gateway side TLS
termination, service discovery and mTLS upstreams.

## Testing

```bash
cargo test -p next-web-gateway
```

The suite has unit tests per module and `tests/` suites that start the real binary as a
subprocess, point it at a mock upstream and assert on the wire: routing, rewrites, limits, load
balancing, error mapping, observability and the CLI. The end-to-end tests pick free ports and
retry a start when a port is taken.
