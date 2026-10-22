//! Loading a private CA bundle for `https://` upstreams.
//!
//! The bundle is parsed once at startup so a missing or malformed file is a
//! configuration error rather than a surprise on the first TLS request.

use std::sync::Arc;

use pingora::protocols::tls::CaType;
use pingora::utils::tls::{WrappedX509, parse_x509};

use crate::config::{ConfigError, TlsConfig};

/// A parsed trust anchor set, shared by every TLS peer.
pub type CaBundle = CaType;

/// Read and parse `upstream.tls.ca_file`, when configured.
pub fn load(tls: &TlsConfig) -> Result<Option<Arc<CaBundle>>, ConfigError> {
    let Some(path) = tls.ca_file.as_ref() else {
        return Ok(None);
    };

    let pem = std::fs::read(path).map_err(|error| {
        ConfigError::new(
            "upstream.tls.ca_file",
            format!("cannot read `{}`: {error}", path.display()),
        )
    })?;

    let certificates = rustls_pemfile::certs(&mut pem.as_slice())
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| {
            ConfigError::new(
                "upstream.tls.ca_file",
                format!(
                    "`{}` is not a PEM certificate bundle: {error}",
                    path.display()
                ),
            )
        })?;

    if certificates.is_empty() {
        return Err(ConfigError::new(
            "upstream.tls.ca_file",
            format!("`{}` contains no certificates", path.display()),
        ));
    }

    let anchors: Vec<WrappedX509> = certificates
        .into_iter()
        .map(|der| WrappedX509::new(der.to_vec(), parse_x509))
        .collect();

    Ok(Some(Arc::from(anchors)))
}
