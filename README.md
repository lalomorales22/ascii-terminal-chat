# Terminal Chat — ASCII Video Chat
<img width="1089" height="515" alt="Screenshot 2025-08-15 at 10 54 19 PM" src="https://github.com/user-attachments/assets/642aa06d-290d-45d4-957a-eb6629cc440f" />
<img width="721" height="526" alt="Screenshot 2025-08-15 at 10 53 56 PM" src="https://github.com/user-attachments/assets/02f54996-aeda-4bff-bc56-b725b61735ae" />

Terminal-based group chat with animated ASCII “video.” Built in Rust with a three‑panel TUI and a WebSocket server.

Status: Stable. Uses a subtle animated test pattern (no real webcam yet) so it runs everywhere without extra setup.

## Features

- Multi-user chat over WebSockets
- Three-panel terminal UI: video • messages • users
- Animated ASCII video test pattern at configurable FPS
- Start server + client in one command, or run server-only
- Optional ngrok for easy remote access

## Requirements

- Rust 1.70+
- FFmpeg installed (required by ffmpeg-next crate)
- macOS/Linux/Windows terminal (interactive TTY required)

## Install

```bash
cargo build --release
```

## Run

- Host (server + client on localhost):
	```bash
	cargo run
	```

- Server only:
	```bash
	cargo run -- --server --port 8080
	```

- Connect to a server:
	```bash
	cargo run -- --connect ws://localhost:8080/ws
	```

CLI options:

- `--server`              Start server only
- `--connect <URL>`       Connect to an existing server (ws://…/ws or wss://…/ws)
- `--port <PORT>`         Server port (default: 8080)
- `--ngrok`               Show ngrok guidance (run ngrok separately)
- `--video-width <N>`     ASCII width in chars (default: 40)
- `--video-height <N>`    ASCII height in chars (default: 30)
- `--fps <N>`             Frame rate (default: 15)

Notes:

- The app must run in an interactive terminal (TTY). Running via pipes/scripts will exit with an error.
- The server currently binds to 127.0.0.1. Remote/LAN access requires a tunnel (see ngrok below).

## Multi-user

Same machine (multiple terminals):

```bash
# Terminal 1 — host
cargo run

# Terminal 2 — client
cargo run -- --connect ws://localhost:8080/ws

# Terminal 3 — client
cargo run -- --connect ws://localhost:8080/ws
```

Remote users (via ngrok):

```bash
# In another terminal, start a tunnel to your local server port
ngrok http 8080

# Start the app (server + client) locally
cargo run

# Share ngrok URL with others; they connect with
cargo run -- --connect wss://YOUR-NGROK-SUBDOMAIN.ngrok.io/ws
```

Tip: The built-in `--ngrok` flag only prints guidance. You still run ngrok as a separate process.

## UI & Controls

- Username screen: type name, Enter to join; Esc to quit
- Chat screen: type messages, Enter to send; Esc to quit
- Panels: left=your ASCII video (test pattern), center=messages, right=online users

## How it works

- Architecture: a lightweight WebSocket server relays chat and frames between clients
- Video: a smooth test pattern is generated and converted to ASCII each frame
- TUI: built with ratatui + crossterm, including simple visual effects

Protocol (JSON over WebSocket):

- `Join { id, username }`
- `Leave { id }`
- `Chat { id, username, text, timestamp }`
- `VideoFrame { id, username, frame }` // serialized ASCII frame bytes
- `UserList { users[] }`
- `ServerInfo { ngrok_url?, room_name }`

## Troubleshooting

- “Requires an interactive terminal”: run directly in Terminal/iTerm/PowerShell, not via pipes
- Can’t connect from another machine: server binds to localhost; use ngrok and connect to the wss URL
- Choppy animation: lower load with `--video-width 30 --video-height 20 --fps 10`

## Roadmap

- Real webcam capture (cross‑platform) instead of the test pattern
- Optional 0.0.0.0 binding for direct LAN access

—

This README consolidates the previous HOW_IT_WORKS.md, README_UPDATED.md, and USAGE.md.
