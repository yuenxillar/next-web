//! The event system of the application.
//!
//! An application publishes an event through its
//! [`ApplicationEventPublisher`](crate::ApplicationEventPublisher), which is the
//! context, and the multicaster of the context hands the event to the
//! [`ApplicationListener`]s that support it. How a listener receives the event is
//! the decision of the listener, and it is made when the listener is registered:
//!
//! - a listener that is dispatched [inline](ListenerDispatch::Inline) runs as part
//!   of the call that publishes the event, so the call returns once the listener
//!   has run. This is the default, and it is what a listener the request depends
//!   on needs: the session registry of an application has to know a session was
//!   destroyed before the next request is authorized;
//! - a listener that is dispatched over a
//!   [channel](ListenerDispatch::Channel) has the event queued for a worker of its
//!   own, one event after the other, and the call that publishes the event returns
//!   without waiting for it. This is what a listener that is slow and that the
//!   request does not depend on needs: a notification, a webhook, an audit trail
//!   that is written somewhere else.
//!
//! The two kinds are dispatched by the same call: the events of the channel
//! listeners are queued first, and the inline listeners run afterwards, so a
//! listener of one kind never waits for a listener of the other.
//!
//! # What a channel dispatched listener guarantees
//!
//! - **Order**: the events of one listener are delivered in the order they were
//!   published, and they are delivered one at a time.
//! - **Bounded queue**: the queue holds a fixed number of events. The publishing
//!   call never waits for room: an event that does not fit is refused, reported to
//!   the error handler of the multicaster, counted in
//!   [`ChannelStatistics::rejected`], and reported as a failure of the publishing
//!   call when the failure policy is
//!   [`Propagate`](ListenerFailurePolicy::Propagate).
//! - **No lost shutdown**: the context closes the multicaster before it closes
//!   itself, and the multicaster drains the queue of every worker before the
//!   worker stops, with a bound of [`DRAIN_TIMEOUT`]. An application that has to
//!   know the queues are empty awaits
//!   [`DefaultApplicationEventMulticaster::flush_async`].
//! - **Isolation**: a listener that panics is reported to the error handler, the
//!   remaining events are still delivered, and the listeners of the other queues
//!   are not affected.
//!
//! What a channel dispatched listener does not guarantee is what the inline
//! dispatch does: it does not run before the publishing call returns, it does not
//! see the security context or the request of the caller, and its failure cannot
//! fail the request that published the event.

mod application_event_multicaster;
mod application_listener_registration;
mod channel_dispatcher;
mod default_application_event_multicaster;
mod execution;
mod listener_dispatch;
mod typed_application_listener;

#[cfg(test)]
mod channel_dispatch_tests;

#[cfg(test)]
mod event_metadata_tests;

pub use application_event_multicaster::{
    ApplicationEventMulticaster, ErasedApplicationListener, ListenerFailure, MulticastError,
};
pub use application_listener_registration::ApplicationListenerRegistration;
pub use channel_dispatcher::DRAIN_TIMEOUT;
pub use default_application_event_multicaster::{
    DefaultApplicationEventMulticaster, DispatchMode, ListenerFailurePolicy,
    LoggingMulticastErrorHandler, MulticastErrorHandler,
};
pub use listener_dispatch::{
    ChannelDispatch, ChannelStatistics, ChannelStatisticsSnapshot, ListenerDispatch, WhenFull,
};
pub use typed_application_listener::{
    TypedApplicationListener, downcast_event, typed_listener,
};
