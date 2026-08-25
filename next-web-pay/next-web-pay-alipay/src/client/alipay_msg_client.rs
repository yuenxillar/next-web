use std::{fmt::Display, sync::Arc, time::Duration};

use futures_util::{SinkExt, TryStreamExt};
use reqwest::Client;
use reqwest_websocket::{Message, WebSocket};
use tokio::sync::mpsc::{Receiver, Sender};

use crate::{AlipayError, client::AlipayClient, handler::AlipayMsgHandler};

#[derive(Clone)]
pub struct AlipayMsgClient {
    server_host: String,
    heartbeat_interval: Duration,
    msg_handler: Arc<dyn AlipayMsgHandler>,
    msg_sender: Option<Sender<Message>>,
    _client: AlipayClient,
}

impl AlipayMsgClient {
    pub fn new(client: AlipayClient, msg_handler: Arc<dyn AlipayMsgHandler>) -> Self {
        Self {
            server_host: "openchannel.alipay.com".into(),
            heartbeat_interval: Duration::from_secs(5),
            msg_handler,
            msg_sender: None,
            _client: client,
        }
    }

    pub fn with_server_host(mut self, server_host: impl Into<String>) -> Self {
        self.server_host = server_host.into();

        self
    }

    pub fn with_heartbeat_interval(mut self, interval: Duration) -> Self {
        self.heartbeat_interval = interval;

        self
    }
}

impl AlipayMsgClient {
    pub async fn connect(&mut self, protocol: Protocol) -> Result<(), AlipayError> {
        use reqwest_websocket::Upgrade;

        if self.msg_sender.is_some() {
            return Ok(());
        }

        let url = format!("{}://{}", protocol, self.server_host);

        match protocol {
            Protocol::Https => unimplemented!("Https not support"),
            Protocol::Wss => (),
        };

        let query = self._client.signed_params("", &(), None)?;

        let resp = Client::default()
            .get(&url)
            .query(&query)
            .upgrade() // Prepares the WebSocket upgrade.
            .send()
            .await
            .map_err(|err| AlipayError::Custom(err.to_string()))?;

        // Turns the response into a WebSocket stream.
        let mut ws = resp
            .into_websocket()
            .await
            .map_err(|err| AlipayError::Custom(err.to_string()))?;

        // Ping the server
        ws.send(Message::Ping(Default::default()))
            .await
            .map_err(|err| AlipayError::Custom(err.to_string()))?;

        let (tx, rx) = tokio::sync::mpsc::channel(100);
        self.msg_sender.replace(tx);

        tokio::spawn(on(
            ws,
            self.msg_handler.clone(),
            rx,
            self.heartbeat_interval,
        ));

        Ok(())
    }
}

async fn on(
    mut ws: WebSocket,
    msg_handler: Arc<dyn AlipayMsgHandler>,
    mut rx: Receiver<Message>,
    heartbeat_interval: Duration,
) {
    let mut interval = tokio::time::interval(heartbeat_interval);

    loop {
        tokio::select! {
            _ = interval.tick() => {
                // Ping the server
                ws.send(Message::Ping(Default::default()))
                    .await
                    .inspect_err(|err| tracing::error!("Ws error sending ping: {err}"))
                    .ok();
            }

            Some(message) = rx.recv() => {
                if let Err(err) = ws.send(message).await {
                    tracing::error!("Ws error sending message: {err}");
                    break;
                }
            }

            item = ws.try_next() => {
                if let Some(msg) = item.ok().unwrap_or_default() {
                    if let Message::Text(text) = msg {
                        if let Ok(value) = serde_json::from_str::<serde_json::Value>(&text) {
                            let msg_api = value["msg_api"]
                            .as_str()
                            .unwrap_or_default();
                            let msg_id = value["msg_id"]
                                .as_str()
                                .unwrap_or_default();
                            let biz_content = value["biz_content"]
                                .as_str()
                                .unwrap_or_default();

                            msg_handler.on_message(msg_api, msg_id, biz_content).await;
                        }
                    }
                }
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum Protocol {
    #[default]
    Wss,
    Https,
}

impl Display for Protocol {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Protocol::Wss => write!(f, "wss"),
            Protocol::Https => write!(f, "https"),
        }
    }
}
