# Chat CLI

A beautiful terminal-based chat interface built with Rust and Ratatui, featuring gradient borders and smooth scrolling.

## Features

- 🎨 **Gradient Borders** - Purple→Blue chat area, Green→Cyan input area
- 📜 **Scrolling Support** - Navigate chat history with arrow keys, Page Up/Down, Home/End
- 📊 **Visual Scrollbar** - Gradient-colored scrollbar showing current position
- ⌨️ **Interactive Input** - Full cursor support with backspace and arrow navigation
- 🎯 **Echo Bot** - Responds to your messages (easily replaceable with AI/API calls)

## Installation

### Prerequisites

- Rust 1.70+ (install from [rustup.rs](https://rustup.rs))

### Build

```bash
cargo build --release
```

The compiled binary will be at `target/release/chat-cli`

### Run

```bash
# Run directly
cargo run --release

# Or run the compiled binary
./target/release/chat-cli
```

## Controls

### Input
- **Type** - Enter text
- **Enter** - Send message
- **Backspace** - Delete character
- **←/→** - Move cursor

### Scrolling
- **↑/↓** - Scroll up/down one message
- **Page Up/Down** - Scroll 10 messages
- **Home** - Jump to top
- **End** - Jump to bottom

### Exit
- **Ctrl+C** or **Esc** - Quit

## Customization

### Gradient Colors

Edit `src/main.rs` and change the RGB values in the `gradient_block()` calls:

```rust
// Chat area gradient (currently Purple → Blue)
(147, 51, 234),  // Start color
(59, 130, 246),  // End color

// Input area gradient (currently Green → Cyan)
(16, 185, 129),  // Start color
(6, 182, 212),   // End color
```

See `gradient_presets.md` for more color combinations!

### Message Handling

Replace the echo logic in `submit_message()` to integrate with:
- AI APIs (OpenAI, Anthropic, etc.)
- Chat servers
- Custom agents
- Multi-agent orchestration

## Project Structure

```
chat-cli/
├── src/
│   ├── main.rs                        # Main application
│   └── animated_gradient_example.rs   # Animation example
├── Cargo.toml                         # Dependencies
├── gradient_presets.md                # Color presets
├── SCROLLING.md                       # Scrolling documentation
└── README.md                          # This file
```

## Dependencies

- [ratatui](https://github.com/ratatui-org/ratatui) - Terminal UI framework
- [crossterm](https://github.com/crossterm-rs/crossterm) - Terminal manipulation

## License

MIT

## Screenshots

```
┌ Chat (↑↓ PgUp/PgDn Home/End to scroll, Ctrl+C to quit) ─┐
│Assistant: Hello! I'm an echo bot. Type something and   │█
│          I'll repeat it back to you.                    ││
│                                                          ││
│You: Hello there!                                        ││
│                                                          ││
│Assistant: You said: Hello there!                        │↓
└──────────────────────────────────────────────────────────┘
┌ Your message ─────────────────────────────────────────────┐
│What should I type?█                                       │
└───────────────────────────────────────────────────────────┘
```

## Future Ideas

- [ ] Animated gradients
- [ ] Syntax highlighting for code blocks
- [ ] Message history persistence
- [ ] AI integration (OpenAI, Anthropic, etc.)
- [ ] Multi-agent workflow support
- [ ] Themes and color schemes
- [ ] Markdown rendering

## Contributing

Feel free to open issues or submit PRs!
