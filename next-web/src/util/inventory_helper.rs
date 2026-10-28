use inventory::Collect;

/// A utility struct for working with the inventory.
#[derive(Default)]
pub struct InventoryHelper;

impl InventoryHelper {
    /// Returns an iterator over all items of type `T` in the inventory.
    pub fn iter<T>() -> impl IntoIterator<Item = &'static T>
    where
        T: Collect + 'static,
    {
        inventory::iter::<T>.into_iter()
    }

    /// Returns a vector of all items of type `T` in the inventory.
    pub fn vec<T>() -> Vec<&'static T>
    where
        T: Collect + 'static,
    {
        inventory::iter::<T>.into_iter().collect::<Vec<_>>()
    }

    /// Executes `func` on each item of type `T` in the inventory.
    pub fn with_action<T, F>(mut func: F)
    where
        T: Collect + 'static,
        F: FnMut(&T),
    {
        for item in inventory::iter::<T> {
            func(item);
        }
    }

    /// Returns the number of items of type `T` in the inventory.
    pub fn count<T>() -> usize
    where
        T: Collect + 'static,
    {
        inventory::iter::<T>.into_iter().count()
    }

    /// Executes `func` on each item of type `T` in the inventory asynchronously.
    pub async fn with_action_async<T, F>(mut func: F)
    where
        T: Collect + 'static,
        F: for<'a> AsyncFnMut(&'a T),
    {
        for item in inventory::iter::<T> {
            func(item).await;
        }
    }
}
