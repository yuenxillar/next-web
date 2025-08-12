
#[cfg(test)]
mod tests {
    use crate::{service::message_source_service::MessageSourceService, util::locale::Locale};

    #[test]
    fn test_get_locale() {
        let locale = Locale::locale();
        sys_locale::get_locales().for_each(|e| println!("locale: {:?}", e));
    }

    #[test]
    fn test_fill_args() {
        let str1 = String::from("ben");
        let str2 = String::from("jack");
        println!("{}", MessageSourceService::fill_args("hello %s, hello %s!", &[str1.as_str(), str2.as_str()]));
    }
}