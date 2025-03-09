use std::sync::Arc;

use minio_rsc::{provider::StaticProvider, Minio};

use super::auto_register::AutoRegister;
use crate::{
    autoconfigure::context::minio_properties::MinioProperties, manager::minio_manager::MinioManager,
};

pub struct MinioAutoRegister(pub MinioProperties);

impl AutoRegister for MinioAutoRegister {
    fn register(&self, ctx: &mut rudi::Context) -> Result<(), Box<dyn std::error::Error>> {
        let provider = StaticProvider::new(self.0.access_key(), self.0.secret_key(), None);
        let minio = Minio::builder()
            .endpoint(self.0.endpoint())
            .provider(provider)
            .secure(false)
            .build()?;
        let minio = MinioManager::new(minio, self.0.clone());
        ctx.insert_single_owner_with_name::<Arc<MinioManager>, String>(
            minio,
            String::from("minioManager"),
        );
        println!("MinioAutoRegister registered successfully!");
        Ok(())
    }
}
