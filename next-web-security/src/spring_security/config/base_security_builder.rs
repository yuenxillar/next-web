use std::{
    marker::PhantomData,
    ops::DerefMut,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
};

#[derive(Clone)]
pub struct BaseSecurityBuilder<O>
where
    O: Send + Sync,
{
    pub(crate) building: Arc<AtomicBool>,

    _marker: PhantomData<O>,
}

impl<O> BaseSecurityBuilder<O>
where
    O: Send + Sync,
{
    pub fn new() -> Self {
        Self {
            building: Arc::new(AtomicBool::new(false)),
            _marker: PhantomData,
        }
    }

    pub fn build<C>(c: &mut C) -> O
    where
        C: DerefMut<Target = Self>,
        C: BaseSecurityBuilderExt<O>,
    {
        assert!(
            !c.building.swap(true, Ordering::SeqCst),
            "This object has already been built"
        );

        c.do_build()
    }
}

impl<O> Default for BaseSecurityBuilder<O>
where
    O: Send + Sync,
{
    fn default() -> Self {
        Self {
            building: Arc::new(Default::default()),
            _marker: PhantomData,
        }
    }
}

pub trait BaseSecurityBuilderExt<O>
where
    O: Send + Sync,
{
    fn do_build(&mut self) -> O;
}
