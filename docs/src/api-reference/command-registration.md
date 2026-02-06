# Command Registration

| Method | Description |
|--------|-------------|
| `register_command(callable, name, description)` | Register a command |
| `unregister_command(name)` | Unregister a command |
| `has_command(name) -> bool` | Check if a command exists |
| `get_command_names(include_aliases) -> PackedStringArray` | List all commands |
| `get_command_description(name) -> String` | Get command description |
| `add_argument_autocomplete_source(command, argument_index, callable)` | Add autocomplete source for an argument (index 0-4) |

## register_command

```gdscript
TinyConsole.register_command(callable: Callable, name: String, description: String)
```

Registers a command that can be invoked from the console. Arguments are automatically parsed from the callable's signature. Supported types: `bool`, `int`, `float`, `String`, `Vector2`, `Vector3`, `Vector4`.

```gdscript
func _ready() -> void:
    TinyConsole.register_command(Callable(self, "greet"), "greet", "greet someone")

func greet(name: String) -> void:
    TinyConsole.info("Hello, %s!" % name)
```

Subcommands are supported by using spaces in the name:

```gdscript
TinyConsole.register_command(Callable(self, "multiply"), "math multiply", "multiply two numbers")
# Usage: math multiply 2 4
```

## add_argument_autocomplete_source

```gdscript
TinyConsole.add_argument_autocomplete_source(command: String, argument_index: int, callable: Callable)
```

Registers a callable that provides autocomplete suggestions for a specific argument (index 0-4). The callable should return an `Array` of strings.

```gdscript
TinyConsole.add_argument_autocomplete_source("teleport", 0,
    Callable(self, "get_locations"))

func get_locations() -> Array:
    return ["entrance", "caves", "boss"]
```
