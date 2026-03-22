use next_web_core::traits::ordered::Ordered;

#[derive(Clone)]
pub struct OrderedCompositeItem<S> {
    pub ordered: Vec<S>,
}

impl<S> OrderedCompositeItem<S>
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

impl<S> Default for OrderedCompositeItem<S> {
    fn default() -> Self {
        Self {
            ordered: Vec::new(),
        }
    }
}
