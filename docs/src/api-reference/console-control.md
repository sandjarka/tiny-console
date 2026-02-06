# Console Control

| Method | Description |
|--------|-------------|
| `open_console()` | Open the console |
| `close_console()` | Close the console |
| `toggle_console()` | Toggle open/closed |
| `is_console_open() -> bool` | Check if console is open |
| `clear_console()` | Clear all output |
| `erase_history()` | Clear command history |

## Example

```gdscript
# Open the console programmatically
TinyConsole.open_console()

# Check state before acting
if TinyConsole.is_console_open():
    TinyConsole.close_console()

# Clear all output
TinyConsole.clear_console()
```
