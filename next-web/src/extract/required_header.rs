use std::marker::PhantomData;

use axum::{
    extract::FromRequestParts,
    http::{HeaderName, request::Parts},
};
use reqwest::StatusCode;

/// An extractor that reads the value of a required HTTP header.
///
/// The header is identified by the marker type `T`, which implements
/// [`ToHeaderName`]. If the header is missing, empty, or not valid UTF-8,
/// the request is rejected with `400 Bad Request`.
///
/// `T` is only used at the type level (via [`PhantomData`]) to distinguish
/// extractors for different headers; it carries no runtime data.
#[derive(Debug, Clone)]
pub struct RequiredHeader<T>(String, PhantomData<T>);

impl<T> RequiredHeader<T> {
    /// Create an instance
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into(), PhantomData)
    }
    /// Returns the header value as a string slice.
    pub fn value(&self) -> &str {
        &self.0
    }
}

/// Maps a marker type to the HTTP header it represents.
///
/// Implement this trait for zero-sized types to use them with
/// [`RequiredHeader`].
pub trait ToHeaderName {
    /// Returns the header name this type corresponds to.
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

        match parts
            .headers
            .get(&name)
            .and_then(|value| value.to_str().ok())
            .filter(|s| !s.is_empty())
            .map(String::from)
        {
            Some(s) => Ok(RequiredHeader(s, PhantomData)),
            None => {
                #[cfg(feature = "trace-log")]
                tracing::debug!("Required header {} not found in request", name.as_str());
                Err((StatusCode::BAD_REQUEST, "Bad Request"))
            }
        }
    }
}

/// Generates zero-sized marker types and their [`ToHeaderName`] impls.
///
/// Each entry maps a type name to a constant in [`axum::http::header`].
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
        (
            AccessControlAllowCredentials,
            ACCESS_CONTROL_ALLOW_CREDENTIALS
        ),
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
        (
            ContentSecurityPolicyReportOnly,
            CONTENT_SECURITY_POLICY_REPORT_ONLY
        ),
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
