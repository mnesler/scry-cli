# Scrolling Features

Your chat CLI now has full scrolling support with a visual scrollbar!

## Scroll Controls

### Basic Scrolling
- **↑ (Up Arrow)** - Scroll up one message
- **↓ (Down Arrow)** - Scroll down one message

### Fast Scrolling
- **Page Up** - Scroll up 10 messages
- **Page Down** - Scroll down 10 messages

### Jump to Position
- **Home** - Jump to the top (first message)
- **End** - Jump to the bottom (latest message)

## Visual Scrollbar

The right side of the chat area shows a gradient scrollbar:
- **↑** - Top arrow (at the beginning)
- **│** - Track (scroll path)
- **█** - Thumb (current position indicator)
- **↓** - Bottom arrow (at the end)

The scrollbar color matches the gradient border (Purple → Blue).

## How It Works

- As you send more messages, the scrollbar thumb gets smaller
- The thumb position shows where you are in the chat history
- Scroll offset is displayed in the scrollbar state
- Messages are dynamically rendered based on scroll position

## Tips

1. **Auto-scroll**: When you send a new message, it appears at the bottom
2. **Navigate history**: Use Up/Down to browse older messages
3. **Quick jump**: Press Home to see the first message, End for the latest
4. **Smooth browsing**: Page Up/Down for faster navigation

## Example Usage

```
1. Start chatting and send 20+ messages
2. Press Home to go to the first message
3. Press ↓ a few times to scroll through history
4. Press End to jump back to the latest message
5. Use Page Up/Page Down for faster browsing
```

The scrollbar on the right shows your position in the chat!
