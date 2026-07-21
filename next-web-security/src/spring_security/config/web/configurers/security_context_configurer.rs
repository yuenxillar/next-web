use std::{
    ops::{Deref, DerefMut},
    sync::Arc,
};

use crate::{
    config::{
        security_configurer::SecurityConfigurer,
        web::{configurers::BaseHttpConfigurer, http_security_builder::HttpSecurityBuilder},
    },
    web::{
        context::{
            DelegatingSecurityContextRepository, HttpSessionSecurityContextRepository,
            RequestAttributeSecurityContextRepository, SecurityContextHolderFilter,
            SecurityContextRepository,
        },
        default_security_filter_chain::DefaultSecurityFilterChain,
    },
};

#[derive(Clone)]
pub struct SecurityContextConfigurer<H>
where
    H: HttpSecurityBuilder<H>,
{
    require_explicit_save: bool,

    inner: BaseHttpConfigurer<Self, H>,
}

impl<H> SecurityContextConfigurer<H>
where
    H: HttpSecurityBuilder<H>,
{
    /// Specifies the shared `SecurityContextRepository` that is to be used.
    ///
    /// # Arguments
    ///
    /// * `security_context_repository` - the `SecurityContextRepository` to use
    ///
    /// # Returns
    ///
    /// The `SecurityContextConfigurer` for further customizations.
    pub fn security_context_repository(
        &mut self,
        security_context_repository: Arc<dyn SecurityContextRepository>,
        http: &mut H,
    ) -> &mut Self {
        http.set_shared_object(security_context_repository);

        self
    }

    pub fn require_explicit_save(&mut self, require_explicit_save: bool) -> &mut Self {
        self.require_explicit_save = require_explicit_save;

        self
    }

    pub fn is_require_explicit_save(&self) -> bool {
        self.require_explicit_save
    }

    pub(crate) fn get_security_context_repository(
        &self,
        http: &H,
    ) -> Arc<dyn SecurityContextRepository> {
        let security_context_repository = http
            .shared_object::<Arc<dyn SecurityContextRepository>>()
            .map(Clone::clone);

        match security_context_repository {
            Some(repo) => repo,
            None => {
                let repo = Arc::new(DelegatingSecurityContextRepository::new(vec![
                    Arc::new(RequestAttributeSecurityContextRepository::default()),
                    Arc::new(HttpSessionSecurityContextRepository::default()),
                ]));

                repo
            }
        }
    }
}

impl<H> SecurityConfigurer<DefaultSecurityFilterChain, H> for SecurityContextConfigurer<H>
where
    H: HttpSecurityBuilder<H>,
    H: 'static,
{
    fn init(&mut self, _http: &mut H) {}

    fn configure(&mut self, http: &mut H) {
        let security_context_repository = self.get_security_context_repository(http);

        if self.require_explicit_save {
            let mut security_context_holder_filter =
                SecurityContextHolderFilter::new(security_context_repository);
            security_context_holder_filter.set_security_context_holder_strategy(
                self.inner.get_security_context_holder_strategy().to_owned(),
            );

            // let filter = self.post_process(security_context_holder_filter);
            http.add_filter(security_context_holder_filter);
        } else {
            // // Use SecurityContextPersistenceFilter for implicit save mode
            // let mut security_context_filter =
            //     SecurityContextPersistenceFilter::new(security_context_repository);
            // security_context_filter.set_security_context_holder_strategy(
            //     self.inner.get_security_context_holder_strategy().to_owned(),
            // );
            // let session_creation_policy = http
            //     .configurer::<SessionManagementConfigurer<H>>()
            //     .map(|sm| sm.get_session_creation_policy());
            // if session_creation_policy == Some(SessionCreationPolicy::Always) {
            //     security_context_filter.set_force_eager_session_creation(true);

            //     // let force_eager_filter = self.post_process(ForceEagerSessionCreationFilter::new());
            //     http.add_filter(ForceEagerSessionCreationFilter::default());
            // }

            // // let filter = self.post_process(security_context_filter);
            // http.add_filter(security_context_filter);
            unreachable!()
        }
    }
}

impl<H> Default for SecurityContextConfigurer<H>
where
    H: HttpSecurityBuilder<H>,
{
    fn default() -> Self {
        Self {
            require_explicit_save: true,

            inner: Default::default(),
        }
    }
}

impl<H> Deref for SecurityContextConfigurer<H>
where
    H: HttpSecurityBuilder<H>,
{
    type Target = BaseHttpConfigurer<Self, H>;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl<H> DerefMut for SecurityContextConfigurer<H>
where
    H: HttpSecurityBuilder<H>,
{
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}
