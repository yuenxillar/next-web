


pub trait ChatClient {
    
    fn generate(&self, message: &str) ->  impl std::future::Future<Output = Result<(), String>> + Send;

}


pub struct A;


impl ChatClient for A {
    async fn generate(&self, message: &str) ->  Result<(), String> {
        todo!()
    }
}