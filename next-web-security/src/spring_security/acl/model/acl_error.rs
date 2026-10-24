use std::fmt::{Display, Formatter};

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum AclErrorKind {
    NotFound,
    AlreadyExists,
    ChildrenExist,
    UnloadedSid,
    Authorization,
    DataAccess,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AclError {
    kind: AclErrorKind,
    message: String,
}

impl AclError {
    pub fn new(kind: AclErrorKind, message: impl Into<String>) -> Self {
        Self {
            kind,
            message: message.into(),
        }
    }

    pub fn not_found(message: impl Into<String>) -> Self {
        Self::new(AclErrorKind::NotFound, message)
    }

    pub fn already_exists(message: impl Into<String>) -> Self {
        Self::new(AclErrorKind::AlreadyExists, message)
    }

    pub fn children_exist(message: impl Into<String>) -> Self {
        Self::new(AclErrorKind::ChildrenExist, message)
    }

    pub fn authorization(message: impl Into<String>) -> Self {
        Self::new(AclErrorKind::Authorization, message)
    }

    pub fn kind(&self) -> &AclErrorKind {
        &self.kind
    }

    pub fn message(&self) -> &str {
        &self.message
    }
}

impl Display for AclError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}: {}", self.kind, self.message)
    }
}

impl std::error::Error for AclError {}
