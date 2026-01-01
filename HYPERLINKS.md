# Hyperlink Support

Your chat CLI now automatically detects URLs and makes them clickable in supported terminals!

## How It Works

The CLI automatically detects your terminal capabilities on startup:
- **Kitty** - Full hyperlink support ✓
- **GNOME Terminal** - Full hyperlink support ✓
- **Alacritty** - Full hyperlink support ✓
- **iTerm2** - Full hyperlink support ✓
- **VSCode Terminal** - Full hyperlink support ✓
- **Basic xterm** - Fallback mode (shows URLs as plain text)

## Terminal Detection

When you start the CLI, it will show your terminal type:

```
Terminal: Modern (hyperlinks enabled ✓)
```

or

```
Terminal: Basic (basic mode)
```

## Supported Terminals

### ✅ Full Support (Clickable Links)
- Kitty
- GNOME Terminal (gnome-terminal)
- Alacritty
- iTerm2 (macOS)
- VSCode integrated terminal
- Windows Terminal
- Hyper
- Warp
- Any terminal with 256-color support

### ⚠️ Fallback (Plain Text URLs)
- Basic xterm
- Older terminals without color support

## Using Hyperlinks

Just type or paste a URL in your message:

```
You: Check out https://github.com
Assistant: You said: Check out https://github.com
```

In supported terminals, the URL will be:
- **Clickable** - Ctrl+Click or Cmd+Click to open
- **Underlined** (in most terminals)
- **Color-coded** (if your terminal supports it)

## Supported URL Schemes

Currently detects:
- `https://` URLs
- `http://` URLs

Example URLs that will be linkified:
- `https://github.com/rust-lang/rust`
- `http://example.com`
- `https://docs.rs/ratatui`

## How Hyperlinks are Encoded

The CLI uses the **OSC 8 escape sequence** standard:
```
\x1b]8;;URL\x1b\\TEXT\x1b]8;;\x1b\\
```

This is a cross-terminal standard supported by:
- [Kitty](https://sw.kovidgoyal.net/kitty/open_actions/#hyperlinks)
- [GNOME Terminal](https://gitlab.gnome.org/GNOME/vte/-/issues/126)
- [iTerm2](https://iterm2.com/documentation-escape-codes.html)

## Graceful Degradation

If your terminal doesn't support hyperlinks:
- URLs are shown as **plain text** (still readable)
- No escape code garbage is displayed
- Everything works normally, just without clicking

## Testing

Try these commands to test hyperlink support:

```bash
# 1. Start the CLI
cargo run --release

# 2. Type a message with a URL
Check out https://github.com

# 3. In supported terminals:
#    - The URL should be underlined or highlighted
#    - Ctrl+Click (or Cmd+Click on macOS) opens the URL
```

## Checking Your Terminal

To see what terminal you're using:

```bash
# Check TERM variable
echo $TERM

# Check TERM_PROGRAM (macOS/some terminals)
echo $TERM_PROGRAM

# Common values:
# - xterm-kitty       → Kitty
# - xterm-256color    → Modern terminal
# - gnome-256color    → GNOME Terminal
# - screen-256color   → tmux/screen
```

## Future Enhancements

Potential improvements:
- [ ] Custom link text: `[Click here](https://url.com)`
- [ ] More URL schemes: `ftp://`, `mailto:`, `file://`
- [ ] Email detection and linking
- [ ] Phone number detection
- [ ] File path detection and linking

## Technical Details

**Terminal Detection** (src/main.rs:24-53):
- Checks `TERM` and `TERM_PROGRAM` environment variables
- Categorizes as Kitty, Modern, or Basic
- Sets capabilities flags

**URL Detection** (src/main.rs:298-352):
- Simple pattern matching for `http://` and `https://`
- Finds URL boundaries (whitespace, punctuation)
- Preserves original text formatting

**Link Encoding** (src/main.rs:293-296):
- Uses OSC 8 escape sequence
- URL and display text can differ
- Gracefully ignored by non-supporting terminals
