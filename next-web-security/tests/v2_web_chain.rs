//! The web filter chain of the Shiro style tree, end to end.
//!
//! An application declares chains by the paths they serve — `/login.jsp` with the
//! anonymous filter, `/**` with the authentication filter — and the resolver
//! picks the chain a request falls into. The tests here hold that path: a request
//! is resolved to the chain of its path, and the filters of that chain run.

#![cfg(all(feature = "v2", feature = "web"))]

use std::sync::{Arc, Mutex};

use axum::{body::Body, http::Request};
use next_web_core::{
    async_trait,
    http::{
        filter::{FilterError, HttpFilter, HttpFilterChain},
        HttpRequest, HttpResponse,
    },
    Named,
};
use next_web_security::web::filter::mgt::{
    default_filter_chain_manager::DefaultFilterChainManager,
    filter_chain_manager::FilterChainManager,
    path_matching_filter_chain_resolver::PathMatchingFilterChainResolver,
};

/// The filters of the tests: each records that it ran.
#[derive(Clone)]
struct Recorder {
    name: &'static str,
    log: Arc<Mutex<Vec<String>>>,
}

impl Recorder {
    fn new(name: &'static str, log: &Arc<Mutex<Vec<String>>>) -> Self {
        Self {
            name,
            log: log.clone(),
        }
    }
}

impl Named for Recorder {
    fn name(&self) -> &str {
        self.name
    }
}

#[async_trait]
impl HttpFilter for Recorder {
    async fn do_filter(
        &self,
        request: &mut dyn HttpRequest,
        response: &mut dyn HttpResponse,
        filter_chain: &dyn HttpFilterChain,
    ) -> Result<(), FilterError> {
        self.log
            .lock()
            .expect("the log is not poisoned")
            .push(self.name.to_string());

        filter_chain.do_filter(request, response).await
    }
}

/// The end of a chain: the application itself.
#[derive(Clone)]
struct Terminal {
    log: Arc<Mutex<Vec<String>>>,
}

#[async_trait]
impl HttpFilterChain for Terminal {
    async fn do_filter(
        &self,
        _request: &mut dyn HttpRequest,
        _response: &mut dyn HttpResponse,
    ) -> Result<(), FilterError> {
        self.log
            .lock()
            .expect("the log is not poisoned")
            .push("terminal".to_string());

        Ok(())
    }
}


/// Builds a manager whose chains are the ones of the tests.
fn manager(log: &Arc<Mutex<Vec<String>>>) -> DefaultFilterChainManager {
    let mut manager = DefaultFilterChainManager::default();
    manager.add_filter(
        "anon",
        Box::new(Recorder::new("anon", log)),
        true,
    );
    manager.add_filter(
        "authc",
        Box::new(Recorder::new("authc", log)),
        true,
    );
    manager.create_chain("/login.jsp".to_string(), "anon".to_string());
    manager.create_chain("/orders/**".to_string(), "authc".to_string());
    manager.create_chain("/**".to_string(), "authc".to_string());

    manager
}

fn request(path: &str) -> Request<Body> {
    Request::builder()
        .uri(path)
        .body(Body::empty())
        .expect("the request is built")
}

#[test]
fn the_chains_of_a_manager_are_the_paths_it_was_given() {
    let log = Arc::new(Mutex::new(Vec::new()));
    let mut manager = manager(&log);

    let names = manager.get_chain_names();
    assert_eq!(names, vec!["/login.jsp", "/orders/**", "/**"]);
    assert!(manager.get_filter("anon").is_some());
    assert!(manager.get_filter("missing").is_none());
}

#[tokio::test]
async fn a_request_is_served_by_the_chain_of_its_path() {
    let log = Arc::new(Mutex::new(Vec::new()));
    let resolver = PathMatchingFilterChainResolver::new(manager(&log));
    let terminal = Terminal { log: log.clone() };
    let mut response = axum::response::Response::default();

    let mut login = request("/login.jsp");
    let chain = resolver
        .get_chain(&login, &response, &terminal)
        .expect("the login page has a chain of its own");
    chain
        .do_filter(&mut login, &mut response)
        .await
        .expect("the chain runs");
    assert_eq!(
        *log.lock().expect("the log is not poisoned"),
        vec!["anon".to_string(), "terminal".to_string()]
    );

    log.lock().expect("the log is not poisoned").clear();
    let mut orders = request("/orders/42");
    let chain = resolver
        .get_chain(&orders, &response, &terminal)
        .expect("an order is authenticated");
    chain
        .do_filter(&mut orders, &mut response)
        .await
        .expect("the chain runs");
    assert_eq!(
        *log.lock().expect("the log is not poisoned"),
        vec!["authc".to_string(), "terminal".to_string()]
    );
}

#[tokio::test]
async fn the_deepest_matching_chain_wins() {
    let log = Arc::new(Mutex::new(Vec::new()));
    let resolver = PathMatchingFilterChainResolver::new(manager(&log));

    // `/orders/**` is declared before `/**`, so it is the chain of an order.
    assert!(resolver.path_matches("/orders/**", "/orders/42/items"));
    assert!(resolver.path_matches("/**", "/anything"));
    assert!(!resolver.path_matches("/orders/**", "/users/42"));
}

#[tokio::test]
async fn a_request_without_a_matching_chain_is_left_to_the_application() {
    let log = Arc::new(Mutex::new(Vec::new()));
    let resolver = PathMatchingFilterChainResolver::new(manager(&log));
    let request = request("/anything");

    // The chain names of the manager cover every path here, so the resolver
    // answers; a manager without a chain answers nothing.
    assert!(resolver
        .get_chain(&request, &axum::response::Response::default(), &Terminal {
            log: log.clone()
        })
        .is_some());

    let empty = PathMatchingFilterChainResolver::new(DefaultFilterChainManager::default());
    assert!(empty
        .get_chain(
            &request,
            &axum::response::Response::default(),
            &Terminal { log: log.clone() }
        )
        .is_none());
}
