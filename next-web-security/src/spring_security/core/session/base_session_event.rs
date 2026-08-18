use next_web_context::EventAttributes;
use next_web_core::AnyObject;

/// Base  for all session related events.
#[derive(Clone)]
pub struct BaseSessionEvent(EventAttributes<AnyObject>);

impl BaseSessionEvent {
    pub fn new(source: AnyObject) -> Self {
        Self(EventAttributes::new(source))
    }
}
