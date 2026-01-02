# Scry CLI

A beautiful terminal-based chat interface built with Rust and Ratatui, featuring gradient borders, smooth scrolling, and optional Terminal Text Effects (TTE) integration.

## Features

- **Animated Welcome Screen** - Uses [Terminal Text Effects](https://github.com/ChrisBuilds/terminaltexteffects) for a stunning beams effect (optional)
- **Gradient Borders** - Purple->Blue chat area, Green->Cyan input area
- **Scrolling Support** - Navigate chat history with arrow keys, Page Up/Down, Home/End
- **Visual Scrollbar** - Gradient-colored scrollbar showing current position
- **Miami-Style Menu** - Popup menu with hot pink/cyan/orange gradients (Ctrl+P to open)
- **Interactive Input** - Full cursor support with backspace and arrow navigation
- **Echo Bot** - Responds to your messages (easily replaceable with AI/API calls)
- **TOML Configuration** - Customize colors and behavior via config file

## Installation

### Prerequisites

- Rust 1.70+ (install from [rustup.rs](https://rustup.rs))
- **Optional:** Python 3.8+ with `terminaltexteffects` for animated welcome screen

### Install Terminal Text Effects (Optional)

For the animated welcome screen with the beams effect:

```bash
pip install terminaltexteffects
```

If TTE is not installed, the app will display a simple welcome message instead.

### Build

```bash
cargo build --release
```

The compiled binary will be at `target/release/scry-cli`

### Run

```bash
# Run directly
cargo run --release

# Or run the compiled binary
./target/release/scry-cli
```

## Controls

### Input
- **Type** - Enter text
- **Enter** - Send message
- **Backspace** - Delete character
- **Left/Right** - Move cursor

### Scrolling
- **Up/Down** - Scroll up/down one message
- **Page Up/Down** - Scroll 10 messages
- **Home** - Jump to top
- **End** - Jump to bottom

### Menu
- **Ctrl+P** - Open/close menu
- **Up/Down** - Navigate menu items (when open)
- **Enter** - Select menu item
- **Esc** - Close menu

### Exit
- **Ctrl+C** or **Esc** - Quit

## Configuration

Scry CLI supports configuration via TOML file at `~/.config/scry-cli/config.toml`.

Copy the example config to get started:

```bash
mkdir -p ~/.config/scry-cli
cp docs/config.example.toml ~/.config/scry-cli/config.toml
```

### Configurable Options

**Colors:**
- Chat area gradient (start and end colors)
- Input area gradient (start and end colors)
- Miami banner colors (pink, purple, cyan, orange)

**Behavior:**
- `scroll_page_size` - Messages scrolled with Page Up/Down (default: 10)
- `animation_chars_per_frame` - Banner animation speed (default: 3)
- `animation_frame_ms` - Animation frame duration in ms (default: 16)
- `idle_poll_ms` - Event polling interval when idle (default: 100)

See [docs/config.example.toml](docs/config.example.toml) for the full example.

## Project Structure

```
scry-cli/
├── src/
│   ├── main.rs          # Entry point, terminal setup
│   ├── app.rs           # Application state and logic
│   ├── config.rs        # Configuration loading (TOML)
│   ├── input.rs         # Event handling and key bindings
│   ├── message.rs       # Message and Role types
│   ├── welcome.rs       # TTE welcome screen integration
│   └── ui/
│       ├── mod.rs       # UI module exports
│       ├── render.rs    # Main UI rendering
│       ├── menu.rs      # Menu overlay rendering
│       ├── gradient.rs  # Gradient color utilities
│       └── text.rs      # Text wrapping and styling
├── docs/
│   ├── config.example.toml  # Example configuration
│   ├── gradient_presets.md  # Color preset examples
│   ├── SCROLLING.md         # Scrolling documentation
│   ├── MENU.md              # Menu system documentation
│   └── NOTES.md             # Development notes
├── Cargo.toml
└── README.md
```

## Dependencies

### Rust
- [ratatui](https://github.com/ratatui-org/ratatui) - Terminal UI framework
- [crossterm](https://github.com/crossterm-rs/crossterm) - Terminal manipulation
- [serde](https://serde.rs/) + [toml](https://github.com/toml-rs/toml) - Configuration parsing
- [anyhow](https://github.com/dtolnay/anyhow) - Error handling
- [dirs](https://github.com/dirs-dev/dirs-rs) - Platform-specific directories

### Optional (Python)
- [terminaltexteffects](https://github.com/ChrisBuilds/terminaltexteffects) - Animated text effects

## License

MIT

## Screenshots

### Welcome Screen (with TTE)
The beams effect creates an animated reveal of the welcome text with Miami-inspired colors.

### Chat Interface
```
┌ Chat (Up/Down PgUp/PgDn Home/End to scroll, Ctrl+C to quit) ─┐
│Assistant: Welcome! Type a message and press Enter to chat.  │#
│           Press Ctrl+P for menu.                              ││
│                                                               ││
│You: Hello there!                                             ││
│                                                               ││
│Assistant: You said: Hello there!                             │v
└───────────────────────────────────────────────────────────────┘
┌ Your message ──────────────────────────────────────────────────┐
│What should I type?|                                            │
└────────────────────────────────────────────────────────────────┘
```

## Future Ideas

- [ ] Syntax highlighting for code blocks
- [ ] Message history persistence
- [ ] AI integration (OpenAI, Anthropic, etc.)
- [ ] Multi-agent workflow support
- [ ] Themes and color schemes
- [ ] Markdown rendering
- [ ] Additional TTE effects (configurable)

## Contributing

Feel free to open issues or submit PRs!
