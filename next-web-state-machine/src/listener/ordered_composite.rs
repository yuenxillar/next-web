use next_web_core::traits::ordered::Ordered;

/// Composite item which can be used in other components which
/// may want to allow automatic and annotation based ordering.
/// Good use case is a list of listeners where user may want
/// to place some of them to be processed before the others.
#[derive(Clone)]
pub struct OrderedComposite<S> {
    pub ordered: Vec<S>,
}

impl<S> OrderedComposite<S>
where
    S: Ordered + Eq,
{
    /// Public setter for the items.
    pub fn set_items(&mut self, items: Vec<S>) {
        self.ordered.clear();

        for item in items {
            self.add(item);
        }
    }

    /// Unregister item.
    pub fn remove(&mut self, item: &S) {
        self.ordered.retain(|i| item == i);
    }

    /// Register additional item.
    pub fn add(&mut self, item: S) {
        if !self.ordered.contains(&item) {
            self.ordered.push(item);
            self.ordered.sort_by(|a, b| a.order().cmp(&b.order()));
        }
    }
}

impl<S> Default for OrderedComposite<S> {
    fn default() -> Self {
        Self {
            ordered: Vec::new(),
        }
    }
}
