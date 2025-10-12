use axum::{extract::FromRequestParts, http::{request::Parts, HeaderName}};
use reqwest::StatusCode;


/// 这个最主要的作用是当请求没有指定的请求时,则返回 400
#[derive(Debug, Clone)]
pub struct RequiredHeader<T>(Option<T>);

pub trait ToHeaderName {
    fn to_header_name() -> HeaderName;
}

impl<T, S> FromRequestParts<S> for RequiredHeader<T>
where
    T: ToHeaderName,
    S: Send + Sync,
{
    type Rejection = (StatusCode, &'static str);

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        let name = T::to_header_name();
        let is_exists = parts.headers.contains_key(&name);
        
        if !is_exists {
            #[cfg(feature = "trace-log")]
            tracing::debug!("Required header {} not found in request", name.as_str());
            return Err((StatusCode::BAD_REQUEST, "Bad Request"));
        }

        return Ok(RequiredHeader(None));
    }
}

macro_rules! impl_to_header_name {
    ($(($name:ident, $header_const:ident)),* $(,)?) => {
        $(
            pub struct $name;

            impl crate::extract::ToHeaderName for $name {
                fn to_header_name() -> ::axum::http::header::HeaderName {
                    ::axum::http::header::$header_const
                }
            }
        )*
    };
}

pub mod header_names {
    impl_to_header_name!(
        (Accept, ACCEPT),
        (AcceptCharset, ACCEPT_CHARSET),
        (AcceptEncoding, ACCEPT_ENCODING),
        (AcceptLanguage, ACCEPT_LANGUAGE),
        (AcceptRanges, ACCEPT_RANGES),
        (AccessControlAllowCredentials, ACCESS_CONTROL_ALLOW_CREDENTIALS),
        (AccessControlAllowHeaders, ACCESS_CONTROL_ALLOW_HEADERS),
        (AccessControlAllowMethods, ACCESS_CONTROL_ALLOW_METHODS),
        (AccessControlAllowOrigin, ACCESS_CONTROL_ALLOW_ORIGIN),
        (AccessControlExposeHeaders, ACCESS_CONTROL_EXPOSE_HEADERS),
        (AccessControlMaxAge, ACCESS_CONTROL_MAX_AGE),
        (AccessControlRequestHeaders, ACCESS_CONTROL_REQUEST_HEADERS),
        (AccessControlRequestMethod, ACCESS_CONTROL_REQUEST_METHOD),
        (Age, AGE),
        (Allow, ALLOW),
        (AltSvc, ALT_SVC),
        (Authorization, AUTHORIZATION),
        (CacheControl, CACHE_CONTROL),
        (CacheStatus, CACHE_STATUS),
        (CdnCacheControl, CDN_CACHE_CONTROL),
        (Connection, CONNECTION),
        (ContentDisposition, CONTENT_DISPOSITION),
        (ContentEncoding, CONTENT_ENCODING),
        (ContentLanguage, CONTENT_LANGUAGE),
        (ContentLength, CONTENT_LENGTH),
        (ContentLocation, CONTENT_LOCATION),
        (ContentRange, CONTENT_RANGE),
        (ContentSecurityPolicy, CONTENT_SECURITY_POLICY),
        (ContentSecurityPolicyReportOnly, CONTENT_SECURITY_POLICY_REPORT_ONLY),
        (ContentType, CONTENT_TYPE),
        (Cookie, COOKIE),
        (Date, DATE),
        (Dnt, DNT),
        (ETag, ETAG),
        (Expect, EXPECT),
        (Expires, EXPIRES),
        (Forwarded, FORWARDED),
        (From, FROM),
        (Host, HOST),
        (IfMatch, IF_MATCH),
        (IfModifiedSince, IF_MODIFIED_SINCE),
        (IfNoneMatch, IF_NONE_MATCH),
        (IfRange, IF_RANGE),
        (IfUnmodifiedSince, IF_UNMODIFIED_SINCE),
        (LastModified, LAST_MODIFIED),
        (Link, LINK),
        (Location, LOCATION),
        (MaxForwards, MAX_FORWARDS),
        (Origin, ORIGIN),
        (Pragma, PRAGMA),
        (ProxyAuthenticate, PROXY_AUTHENTICATE),
        (ProxyAuthorization, PROXY_AUTHORIZATION),
        (PublicKeyPins, PUBLIC_KEY_PINS),
        (PublicKeyPinsReportOnly, PUBLIC_KEY_PINS_REPORT_ONLY),
        (Range, RANGE),
        (Referer, REFERER),
        (ReferrerPolicy, REFERRER_POLICY),
        (Refresh, REFRESH),
        (RetryAfter, RETRY_AFTER),
        (SecWebSocketAccept, SEC_WEBSOCKET_ACCEPT),
        (SecWebSocketExtensions, SEC_WEBSOCKET_EXTENSIONS),
        (SecWebSocketKey, SEC_WEBSOCKET_KEY),
        (SecWebSocketProtocol, SEC_WEBSOCKET_PROTOCOL),
        (SecWebSocketVersion, SEC_WEBSOCKET_VERSION),
        (Server, SERVER),
        (SetCookie, SET_COOKIE),
        (StrictTransportSecurity, STRICT_TRANSPORT_SECURITY),
        (TE, TE),
        (Trailer, TRAILER),
        (TransferEncoding, TRANSFER_ENCODING),
        (Upgrade, UPGRADE),
        (UserAgent, USER_AGENT),
        (UpgradeInsecureRequests, UPGRADE_INSECURE_REQUESTS),
        (Vary, VARY),
        (Via, VIA),
        (Warning, WARNING),
        (WWWAuthenticate, WWW_AUTHENTICATE),
        (XContentTypeOptions, X_CONTENT_TYPE_OPTIONS),
        (XDnsPrefetchControl, X_DNS_PREFETCH_CONTROL),
        (XFrameOptions, X_FRAME_OPTIONS),
        (XXssProtection, X_XSS_PROTECTION),
    );
}