# QuBWic (QUIC/HTTP/3 Web Server)

QuBWic (QUIC Based Web) is Rust-based **QUIC/HTTP/3 exclusive web server** designed for high performance and ultra-low latency. It does not support legacy HTTP/1.x or HTTP/2, focusing entirely on the modern UDP-based HTTP/3 protocol.


## 🚀 Key Features

- QUIC/HTTP/3 Exclusive: Supports only latest QUIC/HTTP3 protocols to reduce complexity.
- Modular Architecture: Clearly separated layers for Configuration (Config), Connection management, and Protocol (HTTP/3) logic for better maintainability.
- Flexible Configuration: Fine-tune server behavior via `server.toml`, environment variables, or CLI arguments.


## 🛠 Build and Run

### Prerequisites
- Rust 1.75 or later (based on 2024 edition)
- OpenSSL or compatible libraries (Required for `quiche` build)

### Build
```bash
cargo build --release
```

### Run Server

By default, the server looks for a `server.toml` file in the current directory.

```bash
# Basic run (uses server.toml)
cargo run --release

# Specify a custom configuration file
cargo run --release -- -c custom_config.toml

# Override log level and server address
cargo run --release -- -l debug -a 127.0.0.1:4433
```

If you want to test client-side based on default 'server.toml' config, you can run following after server is running.
```bash
cargo run --example client
```


## 📂 Project Structure

- `src/config/`: Configuration loading and validation logic.
- `src/connection/`: QUIC connection state and packet handling.
- `src/http3/`: HTTP/3 stream processing and request routing.
- `tests/`: Integration test suites.
- `examples/`: Client implementation examples and samples.
