# Development Notes

## Hyperlinks - Why They Don't Work (Yet)

Attempted to add clickable hyperlinks using OSC 8 escape sequences, but **this doesn't work with Ratatui**.

### The Problem

Ratatui's text rendering (`Text`, `Span`, `Line` widgets) **escapes all control characters** for safety. When you try to embed OSC 8 hyperlink sequences like:

```rust
let link = format!("\x1b]8;;{}\x1b\\{}\x1b]8;;\x1b\\", url, text);
Span::styled(link, style);  // ❌ Doesn't work - escape codes are escaped!
```

Ratatui renders this as literal text, not as terminal escape sequences.

### Why Ratatui Escapes

- **Safety**: Prevents injection of malicious control sequences
- **Cross-platform**: Not all backends support all escape codes
- **Abstraction**: Ratatui abstracts the terminal, so you can't just inject raw sequences

### Possible Solutions

**Option 1: Wait for native support**
- Ratatui might add hyperlink support in the future
- Track: https://github.com/ratatui-org/ratatui/issues

**Option 2: Write raw sequences between frames**
```rust
// After terminal.draw():
use std::io::Write;
write!(
    terminal.backend_mut(),
    "\x1b[{};{}H\x1b]8;;{}\x1b\\{}\x1b]8;;\x1b\\",
    row, col, url, text
)?;
```
- Complex: Need to calculate exact cursor positions
- Fragile: Can break with resizing, scrolling
- Not recommended

**Option 3: Fork/patch Ratatui**
- Add hyperlink support to Ratatui itself
- Contribute upstream
- Most sustainable long-term solution

**Option 4: Use a different TUI framework**
- Look for frameworks with built-in hyperlink support
- May lose other Ratatui features

### What Works Instead

For now, the CLI shows URLs as **plain text**:
- Still readable
- User can manually copy-paste
- Works in all terminals
- Simple and reliable

### References

- [OSC 8 Hyperlink Specification](https://gist.github.com/egmontkob/eb114294efbcd5adb1944c9f3cb5feda)
- [Kitty Hyperlinks](https://sw.kovidgoyal.net/kitty/open_actions/#hyperlinks)
- [Ratatui Documentation](https://docs.rs/ratatui/)
