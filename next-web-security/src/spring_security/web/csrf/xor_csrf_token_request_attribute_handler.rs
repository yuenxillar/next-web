use std::sync::Arc;

use base64::{engine::general_purpose::URL_SAFE, Engine as _};
use next_web_core::{
    async_trait,
    traits::http::{http_request::HttpRequest, http_response::HttpResponse},
};
use rand::rngs::OsRng;
use rand::RngCore;
use tracing::trace;

use crate::web::csrf::{
    CsrfToken, CsrfTokenRequestAttributeHandler, CsrfTokenRequestHandler, CsrfTokenRequestResolver,
    DefaultCsrfToken, DeferredCsrfToken,
};

/// An implementation of the CsrfTokenRequestHandler interface that is capable of
/// masking the value of the CsrfToken on each request and resolving the raw token
/// value from the masked value as either a header or parameter value of the request.
#[derive(Clone, Default)]
pub struct XorCsrfTokenRequestAttributeHandler {
    inner: CsrfTokenRequestAttributeHandler,
}

impl XorCsrfTokenRequestAttributeHandler {
    fn defer_csrf_token_update(&self, csrf_token: Arc<dyn CsrfToken>) -> CachedDeferredCsrfToken {
        CachedDeferredCsrfToken::new(Arc::new(move || {
            let updated_token = Self::create_xored_csrf_token(&mut OsRng, csrf_token.token());
            Arc::new(DefaultCsrfToken::new(
                csrf_token.header_name(),
                csrf_token.parameter_name(),
                updated_token,
            ))
        }))
    }

    /// Attempts to extract the real CSRF token from a masked token.
    ///
    /// # Arguments
    ///
    /// * `actual_token` - The masked token received from the request.
    /// * `token` - The original unmasked token stored in the session.
    ///
    /// # Returns
    ///
    /// The extracted real token value, or None if extraction fails.
    fn get_token_value(actual_token: &str, token: &str) -> Option<String> {
        // Decode Base64-encoded actual token
        let actual_bytes = match URL_SAFE.decode(actual_token) {
            Ok(bytes) => bytes,
            Err(ex) => {
                trace!(
                    "Not returning the CSRF token since it's not Base64-encoded: {}",
                    ex
                );
                return None;
            }
        };

        let token_bytes = token.as_bytes();
        let token_size = token_bytes.len();

        // Check that the decoded length equals token_size * 2
        if actual_bytes.len() != token_size * 2 {
            trace!(
                target: "org.springframework.security.web.csrf",
                "Not returning the CSRF token since its Base64-decoded length ({}) is not equal to ({})",
                actual_bytes.len(),
                token_size * 2
            );
            return None;
        }

        // Extract token and random bytes
        let random_bytes = &actual_bytes[..token_size];
        let xored_csrf = &actual_bytes[token_size..];

        // XOR to retrieve original token
        let csrf_bytes = Self::xor_csrf(random_bytes, xored_csrf);

        String::from_utf8(csrf_bytes).ok()
    }

    /// Creates a masked CSRF token by XORing the original token with random bytes.
    ///
    /// # Arguments
    ///
    /// * `random` - The random number generator to use.
    /// * `token` - The original CSRF token.
    ///
    /// # Returns
    ///
    /// A Base64-encoded masked token.
    fn create_xored_csrf_token(random: &mut dyn RngCore, token: &str) -> String {
        let token_bytes = token.as_bytes();
        let mut random_bytes = vec![0u8; token_bytes.len()];
        random.fill_bytes(&mut random_bytes);

        let xored_bytes = Self::xor_csrf(&random_bytes, token_bytes);

        // Combine random bytes and XORed bytes
        let mut combined_bytes = Vec::with_capacity(token_bytes.len() * 2);
        combined_bytes.extend_from_slice(&random_bytes);
        combined_bytes.extend_from_slice(&xored_bytes);

        URL_SAFE.encode(&combined_bytes)
    }

    /// XORs two byte arrays together.
    ///
    /// # Arguments
    ///
    /// * `random_bytes` - The random bytes to XOR with.
    /// * `csrf_bytes` - The CSRF token bytes.
    ///
    /// # Returns
    ///
    /// The XORed result as a byte vector.
    fn xor_csrf(random_bytes: &[u8], csrf_bytes: &[u8]) -> Vec<u8> {
        assert_eq!(
            random_bytes.len(),
            csrf_bytes.len(),
            "arrays must be equal length"
        );

        let len = csrf_bytes.len();
        let mut xored_csrf = csrf_bytes.to_vec();

        for i in 0..len {
            xored_csrf[i] ^= random_bytes[i];
        }

        xored_csrf
    }
}

#[async_trait]
impl CsrfTokenRequestHandler for XorCsrfTokenRequestAttributeHandler {
    async fn handle(
        &self,
        request: &mut dyn HttpRequest,
        response: &mut dyn HttpResponse,
        deferred_csrf_token: &mut dyn DeferredCsrfToken,
    ) {
        let csrf_token = deferred_csrf_token.token().await;
        let mut updated_csrf_token = self.defer_csrf_token_update(csrf_token);
        self.inner
            .handle(request, response, &mut updated_csrf_token);
    }
}

impl CsrfTokenRequestResolver for XorCsrfTokenRequestAttributeHandler {
    fn resolve_csrf_token_value(
        &self,
        request: &mut dyn HttpRequest,
        csrf_token: &dyn CsrfToken,
    ) -> Option<String> {
        let actual_token = self.inner.resolve_csrf_token_value(request, csrf_token)?;
        Self::get_token_value(actual_token.as_str(), csrf_token.token())
    }
}

/// A supplier that caches the result of the delegate supplier.
///
/// This ensures the CSRF token is only generated once per request,
/// even if the supplier is called multiple times.
struct CachedDeferredCsrfToken {
    csrf_token_supplier: Arc<dyn Fn() -> Arc<dyn CsrfToken> + Send + Sync>,
}

impl CachedDeferredCsrfToken {
    /// Creates a new CachedCsrfTokenSupplier.
    fn new(csrf_token_supplier: Arc<dyn Fn() -> Arc<dyn CsrfToken> + Send + Sync>) -> Self {
        Self {
            csrf_token_supplier,
        }
    }
}

#[async_trait]
impl DeferredCsrfToken for CachedDeferredCsrfToken {
    async fn token(&mut self) -> Arc<dyn CsrfToken> {
        (self.csrf_token_supplier)()
    }

    async fn is_generated(&mut self) -> bool {
        panic!("CachedDeferredCsrfToken is not generated")
    }
}
