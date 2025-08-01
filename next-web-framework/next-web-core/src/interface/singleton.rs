pub trait Singleton: Send + Sync {
    fn singleton_name(&self) -> String {
        let raw_name = std::any::type_name::<Self>();
        let name = raw_name.rsplit("::").next().unwrap_or_default();

        // Convert the first character to lowercase and concatenate with the rest of the string.
        let mut chars = name.chars();
        match chars.next() {
            Some(first_char) => {
                let mut singleton_name = String::with_capacity(name.len());
                singleton_name.extend(first_char.to_lowercase());
                singleton_name.push_str(chars.as_str());
                singleton_name
            }
            None => name.to_string(), // Fallback for an unlikely empty string case.
        }
    }

    fn group(&self) -> String {
        std::any::type_name::<Self>()
            .split("::")
            .next()
            .unwrap_or_default()
            .into()
    }
}
