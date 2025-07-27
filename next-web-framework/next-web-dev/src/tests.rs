mod web_dev_tests {
    use std::sync::Arc;

    #[test]
    fn test_tr() {
        trait Test<A: Default + std::any::Any> {
            fn id(&self) -> std::any::TypeId {
                A::default().type_id()
            }
        }

        #[derive(Default, Clone)]
        struct A;
        impl Test<B> for A {}

        #[derive(Default)]
        struct B;
        impl Test<A> for B {}

        let a = Box::new(A);
        let b = Box::new(B);

        println!("a: {:?}, b: {:?}", a.id(), b.id());
        use axum::body::Bytes;

        let _bytes = Bytes::from_static(b"bytes");
    }

    #[test]
    fn file_len() {
        let file = std::fs::metadata(r"D:\ObsVideo\test.mkv").unwrap();
        println!("file len: {}", file.len());
    }

    async fn test_retry(s: String, b: &i32, c: Arc<String>) -> Result<(), String> {
        println!("s: {}, b: {}, c: {}", s, b, c);

        Ok(())
    }


    fn test_retry_a(s: String, b: &i32, c: Arc<String>) -> std::result::Result<(), String> {
        println!("s: {}, b: {}, c: {}", s, b, c);

        Ok(())
    }

 


    async fn test_a(s: String, b: &i32, c: Arc<String>) {
        let max_attempts: u8 = 3;
        let delay = 1000;

        let retry_count: u8 = 0;

        while retry_count < max_attempts {
            if retry_count > 0 {
                tokio::time::sleep(std::time::Duration::from_millis(delay)).await;
            }

            let s = s.clone();
            let c = c.clone();

            let block = async move { test_retry(s, b, c).await };
            // let block1 = { test_retry_a(s, b, c) };

            let result = block.await;

            if let Err(e) = result {
                
                continue;
            }

            if let Ok(result) = result {}
        }
    }
}
