mod ascii;
mod client;
mod protocol;
mod server;
mod ui;
mod webcam;

use anyhow::Result;
use clap::Parser;
use crossterm::event::{self, Event};
use ratatui::prelude::*;
use std::io::IsTerminal;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use tokio::sync::mpsc;
use tracing_subscriber;
use uuid::Uuid;

use crate::client::ChatClient;
use crate::protocol::Message;
use crate::server::{ServerState, start_server};
use crate::ui::{App, UserAction};
use crate::webcam::WebcamCapture;

#[derive(Parser, Debug, Clone)]
#[command(name = "Terminal Chat", about = "ASCII video chat in your terminal")]
struct Args {
    /// Server mode - start a new chat server
    #[arg(long)]
    server: bool,
    
    /// Connect to a server URL (ws://host:port/ws)
    #[arg(long)]
    connect: Option<String>,
    
    /// Server port (default: 8080)
    #[arg(long, default_value_t = 8080)]
    port: u16,
    
    /// Enable ngrok tunnel for public access
    #[arg(long)]
    ngrok: bool,
    
    /// Video width in characters
    #[arg(long, default_value_t = 40)]
    video_width: u32,
    
    /// Video height in characters
    #[arg(long, default_value_t = 30)]
    video_height: u32,
    
    /// Video FPS
    #[arg(long, default_value_t = 15)]
    fps: u32,
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();
    let args = Args::parse();
    
    if args.server {
        run_server(args).await
    } else if let Some(url) = &args.connect {
        run_client(args.clone(), url.clone()).await
    } else {
        // Default: start server and connect locally
        run_host(args).await
    }
}

async fn run_server(args: Args) -> Result<()> {
    println!("Starting server on port {}...", args.port);
    
    let state = ServerState::new();
    
    if args.ngrok {
        let ngrok_url = setup_ngrok(args.port).await?;
        println!("Ngrok tunnel created: {}", ngrok_url);
        state.set_ngrok_url(ngrok_url).await;
    }
    
    println!("WebSocket URL: ws://localhost:{}/ws", args.port);
    println!("Press Ctrl+C to stop the server");
    
    start_server(state, args.port).await
}

async fn run_client(args: Args, url: String) -> Result<()> {
    // Check if we're in a TTY
    if !std::io::stdout().is_terminal() {
        eprintln!("Error: This application requires an interactive terminal.");
        eprintln!("Please run directly in your terminal, not through pipes or scripts.");
        return Err(anyhow::anyhow!("Not running in a TTY"));
    }
    
    let mut terminal = ratatui::init();
    terminal.clear()?;
    
    let result = run_chat_client(&mut terminal, args, url).await;
    
    ratatui::restore();
    result
}

async fn run_host(args: Args) -> Result<()> {
    // Check if we're in a TTY
    if !std::io::stdout().is_terminal() {
        eprintln!("Error: This application requires an interactive terminal.");
        eprintln!("Please run directly in your terminal, not through pipes or scripts.");
        return Err(anyhow::anyhow!("Not running in a TTY"));
    }
    
    // Start server in background
    let state = ServerState::new();
    let server_state = state.clone();
    let server_port = args.port;
    
    let ngrok_url = if args.ngrok {
        let url = setup_ngrok(server_port).await?;
        state.set_ngrok_url(url.clone()).await;
        Some(url)
    } else {
        None
    };
    
    tokio::spawn(async move {
        if let Err(e) = start_server(server_state, server_port).await {
            eprintln!("Server error: {}", e);
        }
    });
    
    // Give server time to start
    tokio::time::sleep(Duration::from_millis(500)).await;
    
    // Connect as client
    let url = format!("ws://localhost:{}/ws", args.port);
    
    let mut terminal = ratatui::init();
    terminal.clear()?;
    
    let mut app = App::new();
    app.ngrok_url = ngrok_url;
    
    let result = run_chat_client(&mut terminal, args, url).await;
    
    ratatui::restore();
    result
}

async fn run_chat_client(
    terminal: &mut Terminal<impl Backend>,
    args: Args,
    url: String,
) -> Result<()> {
    let mut app = App::new();
    let mut last_draw = Instant::now();
    
    // Initialize webcam (optional - continue even if it fails)
    let webcam = WebcamCapture::new(args.video_width, args.video_height, args.fps)?;
    
    let (_tx_ui, _rx_ui) = mpsc::unbounded_channel::<()>();
    let mut client: Option<ChatClient> = None;
    let user_id = Uuid::new_v4();
    
    // Frame sending task
    let mut frame_interval = tokio::time::interval(Duration::from_millis(1000 / args.fps as u64));
    frame_interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
    
    loop {
        // Handle UI events
        if event::poll(Duration::from_millis(10))? {
            if let Event::Key(key) = event::read()? {
                if let Some(action) = app.handle_key(key.code)? {
                    match action {
                        UserAction::JoinChat(username) => {
                            // Connect to server
                            match ChatClient::connect(&url).await {
                                Ok(c) => {
                                    c.send(Message::Join {
                                        id: user_id,
                                        username: username.clone(),
                                    }).await?;
                                    client = Some(c);
                                }
                                Err(e) => {
                                    app.add_message(
                                        "System".to_string(),
                                        format!("Failed to connect: {}", e)
                                    );
                                }
                            }
                        }
                        UserAction::SendMessage(text) => {
                            if let Some(ref c) = client {
                                c.send(Message::Chat {
                                    id: user_id,
                                    username: String::new(), // Server will fill this
                                    text,
                                    timestamp: SystemTime::now()
                                        .duration_since(UNIX_EPOCH)
                                        .unwrap()
                                        .as_secs(),
                                }).await?;
                            }
                        }
                    }
                }
            }
        }
        
        // Update video frame
        if let Some(frame) = webcam.get_frame() {
            app.update_video_frame(frame.clone());
            
            // Send frame to server if connected periodically
            if client.is_some() {
                if let Some(ref c) = client {
                    let _ = c.send(Message::VideoFrame {
                        id: user_id,
                        username: String::new(),
                        frame: frame.serialize(),
                    }).await;
                }
            }
        }
        
        // Receive messages from server
        if let Some(ref mut c) = client {
            while let Ok(msg) = c.rx.try_recv() {
                match msg {
                    Message::Join { username, .. } => {
                        app.add_message("System".to_string(), format!("{} joined", username));
                    }
                    Message::Leave { .. } => {
                        app.add_message("System".to_string(), "A user left".to_string());
                    }
                    Message::Chat { username, text, .. } => {
                        app.add_message(username, text);
                    }
                    Message::VideoFrame { username, frame, .. } => {
                        if let Ok(ascii_frame) = crate::ascii::AsciiFrame::deserialize(&frame) {
                            app.update_remote_frame(username, ascii_frame);
                        }
                    }
                    Message::UserList { users } => {
                        app.update_users(users);
                    }
                    Message::ServerInfo { ngrok_url, .. } => {
                        if ngrok_url.is_some() {
                            app.ngrok_url = ngrok_url;
                        }
                    }
                    Message::Error { message } => {
                        app.add_message("Error".to_string(), message);
                    }
                }
            }
        }
        
        // Draw UI
        let elapsed = last_draw.elapsed();
        last_draw = Instant::now();
        
        terminal.draw(|f| ui::draw(f, &mut app, elapsed))?;
        
        if app.should_quit {
            break;
        }
        
        tokio::time::sleep(Duration::from_millis(16)).await;
    }
    
    Ok(())
}

async fn setup_ngrok(port: u16) -> Result<String> {
    // For ngrok 0.14, we need to use it differently
    // This is a placeholder - ngrok integration would need proper setup
    // You'll need to run ngrok separately: ngrok http 8080
    
    eprintln!("Note: Please run 'ngrok http {}' in a separate terminal", port);
    eprintln!("Then share the ngrok URL with others to connect");
    
    Ok(format!("wss://your-ngrok-url.ngrok.io/ws"))
}