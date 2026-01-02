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

---

## OAuth Token Validation & Error Handling

### Overview

GitHub Copilot uses OAuth device flow for authentication. To prevent using invalid/expired tokens, the app implements:

1. **Session-scoped validation cache** - Tokens are validated once per app session
2. **Automatic retry with exponential backoff** - Handles transient auth errors gracefully
3. **Smart error recovery** - Preserves chat history when auth fails

### Token Lifecycle

**On first connection:**
1. User completes OAuth flow → Token saved to `~/.local/share/scry-cli/auth.json`
2. Token is **not validated** at save time (to avoid double requests)

**On reconnection (same session):**
1. Check validation cache → If cached as valid, connect instantly
2. If not cached, validate with minimal API request (`max_tokens: 1`)
3. Cache result in memory (cleared on app restart)

**On reconnection (new session):**
1. Load token from storage
2. Validate token (not cached from previous session)
3. Cache result for instant reconnects this session

### Error Handling During Chat

When a 401/403 error occurs during streaming:

1. **Retry with exponential backoff**: 2s, 4s, 8s delays (3 retries max)
2. **Clear cached Copilot token** after each retry (forces re-fetch from GitHub)
3. **After 3 failed retries**:
   - Emit `StreamEvent::AuthError`
   - Clear credentials from storage
   - Clear validation cache
   - Show toast: "Session expired. Please reconnect to continue chatting."
   - **Preserve chat history** (don't clear messages)
   - Set status to `NotConfigured`

### Implementation Details

**Key files:**
- `src/llm/copilot.rs`: `validate_token()`, retry logic in `stream_chat_with_retry()`
- `src/app.rs`: `validated_tokens` HashMap, `start_copilot_validation()`, AuthError handling
- `src/llm/mod.rs`: `StreamEvent::AuthError` variant

**Cache design:**
- Type: `HashMap<String, bool>` (storage_key → validated)
- Scope: Session only (cleared on app restart)
- When set: After successful validation
- When cleared: On auth error, app restart

**Why session-scoped?**
- Instant reconnects within the same session
- Re-validates on app restart (catches expired tokens)
- No persistent state to manage

**Rate limiting (429):**
- Treated as **valid token** (not an auth error)
- Validation succeeds even if rate limited

### User Experience

**Smooth reconnection:**
```
User: Ctrl+P → Connect → GitHub Copilot
App: (checks cache) → "Connected to GitHub Copilot with Claude Sonnet 4.5" (instant)
```

**First-time validation:**
```
User: Ctrl+P → Connect → GitHub Copilot
App: "Validating Copilot token..." (1-2s delay)
App: "Connected to GitHub Copilot with Claude Sonnet 4.5"
```

**Auth error during chat:**
```
User: (sends message)
App: (receives 401) → retries 2s, 4s, 8s → all fail
App: Toast: "Session expired. Please reconnect to continue chatting."
App: (chat history preserved, status = NotConfigured)
User: Ctrl+P → Connect → GitHub Copilot → OAuth flow
```

### Testing

**236 tests** covering:
- Token validation cache initialization
- Validation state with/without model
- OAuth token field accessibility
- Retry logic (implicit via Box::pin for recursion)
- Error handling (AuthError clears credentials)
