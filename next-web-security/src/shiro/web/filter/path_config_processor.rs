use next_web_core::traits::required::Required;

use crate::web::filter::path_matching_filter::PathMatchingFilter;

pub trait PathConfigProcessor {
    fn process_path_config(&mut self, path: &str, config: &str);
}

impl<T> PathConfigProcessor for T
where
    T: Required<PathMatchingFilter>,
{
    fn process_path_config(&mut self, path: &str, config: &str) {
        // Implementation goes her
        self.get_mut_object().process_path_config(path, config);
    }
}
