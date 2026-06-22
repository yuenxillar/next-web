use crate::core::session::session_information::SessionInformation;

/// An event that is fired when a `SessionInformation` is detected to have expired.
///
/// This event is passed to the `SessionInformationExpiredStrategy` to allow
/// custom handling of expired sessions (e.g. redirect, error response).
pub struct SessionInformationExpiredEvent {
    session_information: SessionInformation,
}

impl SessionInformationExpiredEvent {
    /// Creates a new `SessionInformationExpiredEvent`.
    pub fn new(session_information: SessionInformation) -> Self {
        Self {
            session_information,
        }
    }

    /// Returns the expired `SessionInformation`.
    pub fn get_session_information(&self) -> &SessionInformation {
        &self.session_information
    }
}
