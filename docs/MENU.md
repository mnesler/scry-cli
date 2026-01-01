# Menu System

The chat CLI now includes a beautiful popup menu with Miami-style gradient colors!

## Opening the Menu

Press **Ctrl+P** to toggle the menu on/off at any time.

## Menu Navigation

When the menu is open:
- **↑/↓ Arrow Keys** - Navigate through menu items
- **Enter** - Select the highlighted item
- **Esc** - Close the menu without selecting
- **Ctrl+P** - Toggle menu on/off

## Menu Items

1. **Link Model** - (Coming soon) Connect to AI models
2. **Open Dashboard** - (Coming soon) View orchestrator dashboard
3. **Config Orcs** - (Coming soon) Configure orchestrators
4. **Exit** - Quit the application

Currently, only "Exit" is functional. Selecting other items will close the menu.

## Miami Gradient Design

The menu features vibrant Miami-style gradients:
- **Hot Pink** (#FF0080) → **Blue Violet** (#8A2BE2)
- **Cyan** (#00FFFF) → **Dark Orange** (#FF8C00)

### Visual Features

- **Selected Item**:
  - Bright gradient color based on position
  - **Bold** and **underlined** text
  - **▶** indicator arrow

- **Unselected Items**:
  - Subtle purple-cyan gradient
  - Normal weight text

- **Menu Border**:
  - Pink-to-cyan gradient
  - Dark background (RGB: 20, 20, 30)

- **Controls Hint**:
  - Color-coded shortcuts below menu
  - Cyan (↑↓), Pink (Enter), Orange (Esc)

## How It Works

The menu is rendered as an **overlay** on top of the chat interface:
1. Press Ctrl+P to activate
2. Menu appears centered on screen
3. Background chat remains visible but inactive
4. All arrow keys control menu navigation (not scrolling)
5. Close menu to resume normal chat operation

## Technical Details

- Menu state stored in `App.show_menu` and `App.menu_selected`
- Key handling switches between menu mode and normal mode
- Rendered using `render_menu()` as a popup overlay
- Uses same gradient system as borders
- Dark semi-transparent background for contrast

## Future Enhancements

Menu items will connect to:
- [ ] AI model selection and configuration
- [ ] Multi-agent orchestrator dashboard
- [ ] Configuration panels for agents
- [ ] Session management
- [ ] Settings and preferences
