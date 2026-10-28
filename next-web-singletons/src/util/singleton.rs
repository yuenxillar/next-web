pub struct SingletonUtil;

impl SingletonUtil {
    /// Returns the name a singleton of type `T` is registered with by default.
    ///
    /// The name is the last segment of the type name with its first character lower
    /// cased, so `MyService` becomes `myService` and
    /// `crate::service::MyService` becomes `myService` as well.
    ///
    /// # Type Parameters
    ///
    /// * `T` - The type whose default name is returned.
    pub fn default_singleton_name<T: ?Sized>() -> String {
        let type_name = std::any::type_name::<T>();
        let name = type_name.rsplit("::").next().unwrap_or_default();

        let mut characters = name.chars();
        match characters.next() {
            Some(first) => {
                let mut default_name = String::with_capacity(name.len());
                default_name.extend(first.to_lowercase());
                default_name.push_str(characters.as_str());
                default_name
            }
            None => name.to_string(),
        }
    }
}
