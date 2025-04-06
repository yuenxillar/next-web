mod web_dev_tests {
    use std::{
        any::{Any, TypeId},
        sync::{atomic::AtomicBool, Arc},
    };

    #[cfg(feature = "redis_enabled")]
    use deadpool_redis::redis::AsyncCommands;

    #[tokio::test]
    async fn test_web_dev() -> Result<(), Box<dyn std::error::Error>> {
        #[cfg(feature = "redis_enabled")]
        use deadpool_redis::redis::AsyncConnectionConfig;
        #[cfg(feature = "redis_enabled")]
        use deadpool_redis::redis::Client;
        #[cfg(feature = "redis_enabled")]
        use deadpool_redis::redis::PushKind;

        #[cfg(feature = "redis_enabled")]
        {
            let client = Client::open(format!(
                "{}/?protocol=resp3",
                "redis://:KWJKLxnasndkznaks.125334ajwbxajsakjd@192.168.18.164:6379/1"
            ))?;
            let mut con = client.get_connection()?;
            let mut pubsub = con.as_pubsub();
            pubsub.subscribe("__keyevent@1__:expired")?;
            let stop = Arc::new(AtomicBool::new(true));

            let var = stop.clone();
            std::thread::spawn(move || {
                std::thread::sleep(std::time::Duration::from_secs(30));
                var.store(false, std::sync::atomic::Ordering::Relaxed);
            });
            while stop.load(std::sync::atomic::Ordering::Relaxed) {
                let msg = pubsub.get_message().unwrap();
                let payload: String = msg.get_payload().unwrap();
                println!("channel '{}': {}", msg.get_channel_name(), payload);
            }

            println!("Subscribed to __keyevent@0__:expired successfully!");
        }
        Ok(())
    }

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

    }
}
