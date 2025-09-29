use std::ops::{Deref, DerefMut};

pub struct Proxy<A, B, T> {
    before: Option<B>,
    target: T,
    after: Option<A>,
}


impl<A, B, T> Deref for Proxy<A, B, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        & self.target
    }
}

impl<A, B, T> DerefMut for Proxy<A, B, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.target
    }
}

impl<A, B, T> Clone for Proxy<A, B,T>
where
    A: Clone,
    B: Clone,
    T: Clone,
{
    fn clone(&self) -> Self {
        Proxy {
            before: self.before.clone(),
            target: self.target.clone(),
            after: self.after.clone(),
        }
    }
}

#[cfg(test)]
mod proxy_tests {
    
    use super::*;

    #[test]
    fn test_proxy() {
        let mut proxy = Proxy {
            before: Some(1),
            target: 2,
            after: Some(3),
        };

        assert_eq!(*proxy, 2);
        *proxy = 4;
        assert_eq!(*proxy, 4);

        let mut proxy2 = proxy.clone();
        assert_eq!(*proxy2, 4);
        *proxy2 = 5;
        assert_eq!(*proxy2, 5);
        assert_eq!(*proxy, 4);
    }
}