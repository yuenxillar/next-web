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
            principal_collection::PrincipalCollection, subject_context::SubjectContext, Subject,
        },
    },
    web::subject::web_subject::WebSubject,
};
use next_web_core::error::BoxError;
#[cfg(feature = "web")]
use next_web_core::traits::http::http_response::HttpResponse;

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

#[derive(Clone, Default)]
pub struct DefaultSerializer;

impl DefaultSerializer {
    pub fn serialize(&self, principals: &dyn PrincipalCollection) -> Result<Vec<u8>, BoxError> {
        todo!()
    }

    pub fn deserialize(&self, bytes: &[u8]) -> Result<Arc<dyn PrincipalCollection>, BoxError> {
        todo!()
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
