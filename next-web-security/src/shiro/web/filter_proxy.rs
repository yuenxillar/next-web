use std::sync::Arc;

use crate::{
    core::subject::{support::default_subject_context::DefaultSubjectContext, Subject},
    web::{
        filter::mgt::path_matching_filter_chain_resolver::PathMatchingFilterChainResolver,
        filter_proxy_configure::FilterProxyConfigure,
        mgt::{
            default_web_security_manager::DefaultWebSecurityManager,
            web_security_manager::WebSecurityManager,
        },
    },
};

use indexmap::IndexMap;
use next_web_core::{
    async_trait,
    error::BoxError,
    traits::{
        filter::{http_filter::HttpFilter, http_filter_chain::HttpFilterChain},
        http::{http_request::HttpRequest, http_response::HttpResponse},
        nameable::Nameable,
    },
};
use tracing::error;

#[derive(Clone)]
pub struct FilterProxy {
    security_manager: Arc<dyn WebSecurityManager>,
    filter_chain_resolver: Option<PathMatchingFilterChainResolver>,
}

impl FilterProxy {
    pub fn new<S>(security_manager: S, configure: FilterProxyConfigure) -> Self
    where
        S: WebSecurityManager + 'static,
    {
        let mut proxy = Self {
            security_manager: Arc::new(security_manager),
            filter_chain_resolver: Default::default(),
        };
        let manager = configure.create_filter_chain_manager();
        let filter_chain_resolver = PathMatchingFilterChainResolver::new(manager);

        proxy.filter_chain_resolver = Some(filter_chain_resolver);
        proxy
    }

    pub fn get_security_manager(&self) -> &Arc<dyn WebSecurityManager> {
        &self.security_manager
    }

    pub async fn create_subject(
        &self,
        req: &mut dyn HttpRequest,
        resp: &mut dyn HttpResponse,
    ) -> Box<dyn Subject> {
        self.security_manager
            .create_subject(
                Arc::new(DefaultSubjectContext::from_security_manager(
                    self.security_manager.clone(),
                )),
                req,
                resp,
            )
            .await
    }

    pub fn get_filter_chain_resolver(&self) -> Option<&PathMatchingFilterChainResolver> {
        self.filter_chain_resolver.as_ref()
    }
    pub fn get_execution_chain(
        &self,
        req: &dyn HttpRequest,
        res: &dyn HttpResponse,
        orig_chain: &dyn HttpFilterChain,
    ) -> Option<Box<dyn HttpFilterChain>> {
        let resolver = self.get_filter_chain_resolver();
        if resolver.is_none() {
            return None;
        }

        let resolver = match resolver {
            Some(resolver) => resolver,
            None => return None,
        };

        resolver.get_chain(req, res, orig_chain)
    }

    #[allow(unused_variables)]
    pub fn update_session_last_access_time(
        &self,
        req: &dyn HttpRequest,
        res: &dyn HttpResponse,
        subject: &mut dyn Subject,
    ) {
        let session = subject.get_session();
        if let Some(session) = session {
            if let Err(err) = session.touch() {
                error!(
                    "session.touch() method invocation has failed.  Unable to update the corresponding session's last access time based on the incoming request. error: {:?}",
                    err
                )
            }
        }
    }
    pub async fn execute_chain<'a>(
        &'a self,
        req: &mut dyn HttpRequest,
        res: &mut dyn HttpResponse,
        orig_chain: &'a dyn HttpFilterChain,
    ) -> Result<(), BoxError> {
        match self.get_execution_chain(req, res, orig_chain) {
            Some(chain) => chain.do_filter(req, res).await,
            None => orig_chain.do_filter(req, res).await,
        }
    }
}

impl Nameable for FilterProxy {
    fn name(&self) -> &str {
        "FilterProxy"
    }

    fn set_name(&mut self, _name: &str) {}
}

#[async_trait]
impl HttpFilter for FilterProxy {
    async fn do_filter(
        &self,
        req: &mut dyn HttpRequest,
        resp: &mut dyn HttpResponse,
        orig_chain: &dyn HttpFilterChain,
    ) -> Result<(), BoxError> {
        req.ready();

        let mut subject = self.create_subject(req, resp).await;

        self.update_session_last_access_time(req, resp, subject.as_mut());
        self.execute_chain(req, resp, orig_chain).await?;

        req.clean_up();
        Ok(())
    }
}

impl Default for FilterProxy {
    fn default() -> Self {
        let mut configure = FilterProxyConfigure::default();

        let filter_chain_definition_map = IndexMap::from_iter([
            ("/login.jsp".into(), "anon".into()),
            ("/**".into(), "authc".into()),
        ]);

        configure.set_filter_chain_definition_map(filter_chain_definition_map);

        let security_manager = DefaultWebSecurityManager::default();
        Self::new(security_manager, configure)
    }
}
