use std::any::TypeId;

use crate::event::SmartApplicationListener;

/// Extended variant of the standard ApplicationListener interface, exposing further metadata such as
/// the supported event and source type.
/// As of Spring Framework 4.2, this interface supersedes the Class-based SmartApplicationListener
/// with full handling of generic event types. it formally extends SmartApplicationListener,
/// adapting supportsEventType(Class) to supportsEventType(ResolvableType) with a default method.
pub trait GenericApplicationListener: SmartApplicationListener {
    /// Determine whether this listener actually supports the given event type.
    fn supports_source_type(&self, source_type: TypeId) -> bool;
}
