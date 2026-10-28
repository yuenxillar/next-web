use std::sync::{Arc, Mutex};

use crate::{
    core::subject::{support::default_subject_context::DefaultSubjectContext, Subject},
    web::{
        filter::mgt::path_matching_filter_chain_resolver::PathMatchingFilterChainResolver,
        filter_proxy_configure::FilterProxyConfigure,
        subject::web_subject::WebSubject,
        mgt::{
            default_web_security_manager::DefaultWebSecurityManager,
            web_security_manager::WebSecurityManager,
        },
    },
};

use next_web_core::util::indexmap::IndexMap;
use next_web_core::{
    async_trait,
    http::{
        filter::{FilterError, HttpFilter, HttpFilterChain},
        HttpRequest, HttpResponse, HttpResponseShare,
    },
    Named,
};

use crate::core::util::nameable::Nameable;
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
    ) -> Box<dyn WebSubject> {
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

    pub async fn update_session_last_access_time(&self, subject: &mut dyn Subject) {
        let session = subject.get_session();
        if let Some(session) = session {
            if let Err(err) = session.touch().await {
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
    ) -> Result<(), FilterError> {
        match self.get_execution_chain(req, res, orig_chain) {
            Some(chain) => chain.do_filter(req, res).await,
            None => orig_chain.do_filter(req, res).await,
        }
    }
}

impl Named for FilterProxy {
    fn name(&self) -> &str {
        "FilterProxy"
    }
}

impl Nameable for FilterProxy {
    fn set_name(&mut self, _name: &str) {}
}

#[async_trait]
impl HttpFilter for FilterProxy {
    async fn do_filter(
        &self,
        req: &mut dyn HttpRequest,
        resp: &mut dyn HttpResponse,
        orig_chain: &dyn HttpFilterChain,
    ) -> Result<(), FilterError> {
        req.ready();

        let mut subject = self.create_subject(req, resp).await;

        // The subject serves this call: it is given the request and a response of
        // its own, shared by `Arc`, so a filter that works through the subject
        // writes the headers of this response 鈥?the session id cookie above all 鈥?        // and they reach the client when the chain is done.
        let call_response = Arc::new(Mutex::new(HttpResponseShare::default()));
        subject.attach_shared_context(Arc::new(req.shared().clone()), call_response.clone());

        self.update_session_last_access_time(subject.as_mut()).await;

        // The filters of the chain reach the subject of the call through the
        // request: the binding is what makes `roles`, `perms` and `user` check the
        // subject that is actually serving this request.
        req.set_attribute(
            crate::core::util::web::WebUtils::SHARED_SUBJECT_KEY,
            next_web_core::any::AnyValue::BoxedValue(Box::new(Arc::<dyn WebSubject>::from(subject))),
        );

        let chain_result = self.execute_chain(req, resp, orig_chain).await;
        req.remove_attribute(crate::core::util::web::WebUtils::SHARED_SUBJECT_KEY);
        chain_result?;

        if let Ok(share) = call_response.lock() {
            for (name, value) in share.header_map().iter() {
                if let Ok(value) = value.to_str() {
                    resp.append_header(name.as_str(), value);
                }
            }
        }

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
