use anyhow::Result;
use crossterm::event::KeyCode;
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Margin, Rect},
    prelude::*,
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, List, ListItem, Paragraph, Wrap},
};
use std::collections::VecDeque;
use tachyonfx::{fx, EffectManager, Interpolation};

use crate::ascii::AsciiFrame;
use crate::protocol::UserInfo;

pub enum AppState {
    UsernameEntry(String),
    Chat {
        _username: String,
        input_buffer: String,
        messages: VecDeque<ChatMessage>,
        users: Vec<UserInfo>,
        video_frame: Option<AsciiFrame>,
        remote_frames: Vec<(String, AsciiFrame)>,
    },
}

pub struct ChatMessage {
    pub username: String,
    pub text: String,
    pub timestamp: String,
}

pub struct App {
    pub state: AppState,
    pub effects: EffectManager<()>,
    pub should_quit: bool,
    pub ngrok_url: Option<String>,
}

impl App {
    pub fn new() -> Self {
        let mut effects = EffectManager::default();
        
        // Boot effect with CRT-style fade in
        let boot = fx::sequence(&[
            fx::fade_from(Color::Black, Color::Reset, (500, Interpolation::Linear)),
            fx::coalesce(300),
        ]);
        
        // Subtle color shift for retro feel
        let drift = fx::hsl_shift(
            Some([0.0, 0.0, 0.02]),
            None,
            (8000, Interpolation::SineInOut)
        );
        
        effects.add_effect(fx::parallel(&[boot, drift]));
        
        Self {
            state: AppState::UsernameEntry(String::new()),
            effects,
            should_quit: false,
            ngrok_url: None,
        }
    }

    pub fn handle_key(&mut self, key: KeyCode) -> Result<Option<UserAction>> {
        match &mut self.state {
            AppState::UsernameEntry(buffer) => {
                match key {
                    KeyCode::Enter => {
                        if !buffer.is_empty() {
                            let username = buffer.clone();
                            self.state = AppState::Chat {
                                _username: username.clone(),
                                input_buffer: String::new(),
                                messages: VecDeque::new(),
                                users: Vec::new(),
                                video_frame: None,
                                remote_frames: Vec::new(),
                            };
                            return Ok(Some(UserAction::JoinChat(username)));
                        }
                    }
                    KeyCode::Backspace => {
                        buffer.pop();
                    }
                    KeyCode::Char(c) => {
                        if buffer.len() < 20 {
                            buffer.push(c);
                        }
                    }
                    KeyCode::Esc => {
                        self.should_quit = true;
                    }
                    _ => {}
                }
            }
            AppState::Chat { input_buffer, .. } => {
                match key {
                    KeyCode::Enter => {
                        if !input_buffer.is_empty() {
                            let msg = input_buffer.clone();
                            input_buffer.clear();
                            return Ok(Some(UserAction::SendMessage(msg)));
                        }
                    }
                    KeyCode::Backspace => {
                        input_buffer.pop();
                    }
                    KeyCode::Char(c) => {
                        input_buffer.push(c);
                    }
                    KeyCode::Esc => {
                        self.should_quit = true;
                    }
                    _ => {}
                }
            }
        }
        Ok(None)
    }

    pub fn add_message(&mut self, username: String, text: String) {
        if let AppState::Chat { messages, .. } = &mut self.state {
            let timestamp = chrono::Local::now().format("%H:%M:%S").to_string();
            messages.push_back(ChatMessage {
                username,
                text,
                timestamp,
            });
            
            // Keep only last 100 messages
            while messages.len() > 100 {
                messages.pop_front();
            }
        }
    }

    pub fn update_users(&mut self, new_users: Vec<UserInfo>) {
        if let AppState::Chat { users, .. } = &mut self.state {
            *users = new_users;
        }
    }

    pub fn update_video_frame(&mut self, frame: AsciiFrame) {
        if let AppState::Chat { video_frame, .. } = &mut self.state {
            *video_frame = Some(frame);
        }
    }

    pub fn update_remote_frame(&mut self, username: String, frame: AsciiFrame) {
        if let AppState::Chat { remote_frames, .. } = &mut self.state {
            // Keep only latest frame per user
            remote_frames.retain(|(u, _)| u != &username);
            remote_frames.push((username, frame));
            
            // Limit to 4 remote videos
            if remote_frames.len() > 4 {
                remote_frames.remove(0);
            }
        }
    }
}

pub enum UserAction {
    JoinChat(String),
    SendMessage(String),
}

pub fn draw(f: &mut Frame, app: &mut App, elapsed: std::time::Duration) {
    let area = f.area();
    
    // Main border
    let block = Block::default()
        .title(" Terminal Chat // ASCII Vision ")
        .borders(Borders::ALL)
        .border_type(ratatui::widgets::BorderType::Thick)
        .style(Style::default().fg(Color::Cyan));
    
    let inner = block.inner(area);
    f.render_widget(block, area);
    
    match &app.state {
        AppState::UsernameEntry(buffer) => {
            draw_username_entry(f, inner, buffer, &app.ngrok_url);
        }
        AppState::Chat {
            input_buffer,
            messages,
            users,
            video_frame,
            ..
        } => {
            draw_chat(f, inner, input_buffer, messages, users, video_frame.as_ref());
        }
    }
    
    // Apply effects
    app.effects.process_effects(elapsed.into(), f.buffer_mut(), area);
}

fn draw_username_entry(f: &mut Frame, area: Rect, buffer: &str, ngrok_url: &Option<String>) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(2)
        .constraints([
            Constraint::Length(3),
            Constraint::Length(3),
            Constraint::Length(5),
            Constraint::Min(1),
        ])
        .split(area);
    
    let title = Paragraph::new("Welcome to Terminal Chat")
        .style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD))
        .alignment(Alignment::Center);
    
    let input = Paragraph::new(format!("Username: {}_", buffer))
        .style(Style::default().fg(Color::Green))
        .alignment(Alignment::Center)
        .block(Block::default().borders(Borders::ALL));
    
    let mut help_text = vec!["Enter your username to join the chat room".to_string()];
    if let Some(url) = ngrok_url {
        help_text.push(format!("Share this URL with others: {}", url));
    }
    help_text.push("Press ESC to quit".to_string());
    
    let help = Paragraph::new(help_text.join("\n"))
        .style(Style::default().fg(Color::Gray))
        .alignment(Alignment::Center)
        .wrap(Wrap { trim: true });
    
    f.render_widget(title, chunks[0]);
    f.render_widget(input, chunks[1]);
    f.render_widget(help, chunks[2]);
}

fn draw_chat(
    f: &mut Frame,
    area: Rect,
    input: &str,
    messages: &VecDeque<ChatMessage>,
    users: &[UserInfo],
    video_frame: Option<&AsciiFrame>,
) {
    // Layout: [Video | Chat | Users]
    let main_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(30), // Video
            Constraint::Percentage(50), // Chat
            Constraint::Percentage(20), // Users
        ])
        .split(area);
    
    // Video panel
    let video_block = Block::default()
        .title(" You ")
        .borders(Borders::ALL)
        .style(Style::default().fg(Color::Magenta));
    
    let video_area = video_block.inner(main_chunks[0]);
    f.render_widget(video_block, main_chunks[0]);
    
    if let Some(frame) = video_frame {
        render_ascii_frame(f, video_area, frame);
    } else {
        let loading = Paragraph::new("Camera loading...")
            .alignment(Alignment::Center)
            .style(Style::default().fg(Color::DarkGray));
        f.render_widget(loading, video_area);
    }
    
    // Chat panel
    let chat_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(1), Constraint::Length(3)])
        .split(main_chunks[1]);
    
    let messages_block = Block::default()
        .title(" Chat ")
        .borders(Borders::ALL)
        .style(Style::default().fg(Color::Green));
    
    let messages_area = messages_block.inner(chat_chunks[0]);
    f.render_widget(messages_block, chat_chunks[0]);
    
    // Render messages
    let msg_items: Vec<ListItem> = messages
        .iter()
        .map(|m| {
            let content = format!("[{}] {}: {}", m.timestamp, m.username, m.text);
            ListItem::new(content)
        })
        .collect();
    
    let messages_list = List::new(msg_items)
        .style(Style::default().fg(Color::White));
    
    f.render_widget(messages_list, messages_area);
    
    // Input box
    let input_block = Block::default()
        .borders(Borders::ALL)
        .style(Style::default().fg(Color::Yellow));
    
    let input_text = Paragraph::new(format!("> {}_", input))
        .style(Style::default().fg(Color::White));
    
    f.render_widget(input_block, chat_chunks[1]);
    f.render_widget(input_text, chat_chunks[1].inner(Margin::new(1, 0)));
    
    // Users panel
    let users_block = Block::default()
        .title(format!(" Users ({}) ", users.len()))
        .borders(Borders::ALL)
        .style(Style::default().fg(Color::Cyan));
    
    let users_area = users_block.inner(main_chunks[2]);
    f.render_widget(users_block, main_chunks[2]);
    
    let user_items: Vec<ListItem> = users
        .iter()
        .map(|u| ListItem::new(u.username.clone()))
        .collect();
    
    let users_list = List::new(user_items)
        .style(Style::default().fg(Color::White))
        .highlight_style(Style::default().fg(Color::Yellow));
    
    f.render_widget(users_list, users_area);
}

fn render_ascii_frame(f: &mut Frame, area: Rect, frame: &AsciiFrame) {
    let content_w = area.width.min(frame.width);
    let content_h = area.height.min(frame.height);
    
    let x0 = area.x + (area.width.saturating_sub(content_w)) / 2;
    let y0 = area.y + (area.height.saturating_sub(content_h)) / 2;
    
    let buf = f.buffer_mut();
    for y in 0..content_h {
        for x in 0..content_w {
            let idx = (y * frame.width + x) as usize;
            if idx < frame.cells.len() {
                let (ch, r, g, b) = frame.cells[idx];
                if let Some(cell) = buf.cell_mut((x0 + x, y0 + y)) {
                    cell.set_char(ch);
                    cell.set_fg(Color::Rgb(r, g, b));
                }
            }
        }
    }
}