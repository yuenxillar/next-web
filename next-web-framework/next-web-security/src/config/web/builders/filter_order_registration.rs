use std::collections::HashMap;

use crate::{core::filter::Filter, web::{access::{error_translation_filter::ErrorTranslationFilter, intercept::authorization_filter::AuthorizationFilter}, authentication::{logout::logout_filter::LogoutFilter, ui::default_login_page_generating_filter::DefaultLoginPageGeneratingFilter}}};

#[derive(Clone)]
pub struct FilterOrderRegistration {
    filter_to_order: HashMap<String, i32>,
}

impl FilterOrderRegistration {
    fn put<F: Filter>(&mut self, position: i32) {
        self.filter_to_order
            .entry(std::any::type_name::<F>().to_string())
            .or_insert(position);
    }

    fn get_order<F: Filter>(&self) -> Option<i32> {
        self.filter_to_order
            .get(std::any::type_name::<F>())
            .map(|v| *v)
    }
}

impl Default for FilterOrderRegistration {
    fn default() -> Self {
        let mut order = Step::new(100, 100);
        let mut filter_order = Self {
            filter_to_order: Default::default(),
        };


        filter_order.put::<LogoutFilter>(order.next());

        order.next();


        filter_order.put::<DefaultLoginPageGeneratingFilter>(order.next());
        // filter_order.put::<DefaultLogoutPageGeneratingFilter>(order.next());


        filter_order.put::<ErrorTranslationFilter>(order.next());
        filter_order.put::<AuthorizationFilter>(order.next());

        filter_order
    }
}

struct Step {
    value: i32,
    step_size: i32,
}

impl Step {
    fn new(value: i32, step_size: i32) -> Self {
        Self { value, step_size }
    }

    fn next(&mut self) -> i32 {
        let value = self.value;
        self.value += self.step_size;
        value
    }
}
