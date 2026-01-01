# Gradient Color Presets

Replace the RGB values in the `gradient_block()` calls to try different gradients:

## Cool Gradients

### Ocean (Blue → Teal)
```rust
(59, 130, 246),   // Blue
(20, 184, 166),   // Teal
```

### Sunset (Orange → Pink)
```rust
(251, 146, 60),   // Orange
(236, 72, 153),   // Pink
```

### Forest (Green → Dark Green)
```rust
(134, 239, 172),  // Light Green
(22, 163, 74),    // Dark Green
```

### Fire (Red → Yellow)
```rust
(239, 68, 68),    // Red
(251, 191, 36),   // Yellow
```

### Purple Haze (Purple → Magenta)
```rust
(147, 51, 234),   // Purple
(219, 39, 119),   // Magenta
```

### Neon (Cyan → Magenta)
```rust
(6, 182, 212),    // Cyan
(236, 72, 153),   // Magenta
```

### Monochrome (Gray → White)
```rust
(75, 85, 99),     // Gray
(229, 231, 235),  // Light Gray
```

### Matrix (Dark Green → Bright Green)
```rust
(20, 83, 45),     // Dark Green
(134, 239, 172),  // Bright Green
```

### Lava (Dark Red → Orange)
```rust
(127, 29, 29),    // Dark Red
(251, 146, 60),   // Orange
```

### Ice (Light Blue → White)
```rust
(147, 197, 253),  // Light Blue
(241, 245, 249),  // Almost White
```

## How to Use

In `src/main.rs`, find these lines:

```rust
// Chat area
(147, 51, 234),  // Purple
(59, 130, 246),  // Blue

// Input area
(16, 185, 129),  // Green
(6, 182, 212),   // Cyan
```

Replace them with any preset above!

## Custom Colors

Use any RGB values (0-255 for each channel):
```rust
(R, G, B),  // Start color
(R, G, B),  // End color
```

Example - Custom gradient from dark blue to gold:
```rust
(30, 58, 138),   // Dark Blue
(245, 158, 11),  // Gold
```
