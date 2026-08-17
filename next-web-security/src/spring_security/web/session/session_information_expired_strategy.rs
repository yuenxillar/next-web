use next_web_core::error::BoxError;

use crate::web::session::SessionInformationExpiredEvent;

pub trait SessionInformationExpiredStrategy
where
    Self: Send + Sync,
{
    fn on_expired_session_detected(
        &self,
        event: &mut SessionInformationExpiredEvent,
    ) -> Result<(), BoxError>;
}
