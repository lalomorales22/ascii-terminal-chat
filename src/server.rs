use anyhow::{Context, Result};
use axum::{
    extract::{
        State,
        ws::{WebSocketUpgrade, WebSocket, Message as WsMessage},
    },
    response::IntoResponse,
    routing::get,
    Router,
};
use futures_util::{SinkExt, StreamExt};
use std::{
    collections::HashMap,
    net::SocketAddr,
    sync::Arc,
    time::{SystemTime, UNIX_EPOCH},
};
use tokio::sync::{broadcast, RwLock};
use tower_http::cors::CorsLayer;
use uuid::Uuid;

use crate::protocol::{Message, UserInfo};

type Users = Arc<RwLock<HashMap<Uuid, UserInfo>>>;

#[derive(Clone)]
pub struct ServerState {
    pub users: Users,
    pub tx: broadcast::Sender<Message>,
    pub ngrok_url: Arc<RwLock<Option<String>>>,
}

impl ServerState {
    pub fn new() -> Self {
        let (tx, _) = broadcast::channel(100);
        Self {
            users: Arc::new(RwLock::new(HashMap::new())),
            tx,
            ngrok_url: Arc::new(RwLock::new(None)),
        }
    }

    pub async fn set_ngrok_url(&self, url: String) {
        *self.ngrok_url.write().await = Some(url);
    }
}

pub async fn start_server(state: ServerState, port: u16) -> Result<()> {
    let app = Router::new()
        .route("/ws", get(websocket_handler))
        .with_state(state)
        .layer(CorsLayer::permissive());

    let addr = SocketAddr::from(([127, 0, 0, 1], port));
    tracing::info!("WebSocket server listening on {}", addr);

    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .context("Failed to bind to address")?;

    axum::serve(listener, app)
        .await
        .context("Server error")?;

    Ok(())
}

async fn websocket_handler(
    ws: WebSocketUpgrade,
    State(state): State<ServerState>,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_socket(socket, state))
}

async fn handle_socket(socket: WebSocket, state: ServerState) {
    let (mut sender, mut receiver) = socket.split();
    let mut rx = state.tx.subscribe();
    let user_id = Uuid::new_v4();
    let mut username = String::new();

    // Send server info
    if let Ok(msg) = serde_json::to_string(&Message::ServerInfo {
        ngrok_url: state.ngrok_url.read().await.clone(),
        room_name: "Terminal Chat Room".to_string(),
    }) {
        let _ = sender.send(WsMessage::Text(msg)).await;
    }

    // Handle incoming messages
    let state_clone = state.clone();
    let recv_task = tokio::spawn(async move {
        while let Some(msg) = receiver.next().await {
            if let Ok(msg) = msg {
                if let WsMessage::Text(text) = msg {
                    if let Ok(message) = Message::from_bytes(text.as_bytes()) {
                        match message {
                            Message::Join { username: name, .. } => {
                                username = name.clone();
                                
                                // Add user to list
                                let user_info = UserInfo {
                                    id: user_id,
                                    username: username.clone(),
                                    joined_at: SystemTime::now()
                                        .duration_since(UNIX_EPOCH)
                                        .unwrap()
                                        .as_secs(),
                                };
                                
                                state_clone.users.write().await.insert(user_id, user_info.clone());
                                
                                // Broadcast join message
                                let _ = state_clone.tx.send(Message::Join {
                                    id: user_id,
                                    username: username.clone(),
                                });
                                
                                // Send user list to new user
                                let users: Vec<UserInfo> = state_clone.users.read().await
                                    .values()
                                    .cloned()
                                    .collect();
                                let _ = state_clone.tx.send(Message::UserList { users });
                            }
                            Message::Chat { text, .. } => {
                                if !username.is_empty() {
                                    let _ = state_clone.tx.send(Message::Chat {
                                        id: user_id,
                                        username: username.clone(),
                                        text,
                                        timestamp: SystemTime::now()
                                            .duration_since(UNIX_EPOCH)
                                            .unwrap()
                                            .as_secs(),
                                    });
                                }
                            }
                            Message::VideoFrame { frame, .. } => {
                                if !username.is_empty() {
                                    let _ = state_clone.tx.send(Message::VideoFrame {
                                        id: user_id,
                                        username: username.clone(),
                                        frame,
                                    });
                                }
                            }
                            _ => {}
                        }
                    }
                }
            }
        }
    });

    // Broadcast messages to this client
    let send_task = tokio::spawn(async move {
        while let Ok(msg) = rx.recv().await {
            if let Ok(text) = serde_json::to_string(&msg) {
                if sender.send(WsMessage::Text(text)).await.is_err() {
                    break;
                }
            }
        }
    });

    // Wait for tasks to complete
    tokio::select! {
        _ = recv_task => {},
        _ = send_task => {},
    }

    // Clean up on disconnect
    state.users.write().await.remove(&user_id);
    let _ = state.tx.send(Message::Leave { id: user_id });
}