use std::collections::HashMap;

use next_web_core::traits::filter::HttpFilter;

use crate::web::{
    access::{intercept::AuthorizationFilter, ErrorTranslationFilter},
    authentication::{
        logout::LogoutFilter,
        ui::default_login_page_generating_filter::DefaultLoginPageGeneratingFilter,
    },
};

#[derive(Clone)]
pub struct FilterOrderRegistration {
    filter_to_order: HashMap<String, i32>,
}

impl FilterOrderRegistration {
    pub fn put<F: HttpFilter>(&mut self, position: i32) {
        self.filter_to_order
            .entry(std::any::type_name::<F>().to_string())
            .or_insert(position);
    }

    pub fn get_order<F: HttpFilter>(&self) -> Option<i32> {
        self.filter_to_order
            .get(std::any::type_name::<F>())
            .map(|v| *v)
    }

    pub fn get_order_by_name(&self, filter_name: &str) -> Option<i32> {
        self.filter_to_order.get(filter_name).copied()
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
