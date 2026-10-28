use std::{any::Any, sync::Arc};

use crate::{
    core::{
        authc::{
            authentication_info::AuthenticationInfo, authentication_token::AuthenticationToken,
            remember_me_authentication_token::RememberMeAuthenticationToken,
            username_password_token::UsernamePasswordToken,
        },
        crypto::{
            cipher::{aes_cipher_service::AesCipherService, cipher_service::CipherService},
            crypto_error::CryptoError,
        },
        subject::{
            principal_collection::PrincipalCollection,
            simple_principal_collection::SimplePrincipalCollection, subject_context::SubjectContext,
            Subject,
        },
        util::object::Object,
    },
    web::subject::web_subject::WebSubject,
};
use next_web_core::error::BoxError;
#[cfg(feature = "web")]
use next_web_core::http::HttpResponse;

#[derive(Clone)]
pub struct DefaultRememberMeManager<C = AesCipherService> {
    serializer: DefaultSerializer,
    cipher_service: C,
    /// Cipher encryption key to use with the Cipher when encrypting data
    encryption_cipher_key: Vec<u8>,
    /// Cipher decryption key to use with the Cipher when decrypting data
    decryption_cipher_key: Vec<u8>,
}

impl DefaultRememberMeManager {
    pub fn with_key(key: Vec<u8>) -> Self {
        let mut manager = Self::default();
        manager.set_cipher_key(key);
        manager
    }
}

impl<C> DefaultRememberMeManager<C>
where
    C: CipherService,
{
    pub fn new(cipher_service: C, key: Vec<u8>) -> Self {
        Self {
            cipher_service,
            serializer: Default::default(),
            encryption_cipher_key: key.clone(),
            decryption_cipher_key: key,
        }
    }

    pub fn get_serializer(&self) -> &DefaultSerializer {
        &self.serializer
    }

    pub fn get_cipher_service(&self) -> &C {
        &self.cipher_service
    }

    pub fn set_cipher_service(&mut self, cipher_service: C) {
        self.cipher_service = cipher_service;
    }

    pub fn get_encryption_cipher_key(&self) -> &[u8] {
        &self.encryption_cipher_key
    }

    pub fn set_encryption_cipher_key(&mut self, key: Vec<u8>) {
        self.encryption_cipher_key = key;
    }

    pub fn get_decryption_cipher_key(&self) -> &[u8] {
        &self.decryption_cipher_key
    }

    pub fn set_decryption_cipher_key(&mut self, key: Vec<u8>) {
        self.decryption_cipher_key = key;
    }

    pub fn get_cipher_key(&self) -> &[u8] {
        self.get_encryption_cipher_key()
    }

    pub fn set_cipher_key(&mut self, key: Vec<u8>) {
        self.set_encryption_cipher_key(key.clone());
        self.set_decryption_cipher_key(key);
    }

    // 核心方法实现
    pub fn is_remember_me(&self, token: &dyn AuthenticationToken) -> bool {
        if let Some(token) = (token as &dyn Any).downcast_ref::<UsernamePasswordToken>() {
            return token.is_remember_me();
        }
        false
    }

    fn get_identity_to_remember<'a>(
        &'a self,
        _subject: &dyn Subject,
        info: &'a dyn AuthenticationInfo,
    ) -> Option<&'a Arc<dyn PrincipalCollection>> {
        info.get_principals()
    }

    fn _remember_identity(
        &self,
        #[cfg(feature = "web")] subject: &dyn WebSubject,
        #[cfg(not(feature = "web"))] subject: &dyn Subject,
        principals: &dyn PrincipalCollection,
        ext: &dyn DefaultRememberMeManagerExt,

        #[cfg(feature = "web")] resp: &mut dyn HttpResponse,
    ) -> Result<(), BoxError> {
        let bytes = self.convert_principals_to_bytes(principals)?;
        ext.remember_serialized_identity(subject, &bytes, resp)?;
        Ok(())
    }

    pub(super) fn convert_principals_to_bytes(
        &self,
        principals: &dyn PrincipalCollection,
    ) -> Result<Vec<u8>, BoxError> {
        let bytes = self.serialize(principals)?;
        // 检查是否启用加密
        Ok(self.encrypt(&bytes).map_err(Into::<BoxError>::into)?)
    }

    pub fn convert_bytes_to_principals(
        &self,
        bytes: &[u8],
        _subject_context: &dyn SubjectContext,
    ) -> Result<Arc<dyn PrincipalCollection>, BoxError> {
        let bytes = self.decrypt(bytes)?;
        self.deserialize(&bytes)
    }

    fn encrypt(&self, serialized: &[u8]) -> Result<Vec<u8>, CryptoError> {
        self.cipher_service
            .encrypt(serialized, &self.encryption_cipher_key)
    }

    fn decrypt(&self, encrypted: &[u8]) -> Result<Vec<u8>, CryptoError> {
        self.cipher_service
            .decrypt(encrypted, &self.decryption_cipher_key)
    }

    fn serialize(&self, principals: &dyn PrincipalCollection) -> Result<Vec<u8>, BoxError> {
        self.serializer.serialize(principals)
    }

    fn deserialize(
        &self,
        serialized_identity: &[u8],
    ) -> Result<Arc<dyn PrincipalCollection>, BoxError> {
        self.serializer.deserialize(serialized_identity)
    }
}

impl<C> DefaultRememberMeManager<C>
where
    C: CipherService,
{
    pub fn remember_identity(
        &self,
        subject: &dyn WebSubject,
        _token: &dyn AuthenticationToken,
        authc_info: &dyn AuthenticationInfo,
        ext: &dyn DefaultRememberMeManagerExt,

        #[cfg(feature = "web")] resp: &mut dyn HttpResponse,
    ) -> Result<(), BoxError> {
        if let Some(principals) = self.get_identity_to_remember(subject, authc_info) {
            self._remember_identity(subject, principals.as_ref(), ext, resp)?;
        }

        Ok(())
    }
}

impl Default for DefaultRememberMeManager {
    fn default() -> Self {
        let cipher_service = AesCipherService::default();
        let key = cipher_service.generate_new_key().unwrap();

        Self {
            serializer: Default::default(),
            cipher_service,
            encryption_cipher_key: key.clone(),
            decryption_cipher_key: key,
        }
    }
}

/// The serializer of the remembered identity.
///
/// Apache Shiro serializes the principal collection with the Java serializer.
/// The port has no serializer of the language to lean on, so it writes a small,
/// explicit format instead: the number of principals, then each principal as its
/// UTF-8 length and its bytes. A principal is written as the text it renders as,
/// so a remembered identity is read back as a string principal.
#[derive(Clone, Default)]
pub struct DefaultSerializer;

impl DefaultSerializer {
    /// Returns the bytes a remember-me cookie carries for the given principals.
    ///
    /// # Arguments
    ///
    /// * `principals` - The principals to remember.
    ///
    /// # Errors
    ///
    /// Returns an error when the principals do not fit the format.
    pub fn serialize(&self, principals: &dyn PrincipalCollection) -> Result<Vec<u8>, BoxError> {
        let mut values = Vec::new();

        if let Some(primary) = principals.get_primary_principal() {
            values.push(primary.to_string());
        }
        if let Some(realm_names) = principals.get_realm_names() {
            let mut names = realm_names.into_iter().collect::<Vec<_>>();
            names.sort_unstable();
            for name in names {
                values.push(name.to_string());
            }
        }

        let mut bytes = Vec::new();
        bytes.extend_from_slice(&(values.len() as u32).to_be_bytes());
        for value in values {
            let value = value.as_bytes();
            let length = u32::try_from(value.len())
                .map_err(|_| -> BoxError { "The remembered principal is too long.".into() })?;
            bytes.extend_from_slice(&length.to_be_bytes());
            bytes.extend_from_slice(value);
        }

        Ok(bytes)
    }

    /// Returns the principals a remember-me cookie carries, from its bytes.
    ///
    /// # Arguments
    ///
    /// * `bytes` - The bytes of the cookie.
    ///
    /// # Errors
    ///
    /// Returns an error when the bytes are not the format
    /// [`Self::serialize`] writes.
    pub fn deserialize(&self, bytes: &[u8]) -> Result<Arc<dyn PrincipalCollection>, BoxError> {
        let mut values = Vec::new();
        let mut offset = 0;

        let count = Self::read_u32(bytes, &mut offset)?;
        for _ in 0..count {
            let length = Self::read_u32(bytes, &mut offset)? as usize;
            if offset + length > bytes.len() {
                return Err("The remembered identity is truncated.".into());
            }

            let value = std::str::from_utf8(&bytes[offset..offset + length])
                .map_err(|_| -> BoxError { "The remembered identity is not UTF-8.".into() })?;
            values.push(value.to_string());
            offset += length;
        }

        let mut principals = SimplePrincipalCollection::default();
        match values.split_first() {
            Some((primary, realms)) => {
                let realm_name = realms.first().cloned().unwrap_or_else(|| "rememberMe".to_string());
                principals.add(Object::Str(primary.clone()), realm_name);
            }
            None => {
                return Err("The remembered identity holds no principal.".into());
            }
        }

        Ok(Arc::new(principals))
    }

    /// Reads a big endian `u32` from the bytes.
    ///
    /// # Arguments
    ///
    /// * `bytes` - The bytes to read from.
    /// * `offset` - The position to read at, advanced past the value.
    fn read_u32(bytes: &[u8], offset: &mut usize) -> Result<u32, BoxError> {
        if *offset + 4 > bytes.len() {
            return Err("The remembered identity is truncated.".into());
        }

        let value = u32::from_be_bytes([
            bytes[*offset],
            bytes[*offset + 1],
            bytes[*offset + 2],
            bytes[*offset + 3],
        ]);
        *offset += 4;

        Ok(value)
    }
}

#[cfg(test)]
mod tests {
    

    use super::DefaultSerializer;
    use crate::core::{
        subject::{
            simple_principal_collection::SimplePrincipalCollection,
        },
        util::object::Object,
    };

    #[test]
    fn a_remembered_identity_survives_the_round_trip() {
        let principals = SimplePrincipalCollection::new(
            [Object::Str("root".to_string())],
            "iniRealm",
        );
        let serializer = DefaultSerializer;

        let bytes = serializer.serialize(&principals).expect("the identity serializes");
        let remembered = serializer.deserialize(&bytes).expect("the identity deserializes");

        assert_eq!(
            remembered.get_primary_principal().map(ToString::to_string),
            Some("root".to_string())
        );
        assert_eq!(
            remembered
                .get_realm_names()
                .map(|names| names.into_iter().collect::<Vec<_>>()),
            Some(vec!["iniRealm"])
        );
    }

    #[test]
    fn a_truncated_identity_is_refused() {
        let serializer = DefaultSerializer;
        let principals = SimplePrincipalCollection::new(
            [Object::Str("root".to_string())],
            "iniRealm",
        );
        let bytes = serializer.serialize(&principals).expect("the identity serializes");

        let error = serializer
            .deserialize(&bytes[..bytes.len() - 1])
            .err()
            .expect("the identity is incomplete");

        assert!(error.to_string().contains("truncated"));
    }

    #[test]
    fn bytes_that_are_not_the_format_are_refused() {
        let serializer = DefaultSerializer;

        assert!(serializer.deserialize(&[0x00]).is_err());
        assert!(serializer.deserialize(&[0, 0, 0, 0]).is_err());
    }
}


pub trait DefaultRememberMeManagerExt
where
    Self: Send + Sync,
{
    fn remember_serialized_identity(
        &self,
        #[cfg(feature = "web")] subject: &dyn WebSubject,
        #[cfg(not(feature = "web"))] subject: &dyn Subject,
        serialized: &[u8],

        #[cfg(feature = "web")] resp: &mut dyn HttpResponse,
    ) -> Result<(), BoxError>;
}
