use std::sync::Arc;

use next_web_core::{
    async_trait,
    http::{HttpRequest, HttpResponse},
};

use crate::web::csrf::{CsrfToken, CsrfTokenRepository, DeferredCsrfToken};

/// The token of a request, loaded from its repository when the filter starts.
///
/// The reference implementation loads the token lazily, on the first `get()`, and
/// keeps the live request and response so that `saveToken` can store a token it
/// had to generate. This port cannot keep them: the filter needs the request
/// itself for the rest of its turn, and a Rust borrow cannot be held across it.
/// So the work of `init()` — load, and generate and save when there is none — is
/// done once, when the token is loaded, with the live request and response in
/// hand. The filter reads a token from the first moment it processes a request,
/// so the difference is invisible to it.
#[derive(Clone)]
pub struct RepositoryDeferredCsrfToken {
    csrf_token: Arc<dyn CsrfToken>,
    /// Whether the token was generated for this request rather than loaded.
    ///
    /// A token that was generated is the one the client is told about for the
    /// first time, and the one a submission cannot have answered yet.
    generated: bool,
}

impl RepositoryDeferredCsrfToken {
    /// Loads the token of a request, generating and storing one when it has none.
    ///
    /// This is the `init()` of the reference implementation: the token of the
    /// session is loaded; when the repository has none, a token is generated and
    /// saved, which is what creates the session a form submission answers to.
    pub async fn load(
        csrf_token_repository: Arc<dyn CsrfTokenRepository>,
        request: &mut dyn HttpRequest,
        response: &mut dyn HttpResponse,
    ) -> Self {
        let loaded = csrf_token_repository.load_token(request).await;
        let (csrf_token, generated) = match loaded {
            Some(csrf_token) => (csrf_token, false),
            None => {
                let csrf_token = csrf_token_repository.generate_token(request).await;
                if let Err(error) = csrf_token_repository
                    .save_token(Some(&csrf_token), request, response)
                    .await
                {
                    tracing::error!("the CSRF token of the request could not be saved: {error}");
                }

                (csrf_token, true)
            }
        };

        Self {
            csrf_token,
            generated,
        }
    }
}

#[async_trait]
impl DeferredCsrfToken for RepositoryDeferredCsrfToken {
    async fn token(&mut self) -> Arc<dyn CsrfToken> {
        self.csrf_token.clone()
    }

    async fn is_generated(&mut self) -> bool {
        self.generated
    }
}
