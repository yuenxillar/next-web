mod dependency_injection;
mod context;

use std::any::{Any, TypeId};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use std::fmt::{Debug, Display};


// 特征定义，用于示例
pub trait UserService: Send + Sync {
    fn get_user_by_id(&self, id: u64) -> String;
}

// 特征实现，用于示例
#[derive(Clone)]
struct UserServiceImpl {
    prefix: String,
}

impl UserService for UserServiceImpl {
    fn get_user_by_id(&self, id: u64) -> String {
        format!("{}: 用户{}", self.prefix, id)
    }
}

// 另一个实现，用于示例
#[derive(Clone)]
struct AdminUserServiceImpl;

impl UserService for AdminUserServiceImpl {
    fn get_user_by_id(&self, id: u64) -> String {
        format!("管理员: 用户{}", id)
    }
}

// 简单类型，用于示例
#[derive(Clone)]
struct Config {
    app_name: String,
    version: String,
}


impl Config {

   #[Bean(name = "userServiceImpl1")]
   pub fn get_app_name() -> &'static str {
        "&self.app_name"
   }
}

use next_web_macro::Bean;

#[Bean(name = "userServiceImpl", scope = "Singleton")]
pub fn test() -> String {
    "test".to_string()
}

