#[cfg(test)]
mod ai_tests {
    use crate::{
        ai::deep_seek::{
            api::deep_seek_api::{ChatCompletionMessage, ChatCompletionRequest, DeepSeekApi},
            chat_model::ChatModel,
        },
        model::model_description::ModelDescription,
    };

    #[tokio::test]
    async fn test_deep_seek() {
        let api = DeepSeekApi::default();
        let req = ChatCompletionRequest {
            messages: vec![ChatCompletionMessage {
                role: "user".into(),
                content: "hello".into(),
            }],
            model: ChatModel::Chat.get_name().into(),
            stream: false,
        };
        let resp = api.send(&req).await.unwrap();
        println!("resp: {:?}", resp)
    }
}
