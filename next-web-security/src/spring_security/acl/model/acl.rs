use crate::acl::model::{AclError, ObjectIdentity, Permission, SidImpl};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AccessControlEntry {
    id: Option<String>,
    acl_object_identity: ObjectIdentity,
    sid: SidImpl,
    permission_mask: i32,
    granting: bool,
    audit_success: bool,
    audit_failure: bool,
}

impl AccessControlEntry {
    pub fn new(
        id: Option<String>,
        acl_object_identity: ObjectIdentity,
        sid: SidImpl,
        permission_mask: i32,
        granting: bool,
    ) -> Self {
        Self {
            id,
            acl_object_identity,
            sid,
            permission_mask,
            granting,
            audit_success: false,
            audit_failure: false,
        }
    }

    pub fn id(&self) -> Option<&str> {
        self.id.as_deref()
    }

    pub fn acl_object_identity(&self) -> &ObjectIdentity {
        &self.acl_object_identity
    }

    pub fn sid(&self) -> &SidImpl {
        &self.sid
    }

    pub fn permission_mask(&self) -> i32 {
        self.permission_mask
    }

    pub fn is_granting(&self) -> bool {
        self.granting
    }

    pub fn audit_success(&self) -> bool {
        self.audit_success
    }

    pub fn audit_failure(&self) -> bool {
        self.audit_failure
    }

    pub fn set_audit_success(&mut self, value: bool) {
        self.audit_success = value;
    }

    pub fn set_audit_failure(&mut self, value: bool) {
        self.audit_failure = value;
    }
}

pub trait Acl: Send + Sync {
    fn object_identity(&self) -> &ObjectIdentity;

    fn owner(&self) -> Option<&SidImpl>;

    fn entries(&self) -> &[AccessControlEntry];

    fn parent_acl(&self) -> Option<&dyn Acl> {
        None
    }

    fn is_entries_inheriting(&self) -> bool;

    fn is_granted(
        &self,
        permissions: &[&dyn Permission],
        sids: &[SidImpl],
        administrative_mode: bool,
    ) -> Result<bool, AclError>;
}

pub trait MutableAcl: Acl {
    fn set_owner(&mut self, owner: SidImpl);

    fn set_entries_inheriting(&mut self, entries_inheriting: bool);

    fn insert_ace(
        &mut self,
        at_index_location: usize,
        permission: &dyn Permission,
        sid: SidImpl,
        granting: bool,
    );

    fn update_ace(&mut self, ace_index: usize, permission: &dyn Permission) -> Result<(), AclError>;

    fn delete_ace(&mut self, ace_index: usize) -> Result<(), AclError>;
}
