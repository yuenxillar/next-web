//! The [`HttpFilterChain`] contract.

use std::any::Any;

use async_trait::async_trait;
use dyn_clone::DynClone;

use crate::http::{filter::FilterError, http_request::HttpRequest, http_response::HttpResponse};

/// Continues the processing of a filter chain.
///
/// A filter receives the chain as `&dyn HttpFilterChain` and calls
/// [`HttpFilterChain::do_filter`] to hand control to the next filter; returning
/// without calling it stops the chain.
///
/// Implementations are `Send + Sync + 'static` and clonable through
/// [`DynClone`], so a filter can take an owned handle to the remainder of the
/// chain. This is what lets [`CompositeFilter`] insert its own delegates in front
/// of the chain it received while keeping a `'static` trait object.
///
/// Because the method takes `&self`, an implementation that tracks a position
/// (such as [`ApplicationFilterChain`] or a composite's virtual chain) must keep
/// that position in interior mutable state shared by its clones.
///
/// [`CompositeFilter`]: crate::filter::CompositeFilter
/// [`ApplicationFilterChain`]: crate::filter::application_filter_chain::ApplicationFilterChain
#[async_trait]
pub trait HttpFilterChain
where
    Self: Send + Sync,
    Self: Any + DynClone,
{
    /// Runs the rest of the chain.
    ///
    /// # Errors
    ///
    /// Returns the [`FilterError`] produced by the next filter or by the
    /// terminal handler.
    async fn do_filter(
        &self,
        request: &mut dyn HttpRequest,
        response: &mut dyn HttpResponse,
    ) -> Result<(), FilterError>;
}

dyn_clone::clone_trait_object!(HttpFilterChain);
