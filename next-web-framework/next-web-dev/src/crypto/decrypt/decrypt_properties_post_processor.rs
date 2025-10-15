use next_web_core::{constants::application_constants::NEXT_SECURE_PROPERTIES, traits::{ordered::Ordered, properties_post_processor::PropertiesPostProcessor}};
use rudi_dev::Singleton;
use serde_yaml::Value;

#[Singleton(binds = [Self::into_post_processor])]
#[derive(Clone)]
pub struct DecryptPropertiesPostProcessor;

impl DecryptPropertiesPostProcessor {
    fn into_post_processor(self) -> Box<dyn PropertiesPostProcessor> {
        Box::new(self)
    }
}
impl Ordered for DecryptPropertiesPostProcessor {
    fn order(&self) -> i32 {
        i32::MIN
    }
}

/// Decrypt the properties.
impl PropertiesPostProcessor for DecryptPropertiesPostProcessor {
    fn post_process_properties(&mut self, mapping: Option<&mut serde_yaml::Value>) {
         if let Some(mapping) = mapping {
            // println!("mapping: {:#?}", mapping);

            match mapping.as_mapping_mut() {
                Some(mapping) => {

                   let mut ref_mapping = mapping;
                    loop {
                        ref_mapping.iter_mut().for_each(|(_key, value)| match value {
                            Value::String(value) => {
                                if value.starts_with(NEXT_SECURE_PROPERTIES) && value[3..4] == *":" {
                                    let val = value[4..].trim();

                                    // decrypt
                                    // let decrypted = crate::crypto::decrypt(val).unwrap();
                                }
                            }
                            Value::Mapping(mapping) => todo!(),
                            _ => {}
                        });

                    }
                }
                None => return
            }
        }
    }
}
