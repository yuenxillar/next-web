use crate::core::AuthenticationError;

use super::x509_certificate::X509Certificate;

/// Obtains the principal from an [`X509Certificate`] for use within the framework.
pub trait X509PrincipalExtractor: Send + Sync {
    /// Returns the principal (usually a string) for the given certificate.
    fn extract_principal(&self, certificate: &X509Certificate)
        -> Result<String, AuthenticationError>;
}

impl<F> X509PrincipalExtractor for F
where
    F: Fn(&X509Certificate) -> Result<String, AuthenticationError> + Send + Sync,
{
    fn extract_principal(
        &self,
        certificate: &X509Certificate,
    ) -> Result<String, AuthenticationError> {
        self(certificate)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn closures_can_be_used_as_extractors() {
        let extractor = |certificate: &X509Certificate| {
            Ok(format!("principal:{}", certificate.subject_dn()))
        };

        let principal = extractor
            .extract_principal(&X509Certificate::new("CN=Luke Taylor"))
            .expect("principal should be extracted");

        assert_eq!(principal, "principal:CN=Luke Taylor");
    }
}
