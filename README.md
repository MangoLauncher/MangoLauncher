# MangoLauncher

⚠️ **EXPERIMENTAL SOFTWARE** - This launcher is in early development stage and is highly experimental. Use at your own risk!

A fast, lightweight Minecraft launcher built in Rust with a terminal user interface (TUI). MangoLauncher provides a clean, efficient way to manage Minecraft versions, accounts, and game instances.

## Features

- **Terminal User Interface**: Clean, responsive TUI built with ratatui
- **Multi-threaded Downloads**: Fast parallel downloading of game files
- **Smart Caching**: Intelligent version manifest caching with automatic updates
- **Account Management**: Support for offline and Microsoft accounts
- **Instance Management**: Create and manage multiple game instances
- **Java Detection**: Automatic Java installation scanning and management
- **Progress Tracking**: Real-time download progress with cancellation support
- **Comprehensive Logging**: Detailed logging system for debugging
- **Cross-platform**: Runs on Windows, macOS, and Linux

## Installation

### Prerequisites

- Rust 1.70+ (latest stable recommended)
- Java 8+ (for running Minecraft)

### From Source

```bash
git clone https://github.com/MangoLauncher/MangoLauncher.git
cd MangoLauncher
cargo build --release
```

The binary will be available at `target/release/mango-launcher`.

## Usage

Run the launcher:

```bash
cargo run
```

Or use the compiled binary:

```bash
./target/release/mango-launcher
```

### Navigation

- **Arrow Keys**: Navigate through menus and lists
- **Enter**: Select/Download versions, launch instances
- **Tab**: Switch between different sections
- **R**: Refresh version lists
- **F**: Force refresh (bypass cache)
- **T**: Toggle version display modes
- **A**: Add new accounts/instances
- **S**: Access settings
- **D**: Delete selected item
- **Esc**: Go back/Exit

### Key Sections

1. **Launcher**: Browse and download Minecraft versions
2. **Instances**: Manage game instances
3. **Accounts**: Handle player accounts
4. **Settings**: Configure Java installations and preferences
5. **Logs**: View application and game logs

## Architecture

MangoLauncher is built with a modular architecture:

- **UI Layer** (`ui.rs`): TUI interface using ratatui
- **App Layer** (`app.rs`): Application state management
- **Network Layer** (`network.rs`): HTTP client with progress tracking
- **Version Management** (`version.rs`): Minecraft version handling
- **Auth System** (`auth.rs`): Account management
- **Launch System** (`launch.rs`): Game launching and process management
- **Java Detection** (`java.rs`): Java installation discovery
- **Logging** (`logs.rs`): Comprehensive logging system

## Configuration

The launcher stores its data in:

- **Linux/macOS**: `~/.config/mango-launcher/`
- **Windows**: `%APPDATA%\mango-launcher\`

Configuration includes:
- Game versions and instances
- Account information
- Java installations
- Application settings
- Logs

## Development Status

⚠️ **This launcher is experimental** and includes:

- Basic Minecraft version downloading and launching
- Account management (offline accounts supported)
- Instance management
- Java detection and management
- Comprehensive logging

### Known Limitations

- Microsoft authentication may need improvements
- Mod support is limited
- Some edge cases in version handling
- UI could be more polished

### Planned Features

- Better mod support (Forge, Fabric)
- Enhanced Microsoft authentication
- Instance export/import
- Custom version support
- Resource pack management

## Contributing

Contributions are welcome! This is an experimental project, so:

1. Fork the repository
2. Create a feature branch
3. Make your changes
4. Test thoroughly
5. Submit a pull request

Please note that the codebase is still evolving rapidly.

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## Disclaimer

⚠️ **EXPERIMENTAL SOFTWARE WARNING**

This launcher is in active development and is considered experimental. It may:
- Have bugs that could affect your Minecraft installations
- Lose configuration data between updates
- Behave unexpectedly in certain scenarios
- Not be compatible with all Minecraft versions

Always backup your Minecraft data before using experimental launchers.

## Inspiration

MangoLauncher draws inspiration from existing launchers like PrismLauncher while focusing on:
- Performance and efficiency
- Clean, minimal interface
- Fast operation through Rust's performance
- Terminal-based workflow for power users

---

**Note**: This launcher is not affiliated with Mojang Studios or Microsoft.
