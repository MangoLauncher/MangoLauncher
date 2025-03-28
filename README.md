# Mango Launcher

A modern, open-source Minecraft launcher written in Rust with a beautiful terminal user interface.

## Features

- Beautiful TUI using ratatui
- Fast and memory-efficient
- Easy configuration through JSON
- Asynchronous downloads
- Version management
- Profile support
- Mod support (planned)

## Requirements

- Rust 1.75 or higher
- Cargo (comes with Rust)

## Installation

1. Clone the repository:
```bash
git clone https://github.com/yourusername/mango-launcher.git
cd mango-launcher
```

2. Build the project:
```bash
cargo build --release
```

3. Run the launcher:
```bash
cargo run --release
```

## Development

To set up the development environment:

1. Install Rust:
```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

2. Run tests:
```bash
cargo test
```

3. Format code:
```bash
cargo fmt
```

## License

This project is licensed under the MIT License - see the LICENSE file for details. 