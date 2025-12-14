use next_web_core::{
    constants::application_constants::{NEXT_DECRYPT_PASSWORD, NEXT_SECURE_PROPERTIES},
    context::application_args::ApplicationArgs,
    traits::{ordered::Ordered, properties_post_processor::PropertiesPostProcessor},
};
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
        let args = ApplicationArgs::default();
        let password = match args.decrypt_password.as_ref() {
            Some(s) => Some(s.clone()),
            None => match std::env::var(NEXT_DECRYPT_PASSWORD) {
                Ok(password) => Some(password),
                Err(_) => None,
            },
        };

        let password = match password {
            Some(password) => password,
            None => {
                return;
            }
        };

        if let Some(mapping) = mapping {
            match mapping.as_mapping_mut() {
                Some(mapping) => {
                    for (_key, value) in mapping.iter_mut() {
                        decrypt_properties(value, &password);
                    }
                }
                None => return,
            }
        }
    }
}

fn decrypt_properties(value: &mut Value, password: &str) {
    match value {
        Value::String(s) => {
            if s.starts_with(NEXT_SECURE_PROPERTIES) && s[3..4] == *":" {
                let ciphertext = s[4..].trim();
                match crate::util::aes::decrypt(ciphertext, password) {
                    Ok(plaintext) => *s = plaintext,
                    Err(e) => {
                        panic!("Failed to decrypt '{}': {}", ciphertext, e);
                    }
                }
            }
        }
        Value::Mapping(m) => {
            // 递归处理每个子字段
            for (_, v) in m.iter_mut() {
                decrypt_properties(v, password);
            }
        }
        // 其他类型（Array, Bool, Null 等）不处理
        _ => {}
    }
}
