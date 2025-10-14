
pub struct HttpMethodHandlerConfigurer {
}

impl HttpMethodHandlerConfigurer {
 
}

impl Default for HttpMethodHandlerConfigurer {
    fn default() -> Self {
        Self {
        }
    }
}


#[derive(Default)]
pub struct RouterContext {
    pub(crate) state: RouteState,
    pub(crate) index: usize,

    // pub(crate) has_idempotency: bool,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum RouteState {
    #[default]
    None,

    Default,
    /// This means the polling is over
    End,
}

impl RouterContext {
}

impl Iterator for RouterContext {
    type Item = RouteState;

    fn next(&mut self) -> Option<Self::Item> {
        let elements = [
            RouteState::Default,
            RouteState::End,
        ];

        if self.index >= elements.len() {
            return None;
        }

        self.state = elements[self.index];

        let result = Some(elements[self.index]);
        self.index += 1;

        result
    }
}