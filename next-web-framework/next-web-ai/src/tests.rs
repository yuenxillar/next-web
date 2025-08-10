#[cfg(test)]
mod ai_tests {
    use futures_util::StreamExt;

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
                content: "rust arc示例!".into(),
            }],
            model: ChatModel::Chat.get_name().into(),
            stream: true,
        };
        let resp = api.send(&req).await.unwrap();
        match resp {
            crate::ai::deep_seek::api::deep_seek_api::ChatApiRespnose::Entity(chat_completion) => {
                println!("chat_completion: {:?}", chat_completion);
            }
            crate::ai::deep_seek::api::deep_seek_api::ChatApiRespnose::Stream(mut stream) => {
                let mut index = 0;
                while let Some(chunk) = stream.next().await {
                    index += 1;
                    println!("chat_completion item{}: {:?} \n", index, chunk);
                }
            }
        }
    }

    #[test]
    fn test_show() {
        println!("{:?}", "data: ".as_bytes())
    }
}
