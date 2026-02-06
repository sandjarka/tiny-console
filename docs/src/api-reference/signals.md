# Signals

| Signal | Description |
|--------|-------------|
| `toggled(is_shown: bool)` | Emitted when the console is shown or hidden |

## Example

```gdscript
func _ready() -> void:
    TinyConsole.toggled.connect(_on_console_toggled)

func _on_console_toggled(is_shown: bool) -> void:
    if is_shown:
        Input.set_mouse_mode(Input.MOUSE_MODE_VISIBLE)
```
