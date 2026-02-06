# Installation

1. Copy the `addons/tiny_console/` directory into your project's `addons/` folder.
2. Reload the project. The `TinyConsole` singleton is registered automatically.

Toggle the console with the **backtick** key (`` ` ``, the key to the left of `1`).

## Quick Start

```gdscript
func _ready() -> void:
    TinyConsole.register_command(multiply, "multiply", "multiply two numbers")

func multiply(a: float, b: float) -> void:
    TinyConsole.info("%.2f * %.2f = %.2f" % [a, b, a * b])
    
# OR: TinyConsole.register_command(Callable(self, "multiply"), "multiply", "multiply two numbers")    
# OR: TinyConsole.register_command(func(a: float, b: float) -> void: TinyConsole.info("%.2f * %.2f = %.2f" % [a, b, a * b]), "multiply", "multiply two numbers")        
```

Type `multiply 2 4` in the console to see the result.

You can also specify a command as a subcommand:

```gdscript
TinyConsole.register_command(multiply, "math multiply", "multiply two numbers")
# Usage: math multiply 2 4
```

The parent command (`math`) doesn't need to exist.

## Autocompletion

Autocompletion works for command names and history out of the box. You can also provide custom autocomplete sources for specific arguments:

```gdscript
TinyConsole.register_command(teleport, "teleport", "teleport to a location")
TinyConsole.add_argument_autocomplete_source("teleport", 0, get_locations)

func get_locations() -> Array:
    return ["entrance", "caves", "boss"]
```

For dynamic values:

```gdscript
TinyConsole.add_argument_autocomplete_source("teleport", 0,
    func(): return get_tree().get_nodes_in_group("teleport_points").map(
        func(node): return node.name))
```
