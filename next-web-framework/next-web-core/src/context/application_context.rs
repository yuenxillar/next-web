// use std::{any::{Any, TypeId}, borrow::Cow, collections::HashMap, fmt::Display, sync::{atomic::AtomicBool, RwLock}};

// use crate::dependency_injection::{definition::Key, provider::{DynProvider, Provider}, scope::Scope};


// /// 依赖注入容器错误
// #[derive(Debug)]
// pub enum ApplicationContextError {
//     ComponentNotFound(String),
//     WrongType(String),
//     DuplicateRegistration(String),
// }

// impl Display for ApplicationContextError {
//     fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
//         match self {
//             Self::ComponentNotFound(msg) => write!(f, "组件未找到: {}", msg),
//             Self::WrongType(msg) => write!(f, "类型错误: {}", msg),
//             Self::DuplicateRegistration(msg) => write!(f, "重复注册: {}", msg),
//         }
//     }
// }

// impl std::error::Error for ApplicationContextError {}

// // 创建一个对象安全的 trait
// trait ComponentTrait: Any {
//     // 定义一个克隆方法，返回 Box<dyn ComponentTrait> 而不是 Self
//     fn clone_box(&self) -> Box<dyn ComponentTrait + Send + Sync>;
    
//     // 提供转换为 Any 的方法
//     fn as_any(&self) -> &dyn Any;
// }

// // 为所有实现 Any + Clone 的类型实现 ComponentTrait
// impl<T: Any + Clone + Send + Sync + 'static> ComponentTrait for T {
//     fn clone_box(&self) -> Box<dyn ComponentTrait + Send + Sync> {
//         Box::new(self.clone())
//     }
    
//     fn as_any(&self) -> &dyn Any {
//         self
//     }
// }

// // 为 Box<dyn ComponentTrait> 实现 Clone
// impl Clone for Box<dyn ComponentTrait + Send + Sync> {
//     fn clone(&self) -> Self {
//         self.clone_box()
//     }
// }

// /// 依赖注入容器
// #[derive(Default)]
// pub struct ApplicationContext {
//     // 按类型存储的组件
//     components_by_type: RwLock<HashMap<TypeId, Box<dyn ComponentTrait + Send + Sync>>>,
    
//     // 按名称存储的组件
//     components_by_name: RwLock<HashMap<Cow<'static, str>, Box<dyn ComponentTrait + Send + Sync>>>,
    
//     // 类型到名称的映射，用于找到某个类型的所有实现
//     type_to_names: RwLock<HashMap<TypeId, Vec<Cow<'static, str>>>>,
    
//     // 默认是否覆盖已存在组件
//     allow_override: bool,
    
//     loaded_modules: Vec<Type>,
//     single_registry: SingleRegistry,
//     provider_registry: ProviderRegistry,

//     conditional_providers: Vec<(bool, DynProvider)>,
    

//     dependency_chain: DependencyChain,
    
//     // 工厂方法存储
//     factories: RwLock<HashMap<Cow<'static, str>, Box<dyn Fn() -> Box<dyn ComponentTrait + Send + Sync> + Send + Sync>>>,
// }

// impl Clone for ApplicationContext {
//     fn clone(&self) -> Self {
//         // 克隆按类型存储的组件
//         let components_by_type = {
//             let components = self.components_by_type.read().unwrap();
//             let mut new_components = HashMap::new();
            
//             for (type_id, component) in components.iter() {
//                 // 使用 Any 的 downcast_ref 获取具体类型，然后克隆
//                 let cloned: Box<dyn ComponentTrait + Send + Sync> = component.clone();
//                 new_components.insert(*type_id, cloned);
//             }
            
//             RwLock::new(new_components)
//         };
        
//         // 克隆按名称存储的组件
//         let components_by_name = {
//             let components = self.components_by_name.read().unwrap();
//             let mut new_components = HashMap::new();
            
//             for (name, component) in components.iter() {
//                 let cloned: Box<dyn ComponentTrait + Send + Sync> = component.clone();
//                 new_components.insert(name.clone(), cloned);
//             }
            
//             RwLock::new(new_components)
//         };
        
//         // 克隆类型到名称的映射
//         let type_to_names = {
//             let mappings = self.type_to_names.read().unwrap();
//             let mut new_mappings = HashMap::new();
            
//             for (type_id, names) in mappings.iter() {
//                 new_mappings.insert(*type_id, names.clone());
//             }
            
//             RwLock::new(new_mappings)
//         };
        
//         Self {
//             components_by_type,
//             components_by_name,
//             type_to_names,
//             allow_override: self.allow_override,
//             factories: RwLock::new(HashMap::new()),
//         }
//     }
// }

// impl Default for ApplicationContext {
//     fn default() -> Self {
//         Self {
//             components_by_type: Default::default(),
//             components_by_name: Default::default(),
//             type_to_names: Default::default(),
//             allow_override: true,
//             factories: RwLock::new(HashMap::new()),
//         }

//     }
// }

// impl ApplicationContext {
//     /// 创建新的依赖注入容器
//     pub fn new() -> Self {
//         Self::default()
//     }
    
//     /// 设置默认覆盖行为
//     pub fn set_allow_override(&mut self, allow_override: bool) {
//         self.allow_override = allow_override;
//     }
    
//     /// 获取默认覆盖行为
//     pub fn get_allow_override(&self) -> bool {
//         self.allow_override
//     }
    
//     /// 按类型注册组件，可选是否覆盖
//     pub fn register_with_override<T: 'static + Send + Sync + Clone>(&self, component: T, override_existing: bool) -> Result<(), ApplicationContextError> {
//         let type_id = TypeId::of::<T>();
//         let mut components = self.components_by_type.write().unwrap();
        
//         if components.contains_key(&type_id) && !override_existing {
//             return Err(ApplicationContextError::DuplicateRegistration(
//                 format!("类型 {:?} 已注册", type_id)
//             ));
//         }
        
//         components.insert(type_id, Box::new(component));
//         Ok(())
//     }
    
//     /// 按名称注册组件，可选是否覆盖
//     pub fn register_named_with_override<T: 'static + Send + Sync + Clone>(
//         &self, 
//         name: impl Into<Cow<'static, str>>, 
//         component: T,
//         override_existing: bool
//     ) -> Result<(), ApplicationContextError> {
//         let type_id = TypeId::of::<T>();
//         let mut components = self.components_by_name.write().unwrap();
//         let mut type_names = self.type_to_names.write().unwrap();
        
//         let name = name.into();
//         if components.contains_key(&name) && !override_existing {
//             return Err(ApplicationContextError::DuplicateRegistration(
//                 format!("名称 {} 已注册", &name)
//             ));
//         }
        
//         // 如果覆盖且名称已存在，需要移除旧类型到名称的映射
//         if override_existing && components.contains_key(&name) {
//             // 查找当前关联的类型
//             for (_tid, names) in type_names.iter_mut() {
//                 names.retain(|n| n != &name);
//             }
//         }
        
//         components.insert(name.clone(), Box::new(component));
        
//         // 更新类型到名称的映射
//         type_names
//             .entry(type_id)
//             .or_insert_with(Vec::new)
//             .push(name);
            
//         Ok(())
//     }
    
//     /// 按类型获取组件
//     fn get_instance<T: 'static + Send + Sync + Clone>(&self) -> Result<T, ApplicationContextError> {
//         let type_id = TypeId::of::<T>();
//         let components = self.components_by_type.read().unwrap();
        
//         if let Some(component) = components.get(&type_id) {
//             if let Some(typed_component) = component.as_any().downcast_ref::<T>() {
//                 return Ok(typed_component.clone());
//             }
//         }
        
//         // 查找是否有名称注册的同类型组件
//         let type_names = self.type_to_names.read().unwrap();
//         if let Some(names) = type_names.get(&type_id) {
//             if !names.is_empty() {
//                 return self.get_named(names[0].clone());
//             }
//         }
        
//         Err(ApplicationContextError::ComponentNotFound(
//             format!("未找到类型为 {:?} 的组件", type_id)
//         ))
//     }
    
//     /// 按名称获取组件
//     fn get_named<T: 'static + Send + Sync + Clone>(&self, name: impl Into<Cow<'static, str>>) -> Result<T, ApplicationContextError> {
//         let components = self.components_by_name.read().unwrap();
        
//         let name = name.into();
//         if let Some(component) = components.get(&name) {
//             if let Some(typed_component) = component.as_any().downcast_ref::<T>() {
//                 return Ok(typed_component.clone());
//             } else {
//                 return Err(ApplicationContextError::WrongType(
//                     format!("组件 {} 类型不匹配", &name)
//                 ));
//             }
//         }
        
//         Err(ApplicationContextError::ComponentNotFound(
//             format!("未找到名称为 {} 的组件", &name)
//         ))
//     }
    
//     /// 检查某个类型是否已注册
//     pub fn contains<T: 'static + Send + Sync + Clone>(&self) -> bool {
//         let type_id = TypeId::of::<T>();
//         let components = self.components_by_type.read().unwrap();
//         components.contains_key(&type_id)
//     }
    
//     /// 检查某个名称是否已注册
//     pub fn contains_named(&self, name: impl Into<Cow<'static, str>>) -> bool {
//         let components = self.components_by_name.read().unwrap();
//         let name = name.into();
//         components.contains_key(&name)
//     }

//     // 保持原始方法名称的向后兼容方法
//     pub fn register_swith_type<T: 'static + Send + Sync + Clone>(&self, component: T) -> Result<(), ApplicationContextError> {
//         self.register_with_override(component, false)
//     }


//     pub fn register_with_name<T: 'static + Send + Sync + Clone>(
//         &self, 
//         name: impl Into<Cow<'static, str>>, 
//         component: T
//     ) -> Result<(), ApplicationContextError> {
//         self.register_named_with_override(name, component, false)
//     }

//     /// 获取特定类型的所有组件（包括按类型和按名称注册的）
//     pub fn get_singles_by_type<T: 'static + Send + Sync + Clone + PartialEq>(&self) -> Vec<T> {
//         let mut results = Vec::new();
        
//         // 获取按类型注册的组件
//         if let Ok(component) = self.get_instance::<T>() {
//             results.push(component);
//         }
        
//         // 获取按名称注册的该类型的所有组件
//         let type_id = TypeId::of::<T>();
//         if let Ok(type_names) = self.type_to_names.read() {
//             if let Some(names) = type_names.get(&type_id) {
//                 for name in names {
//                     if let Ok(component) = self.get_named::<T>(name.clone()) {
//                         // 避免重复（如果已经通过 get 方法获取过）
//                         if results.is_empty() || !results.contains(&component) {
//                             results.push(component);
//                         }
//                     }
//                 }
//             }
//         }
        
//         results
//     }
    
//     /// 获取可选的单个组件（按类型）
//     // pub fn get_single_option<T: 'static + Send + Sync + Clone>(&self) -> Option<T> {
//     //     self.get_instance::<T>().ok()
//     // }
    
//     // /// 获取可选的单个组件（按名称）
//     // pub fn get_single_option_with_name<T: 'static + Send + Sync + Clone>(&self, name: impl Into<Cow<'static, str>>) -> Option<T> {
//     //     self.get_named::<T>(name).ok()
//     // }
    
//     // /// 获取单个组件（按类型，返回值）
//     // pub fn get_single<T: 'static + Send + Sync + Clone>(&self) -> T {
//     //     self.get_instance::<T>().unwrap()
//     // }

//     // 获取全局单例 ApplicationContext
//     pub fn get_global_context() -> &'static ApplicationContext {
//         static mut CONTEXT: Option<ApplicationContext> = None;
//         static INIT: std::sync::Once = std::sync::Once::new();
        
//         unsafe {
//             INIT.call_once(|| {
//                 CONTEXT = Some(ApplicationContext::new());
//             });
//             CONTEXT.as_ref().unwrap()
//         }
//     }
    
//     // 注册工厂方法
//     pub fn register_factory<T: 'static + Send + Sync + Clone>(
//         &self, 
//         name: impl Into<Cow<'static, str>>, 
//         factory: Box<dyn Fn() -> T + Send + Sync>
//     ) -> Result<(), ApplicationContextError> {
//         let type_id = TypeId::of::<T>();
//         let mut factories = self.factories.write().unwrap();
        
//         let name = name.into();
//         factories.insert(name.clone(), Box::new(move || {
//             Box::new(factory()) as Box<dyn ComponentTrait + Send + Sync>
//         }));
        
//         // 更新类型到名称的映射
//         let mut type_names = self.type_to_names.write().unwrap();
//         type_names
//             .entry(type_id)
//             .or_insert_with(Vec::new)
//             .push(name);
            
//         Ok(())
//     }
    

//     pub async fn get_single_with_name_async<T:'static + Send + Sync + Clone>(&self, name: impl Into<Cow<'static, str>>) -> T{
//         T
//     }


//     // // 从 ApplicationContext 获取组件，如果不存在则使用工厂创建
//     // pub fn get_single_with_name<T: 'static + Send + Sync + Clone>(&self, name: impl Into<Cow<'static, str>>) -> Option<T> {
//     //     // 先尝试从一级缓存获取
//     //     let name = name.into();
//     //     if let Some(component) = self.get_named::<T>(name.clone()).ok() {
//     //         return Some(component);
//     //     }
        
//     //     // 检查是否有工厂方法
//     //     let factories = self.factories.read().unwrap();
//     //     if let Some(factory) = factories.get(&name) {
//     //         // 创建实例
//     //         let instance = factory();
//     //         if let Some(typed_instance) = instance.as_any().downcast_ref::<T>() {
//     //             let cloned = typed_instance.clone();
                
//     //             // 如果是单例，注册到容器
//     //             let _ = self.register_named_with_override(name, cloned.clone(), true);
                
//     //             return Some(cloned);
//     //         }
//     //     }
//     //     None
//     // }

  

// }



// impl ApplicationContext {


    
//     /// Returns true if the context contains a [`Singleton`](crate::Scope::Singleton) or [`SingleOwner`](crate::Scope::SingleOwner) instance for the specified type and default name `""`.
//     ///
//     /// # Example
//     ///
//     /// ```rust
//     /// use rudi::{Context, Singleton};
//     ///
//     /// #[derive(Clone)]
//     /// #[Singleton(eager_create)]
//     /// struct A;
//     ///
//     /// # fn main() {
//     /// let cx = Context::auto_register();
//     /// assert!(cx.contains_single::<A>());
//     /// # }
//     /// ```
//     pub fn contains_single<T: 'static>(&self) -> bool {
//         self.contains_single_with_name::<T>("")
//     }

//     /// Returns true if the context contains a [`Singleton`](crate::Scope::Singleton) or [`SingleOwner`](crate::Scope::SingleOwner) instance for the specified type and name.
//     ///
//     /// # Example
//     ///
//     /// ```rust
//     /// use rudi::{Context, Singleton};
//     ///
//     /// #[derive(Clone)]
//     /// #[Singleton(eager_create, name = "a")]
//     /// struct A;
//     ///
//     /// # fn main() {
//     /// let cx = Context::auto_register();
//     /// assert!(cx.contains_single_with_name::<A>("a"));
//     /// # }
//     /// ```
//     pub fn contains_single_with_name<T: 'static>(
//         &self,
//         name: impl Into<Cow<'static, str>>,
//     ) -> bool {
//         let key = Key::new::<T>(name.into());
//         self.single_registry.contains(&key)
//     }

//     /// Returns a reference to a [`Singleton`](crate::Scope::Singleton) or [`SingleOwner`](crate::Scope::SingleOwner) instance based on the given type and default name `""`.
//     ///
//     /// # Panics
//     ///
//     /// - Panics if no single instance is registered for the given type and default name `""`.
//     ///
//     /// # Example
//     ///
//     /// ```rust
//     /// use rudi::{Context, Singleton};
//     ///
//     /// #[derive(Clone, Debug)]
//     /// #[Singleton(eager_create)]
//     /// struct A;
//     ///
//     /// # fn main() {
//     /// let cx = Context::auto_register();
//     /// let a = cx.get_single::<A>();
//     /// assert_eq!(format!("{:?}", a), "A");
//     /// # }
//     /// ```
//     #[track_caller]
//     pub fn get_single<T: 'static>(&self) -> &T {
//         self.get_single_with_name("")
//     }

//     /// Returns a reference to a [`Singleton`](crate::Scope::Singleton) or [`SingleOwner`](crate::Scope::SingleOwner) instance based on the given type and name.
//     ///
//     /// # Panics
//     ///
//     /// - Panics if no single instance is registered for the given type and name.
//     ///
//     /// # Example
//     ///
//     /// ```rust
//     /// use rudi::{Context, Singleton};
//     ///
//     /// #[derive(Clone, Debug)]
//     /// #[Singleton(eager_create, name = "a")]
//     /// struct A;
//     ///
//     /// # fn main() {
//     /// let cx = Context::auto_register();
//     /// let a = cx.get_single_with_name::<A>("a");
//     /// assert_eq!(format!("{:?}", a), "A");
//     /// # }
//     /// ```
//     #[track_caller]
//     pub fn get_single_with_name<T: 'static>(&self, name: impl Into<Cow<'static, str>>) -> &T {
//         let key = Key::new::<T>(name.into());
//         self.single_registry
//             .get_ref(&key)
//             .unwrap_or_else(|| panic!("no instance registered for: {:?}", key))
//     }

//     /// Returns an optional reference to a [`Singleton`](crate::Scope::Singleton) or [`SingleOwner`](crate::Scope::SingleOwner) instance based on the given type and default name `""`.
//     ///
//     /// # Example
//     ///
//     /// ```rust
//     /// use rudi::{Context, Singleton};
//     ///
//     /// #[derive(Clone, Debug)]
//     /// #[Singleton(eager_create)]
//     /// struct A;
//     ///
//     /// # fn main() {
//     /// let cx = Context::auto_register();
//     /// assert!(cx.get_single_option::<A>().is_some());
//     /// # }
//     /// ```
//     pub fn get_single_option<T: 'static>(&self) -> Option<&T> {
//         self.get_single_option_with_name("")
//     }

//     /// Returns an optional reference to a [`Singleton`](crate::Scope::Singleton) or [`SingleOwner`](crate::Scope::SingleOwner) instance based on the given type and name.
//     ///
//     /// # Example
//     ///
//     /// ```rust
//     /// use rudi::{Context, Singleton};
//     ///
//     /// #[derive(Clone, Debug)]
//     /// #[Singleton(eager_create, name = "a")]
//     /// struct A;
//     ///
//     /// # fn main() {
//     /// let cx = Context::auto_register();
//     /// assert!(cx.get_single_option_with_name::<A>("a").is_some());
//     /// # }
//     /// ```
//     pub fn get_single_option_with_name<T: 'static>(
//         &self,
//         name: impl Into<Cow<'static, str>>,
//     ) -> Option<&T> {
//         let key = Key::new::<T>(name.into());
//         self.single_registry.get_ref(&key)
//     }

//     /// Returns a collection of references to [`Singleton`](crate::Scope::Singleton) and [`SingleOwner`](crate::Scope::SingleOwner) instances based on the given type.
//     ///
//     /// # Example
//     ///
//     /// ```rust
//     /// use rudi::{Context, Singleton};
//     ///
//     /// #[Singleton(eager_create, name = "a")]
//     /// fn A() -> i32 {
//     ///     1
//     /// }
//     ///
//     /// #[Singleton(eager_create, name = "b")]
//     /// fn B() -> i32 {
//     ///     2
//     /// }
//     ///
//     /// fn main() {
//     ///     let cx = Context::auto_register();
//     ///     assert_eq!(cx.get_singles_by_type::<i32>().into_iter().sum::<i32>(), 3);
//     /// }
//     /// ```
//     pub fn get_singles_by_type<T: 'static>(&self) -> Vec<&T> {
//         let type_id = TypeId::of::<T>();
//         self.single_registry()
//             .iter()
//             .filter(|(key, _)| key.ty.id == type_id)
//             .filter_map(|(_, instance)| instance.as_single())
//             .map(|instance| instance.get_ref())
//             .collect()
//     }

      
//     /// Appends a standalone [`Singleton`](crate::Scope::Singleton) instance to the context with default name `""`.
//     ///
//     /// # Panics
//     ///
//     /// - Panics if a `Provider<T>` with the same name as the inserted instance exists in the `Context` and the context's [`allow_override`](Context::allow_override) is false.
//     ///
//     /// # Example
//     ///
//     /// ```rust
//     /// use rudi::Context;
//     ///
//     /// # fn main() {
//     /// let mut cx = Context::default();
//     /// cx.insert_singleton(42);
//     /// assert_eq!(cx.get_single::<i32>(), &42);
//     /// # }
//     /// ```
//     #[track_caller]
//     pub fn insert_singleton_with_type<T>(&mut self, instance: T)
//     where
//         T: 'static + Clone + Send + Sync,
//     {
//         self.insert_singleton_with_name(instance, "");
//     }


//     /// Appends a standalone [`Singleton`](crate::Scope::Singleton) instance to the context with name.
//     ///
//     /// # Panics
//     ///
//     /// - Panics if a `Provider<T>` with the same name as the inserted instance exists in the `Context` and the context's [`allow_override`](Context::allow_override) is false.
//     ///
//     /// # Example
//     ///
//     /// ```rust
//     /// use rudi::Context;
//     ///
//     /// # fn main() {
//     /// let mut cx = Context::default();
//     ///
//     /// cx.insert_singleton_with_name(1, "one");
//     /// cx.insert_singleton_with_name(2, "two");
//     ///
//     /// assert_eq!(cx.get_single_with_name::<i32>("one"), &1);
//     /// assert_eq!(cx.get_single_with_name::<i32>("two"), &2);
//     /// # }
//     /// ```
//     #[track_caller]
//     pub fn insert_singleton_with_name<T, N>(&mut self, instance: T, name: N)
//     where
//         T: 'static + Clone + Send + Sync,
//         N: Into<Cow<'static, str>>,
//     {
//         let provider: DynProvider =
//             Provider::<T>::never_construct(name.into(), Scope::Singleton).into();
//         let single = Single::new(instance, Some(Clone::clone)).into();

//         let key = provider.key().clone();
//         self.provider_registry.insert(provider, self.allow_override);
//         self.single_registry.insert(key, single);
//     }
    
// }