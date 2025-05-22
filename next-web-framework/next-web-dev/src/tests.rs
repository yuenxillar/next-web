mod web_dev_tests {
    use std::{
        any::{Any, TypeId},
        sync::{atomic::AtomicBool, Arc},
    };

    #[test]
    fn test_tr() {
        trait Test<A: Default + Any> {
            fn id(&self) -> TypeId {
                A::default().type_id()
            }
        }

        #[derive(Default,Clone)]
        struct A;
        impl Test<B> for A {}

        #[derive(Default)]
        struct B;
        impl Test<A> for B {}

        let a = Box::new(A);
        let b = Box::new(B);

        println!("a: {:?}, b: {:?}", a.id(), b.id());
        use axum::body::Bytes;

        let bytes = Bytes::from_static(b"bytes");
        let vec = bytes.to_vec();
    }
}
