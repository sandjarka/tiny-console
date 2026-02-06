# Output

| Method | Description |
|--------|-------------|
| `info(text)` | Print informational message |
| `error(text)` | Print error with `ERROR:` prefix |
| `warn(text)` | Print warning with `WARNING:` prefix |
| `debug_msg(text)` | Print debug message |
| `print_line(text)` | Print raw line (supports BBCode) |
| `print_boxed(text)` | Print text in an ASCII art box |

## info

```gdscript
TinyConsole.info("Player health: %d" % health)
```

Prints a plain informational message using the default text color.

## error

```gdscript
TinyConsole.error("Failed to load save file")
```

Prints a message prefixed with `ERROR:` using the error color.

## warn

```gdscript
TinyConsole.warn("Config file not found, using defaults")
```

Prints a message prefixed with `WARNING:` using the warning color.

## debug_msg

```gdscript
TinyConsole.debug_msg("Spawned 3 enemies at position (10, 20)")
```

Prints a message prefixed with `DEBUG:` using the debug color.

## print_line

```gdscript
TinyConsole.print_line("[color=red]Custom[/color] [b]BBCode[/b] formatting")
```

Prints a raw line to the console. Supports Godot's BBCode tags for rich text formatting.

## print_boxed

```gdscript
TinyConsole.print_boxed("Important Notice")
```

Prints the text surrounded by an ASCII art box for emphasis.
