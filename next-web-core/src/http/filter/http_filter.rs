use std::any::Any;

use async_trait::async_trait;
use dyn_clone::{DynClone, clone_trait_object};

use crate::{
    Named,
    http::{
        HttpRequest, HttpResponse,
        filter::{FilterError, HttpFilterChain},
    },
};

#[async_trait]
pub trait HttpFilter
where
    Self: Send + Sync,
    Self: Any + DynClone,
    Self: Named,
{
    async fn do_filter(
        &self,
        request: &mut dyn HttpRequest,
        response: &mut dyn HttpResponse,
        filter_chain: &dyn HttpFilterChain,
    ) -> Result<(), FilterError>;

    #[allow(unused_variables)]
    fn supports(&self, name: &str) -> bool {
        false
    }

    /// Applies the login URL of the application to this filter.
    ///
    /// The URL of the login page is a property of the application, not of each
    /// filter that answers it: an application configures it once, and every
    /// filter that has a login page takes it from there.
    ///
    /// The default keeps the URL the filter was built with and reports `false`.
    /// A filter that has a login page returns `true` — and keeps a login URL the
    /// application set on the filter itself.
    ///
    /// # Arguments
    ///
    /// * `login_url` - The path of the login page of the application.
    fn apply_login_url(&mut self, _login_url: &str) -> bool {
        false
    }

    /// Applies the success URL of the application to this filter.
    ///
    /// A filter that answers a successful login sends the client on to the page
    /// the application configured for it; that page is a property of the
    /// application, not of the filter.
    ///
    /// The default keeps the URL the filter was built with and reports `false`.
    ///
    /// # Arguments
    ///
    /// * `success_url` - The path of the page a successful login returns to.
    fn apply_success_url(&mut self, _success_url: &str) -> bool {
        false
    }

    /// Applies the unauthorized URL of the application to this filter.
    ///
    /// A filter that refuses a request answers with the page the application
    /// configured for a refused request, or with 401 when the application
    /// configured none.
    ///
    /// The default keeps the URL the filter was built with and reports `false`.
    ///
    /// # Arguments
    ///
    /// * `unauthorized_url` - The path of the page a refused request returns to.
    fn apply_unauthorized_url(&mut self, _unauthorized_url: &str) -> bool {
        false
    }

    /// Renames this filter.
    ///
    /// A filter chain resolves a filter by the name it reports from
    /// [`Named::name`], and a filter that is registered under another name is
    /// renamed with this method. The default implementation ignores the call: a
    /// filter that cannot be renamed keeps the name it was built with.
    ///
    /// # Arguments
    ///
    /// * `name` - The name the filter reports from now on.
    #[allow(unused_variables)]
    fn set_name(&mut self, name: &str) {}
}

clone_trait_object!(HttpFilter);
