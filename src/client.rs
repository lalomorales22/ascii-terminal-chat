use anyhow::{Context, Result};
use futures_util::{SinkExt, StreamExt};
use tokio::sync::mpsc;
use tokio_tungstenite::{connect_async, tungstenite::Message as WsMessage};

use crate::protocol::Message;

pub struct ChatClient {
    pub tx: mpsc::UnboundedSender<Message>,
    pub rx: mpsc::UnboundedReceiver<Message>,
}

impl ChatClient {
    pub async fn connect(url: &str) -> Result<Self> {
        let (ws_stream, _) = connect_async(url)
            .await
            .context("Failed to connect to server")?;
        
        let (write, read) = ws_stream.split();
        
        let (tx_to_ws, mut rx_from_app) = mpsc::unbounded_channel::<Message>();
        let (tx_to_app, rx_from_ws) = mpsc::unbounded_channel::<Message>();
        
        // Handle sending messages to WebSocket
        let tx_to_app_clone = tx_to_app.clone();
        tokio::spawn(async move {
            let mut write = write;
            while let Some(msg) = rx_from_app.recv().await {
                if let Ok(json) = serde_json::to_string(&msg) {
                    if write.send(WsMessage::Text(json)).await.is_err() {
                        let _ = tx_to_app_clone.send(Message::Error {
                            message: "Connection lost".to_string(),
                        });
                        break;
                    }
                }
            }
        });
        
        // Handle receiving messages from WebSocket
        tokio::spawn(async move {
            let mut read = read;
            while let Some(msg) = read.next().await {
                match msg {
                    Ok(WsMessage::Text(text)) => {
                        if let Ok(message) = Message::from_bytes(text.as_bytes()) {
                            if tx_to_app.send(message).is_err() {
                                break;
                            }
                        }
                    }
                    Ok(WsMessage::Close(_)) => {
                        let _ = tx_to_app.send(Message::Error {
                            message: "Server closed connection".to_string(),
                        });
                        break;
                    }
                    Err(e) => {
                        let _ = tx_to_app.send(Message::Error {
                            message: format!("WebSocket error: {}", e),
                        });
                        break;
                    }
                    _ => {}
                }
            }
        });
        
        Ok(Self {
            tx: tx_to_ws,
            rx: rx_from_ws,
        })
    }
    
    pub async fn send(&self, message: Message) -> Result<()> {
        self.tx.send(message)
            .map_err(|e| anyhow::anyhow!("Failed to send message: {}", e))
    }
    
    #[allow(dead_code)]
    pub async fn recv(&mut self) -> Option<Message> {
        self.rx.recv().await
    }
}