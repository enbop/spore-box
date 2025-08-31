# Spore Box

A lightweight, self-hosted file sharing platform designed for personal use across multiple devices. Chat-like interface for seamless file transfer accessible through a web browser.

## Features

- **Multi-device sync**: Chat-like interface for sharing files between devices
- **File support**: Text messages, images, and file uploads/downloads
- **Markdown support**: Rich text rendering with code syntax highlighting
- **Temporary storage**: Data archived after 30 days, with recycle bin (30-day retention)
- **Device identification**: Different senders shown based on device name in URL parameter
- **Quick paste**: Browser clipboard integration for rapid file sharing

## Development

### Frontend Development

```bash
cd frontend
yarn install
yarn start     # Development server
yarn build     # Production build
```

### Backend Development

Build the Rust WASM backend:
```bash
cargo build --target=wasm32-wasip2 -r
```

### Running the Server

Start the WASM server with data directory access:
```bash
wasmtime serve --addr=0.0.0.0:8081 -Scli --dir data ./target/wasm32-wasip2/release/spore-box.wasm
```

Access the application at: http://localhost:8081

### Usage

Add device name to URL for identification:
```
http://localhost:8081?device=iPhone
http://localhost:8081?device=MacBook
```

## Tech Stack

- **Frontend**: React + TypeScript + Tailwind CSS
- **Backend**: Rust + WASI (WebAssembly)
- **Storage**: JSONL files for messages, filesystem for file uploads
- **UI Components**: Custom chat interface with Markdown/code highlighting
