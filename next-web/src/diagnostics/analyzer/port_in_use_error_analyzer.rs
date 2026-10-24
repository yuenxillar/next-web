use std::io;

use next_web_core::constants::application_constants::APPLICATION_DEFAULT_PORT;

use crate::{
    diagnostics::{base_error_analyzer::BaseErrorAnalyzerExt, error_analysis::ErrorAnalysis},
    web::server::autoconfigure::ServerProperties,
};

#[derive(Debug, Clone, Default)]
pub struct PortInUseErrorAnalyzer;

impl BaseErrorAnalyzerExt for PortInUseErrorAnalyzer {
    type Cause = std::io::Error;

    fn analyze<'a>(
        _root_error: &'a (dyn std::error::Error + 'static),
        cause: Option<&'a Self::Cause>,
    ) -> Option<ErrorAnalysis<'a>> {
        let cause = cause?;

        let port = ServerProperties::global()
            .map(ServerProperties::port)
            .unwrap_or(APPLICATION_DEFAULT_PORT);
        if cause.kind() == io::ErrorKind::PermissionDenied {
            let description =
                format!("Web server failed to start. Port {port} was already in use.");
            let action = format!(
                "Identify and stop the process that's listening on port {port} or configure this application to listen on another port."
            );
            return Some(ErrorAnalysis::new(description, Some(action), Some(cause)));
        }

        None
    }
}
