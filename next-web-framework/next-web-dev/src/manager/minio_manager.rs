use std::sync::Arc;

use crate::autoconfigure::context::minio_properties::MinioProperties;

#[derive(Clone)]
pub struct MinioManager {
    minio: minio_rsc::Minio,
    options: MinioProperties,
}

impl MinioManager {
    pub fn new(minio: minio_rsc::Minio, options: MinioProperties) -> Arc<Self> {
        Arc::new(Self { minio, options })
    }

    pub fn get_conn(&mut self) -> &minio_rsc::Minio {
        &self.minio
    }

    pub fn options(&self) -> &MinioProperties {
        &self.options
    }
}
