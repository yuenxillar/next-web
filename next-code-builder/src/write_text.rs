use std::{fs::File, io::Write, path::Path};

use crate::builder_rule::BuilderRule;

#[derive(Default)]
pub struct WriteText {
    pub path: String,
}

impl WriteText {
    pub fn new(path: String, rule: BuilderRule) -> Self {
        let required_folder = match rule {
            BuilderRule::DDD => {
                vec![
                    "/application",
                    "/application/dto",
                    "/application/error",
                    "/application/service",
                    "/application/event",
                    "/domain",
                    "/domain/model",
                    "/domain/model/entity",
                    "/domain/event",
                    "/domain/repository",
                    "/domain/service",
                    "/domain/service/impl",
                    "/domain/error",
                    "/infrastructure",
                    "/infrastructure/data",
                    "/infrastructure/utils",
                    "/infrastructure/impl",
                    "/interface",
                    "/interface/controller",
                    "/interface/error",
                    "/interface/convert",
                ]
            }
            BuilderRule::MVC => {
                vec![
                    "/controller",
                    "/service",
                    "/service/impl",
                    "/model",
                    "/model/dto",
                    "/model/entity",
                    "/model/vo",
                    "/mapper",
                ]
            }
        };

        // check if directory exists and create if not
        for folder in required_folder {
            let folder_path = &format!("{}{}", &path, folder);
            let mod_file = &format!("{}/mod.rs", folder_path);
            if !Path::new(folder_path).exists() {
                match std::fs::create_dir_all(folder_path) {
                    Ok(_) => {}
                    Err(e) => println!("创建文件夹时出错：{}", e),
                }
            } else {
                println!("文件夹已存在：{}", folder_path);
            }

            if !Path::new(mod_file).exists() {
                match File::create(mod_file) {
                    Ok(_) => {}
                    Err(e) => println!("创建文件时出错：{}", e),
                }
            }
        }

        Self { path }
    }

    pub fn write(
        &mut self,
        tera: &tera::Tera,
        context: &tera::Context,
        name: &str,
        role: &BuilderRule,
    ) {
        match role {
            BuilderRule::DDD => {
                for item in DDDWriteContent::items().iter() {
                    let buf = tera.render(&item.template_name(), context).unwrap();
                    let path = item.to_path(&self.path, name);
                    File::create(path)
                        .unwrap()
                        .write_all(buf.as_bytes())
                        .unwrap();
                }
            }
            BuilderRule::MVC => {
                for item in MVCWriteContent::items().iter() {
                    let buf = tera.render(&item.template_name(), context).unwrap();
                    let path = item.to_path(&self.path, name);
                    File::create(path)
                        .unwrap()
                        .write_all(buf.as_bytes())
                        .unwrap();
                }
            }
        };
    }
}

pub trait WriteContent {
    fn items() -> Vec<Self>
    where
        Self: Sized;

    fn to_path(&self, project_path: &str, name: &str) -> String;

    fn template_name(&self) -> String;
}

#[derive(Clone, PartialEq, Eq)]
pub enum DDDWriteContent {
    Controller,
    Entity,
    Mapper,
    Service,
    ServiceImpl,
    Repository,
    RepositoryImpl,
    Config,
}

impl WriteContent for DDDWriteContent {
    fn to_path(&self, project_path: &str, name: &str) -> String {
        match self {
            DDDWriteContent::Controller => format!(
                "{}/interface/controller/{}_controller.rs",
                project_path, name
            ),

            DDDWriteContent::Entity => format!("{}/domain/model/entity/{}.rs", project_path, name),
            DDDWriteContent::Mapper => {
                let path = Path::new(project_path).parent().unwrap().join("resources");
                format!("{}/mapper/{}Mapper.html", path.to_str().unwrap(), name)
            }
            DDDWriteContent::Service => {
                format!("{}/domain/service/{}_service.rs", project_path, name)
            }
            DDDWriteContent::ServiceImpl => format!(
                "{}/domain/service/impl/{}_service_impl.rs",
                project_path, name
            ),
            DDDWriteContent::Repository => {
                format!("{}/domain/repository/{}_repository.rs", project_path, name)
            }
            DDDWriteContent::RepositoryImpl => {
                format!(
                    "{}/infrastructure/impl/{}_repository_impl.rs",
                    project_path, name
                )
            }
            DDDWriteContent::Config => {
                format!("{}/infrastructure/config/{}_config.rs", project_path, name)
            }
        }
    }

    fn template_name(&self) -> String {
        match self {
            DDDWriteContent::Controller => "controller.tmp".into(),
            DDDWriteContent::Entity => "entity.tmp".into(),
            DDDWriteContent::Mapper => "mapper.tmp".into(),
            DDDWriteContent::Service => "service.ddd.tmp".into(),
            DDDWriteContent::ServiceImpl => "service_impl.ddd.tmp".into(),
            DDDWriteContent::Repository => "repository.ddd.tmp".into(),
            DDDWriteContent::RepositoryImpl => "repository_impl.ddd.tmp".into(),
            DDDWriteContent::Config => "".into(),
        }
    }

    fn items() -> Vec<Self>
    where
        Self: Sized,
    {
        vec![
            DDDWriteContent::Controller,
            DDDWriteContent::Entity,
            DDDWriteContent::Mapper,
            DDDWriteContent::Service,
            DDDWriteContent::ServiceImpl,
            DDDWriteContent::Repository,
            DDDWriteContent::RepositoryImpl,
        ]
    }
}

#[derive(Clone, PartialEq, Eq)]
pub enum MVCWriteContent {
    Controller,
    Entity,
    Mapper,
    Service,
    ServiceImpl,
    Config,
}

impl WriteContent for MVCWriteContent {
    fn to_path(&self, project_path: &str, name: &str) -> String {
        match self {
            MVCWriteContent::Controller => {
                format!("{}/controller/{}_controller.rs", project_path, name)
            }
            MVCWriteContent::Entity => format!("{}/model/entity/{}.rs", project_path, name),
            MVCWriteContent::Mapper => {
                let path = Path::new(project_path).parent().unwrap().join("resources");
                format!("{}/mapper/{}Mapper.html", path.to_str().unwrap(), name)
            }
            MVCWriteContent::Service => format!("{}/service/{}_service.rs", project_path, name),
            MVCWriteContent::ServiceImpl => {
                format!("{}/service/impl/{}_service_impl.rs", project_path, name)
            }
            MVCWriteContent::Config => format!("{}/config/{}_config.rs", project_path, name),
        }
    }

    fn template_name(&self) -> String {
        match self {
            MVCWriteContent::Controller => "controller.tmp".into(),
            MVCWriteContent::Entity => "entity.tmp".into(),
            MVCWriteContent::Mapper => "mapper.tmp".into(),
            MVCWriteContent::Service => "service.mvc.tmp".into(),
            MVCWriteContent::ServiceImpl => "service_impl.mvc.tmp".into(),
            MVCWriteContent::Config => todo!(),
        }
    }
    fn items() -> Vec<Self>
    where
        Self: Sized,
    {
        vec![
            MVCWriteContent::Controller,
            MVCWriteContent::Entity,
            MVCWriteContent::Mapper,
            MVCWriteContent::Service,
            MVCWriteContent::ServiceImpl,
        ]
    }
}
