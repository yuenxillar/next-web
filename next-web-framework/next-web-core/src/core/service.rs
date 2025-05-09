pub trait Service: Send + Sync {
    fn service_name(&self) -> String {
        let raw_name: String = std::any::type_name::<Self>().into();
        let mut service_name: String = raw_name.rsplit("::").next().unwrap_or_default().into();

        service_name.get_mut(0..1).map(|s| {
            s.make_ascii_lowercase();
            &*s
        });

        service_name
    }
}
