use futures::StreamExt;
use next_web_ai::{
    ai::deep_seek::{
        api::deep_seek_api::{
            ChatApiRespnose, ChatCompletionMessage, ChatCompletionRequest, DeepSeekApi,
        },
        chat_model::ChatModel,
    },
    model::model_description::ModelDescription,
};
use tokio::io::AsyncReadExt;

#[tokio::main]
async fn main() {
    println!("\n\nDeepSeek model: deepseek-chat\nPlease enter the prompt:");
    let mut std_in = tokio::io::stdin();
    let mut buf = [0; 1024];

    let api = DeepSeekApi::default();

    loop {
        std_in.read(&mut buf).await.unwrap();

        let req = ChatCompletionRequest::new(
            vec![ChatCompletionMessage::new(
                "user",
                String::from_utf8_lossy(&buf),
            )],
            ChatModel::Chat.get_name(),
            true,
        );
        let resp = api.send(&req).await.unwrap();
        match resp {
            ChatApiRespnose::Data(chat_completion) => {
                println!("chat_completion: {:?}", chat_completion);
            }
            ChatApiRespnose::DataStream(mut stream) => {
                while let Some(chunk) = stream.next().await {
                    if let Ok(chunk) = chunk {
                        chunk.iter().for_each(|str| {
                            print!(
                                "{}",
                                str.choices
                                    .iter()
                                    .map(|choice| choice
                                        .delta
                                        .as_ref()
                                        .map(|s| s.content.as_ref())
                                        .unwrap_or_default())
                                    .collect::<Vec<&str>>()
                                    .join("")
                            )
                        });
                    }
                }
            }
        }
    }
}
