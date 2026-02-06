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
    TinyConsole.register_command(greet, "greet", "greet someone")
    
func greet(name: String) -> void:
    TinyConsole.info("Hello, %s!" % name)
    
# OR: TinyConsole.register_command(Callable(self, "greet"), "greet", "greet someone")    
# OR: TinyConsole.register_command(func() -> void: TinyConsole.info("Hello, %s!" % name), "greet", "greet someone")    
```

You can also pass a `Callable` explicitly:

```gdscript
TinyConsole.register_command(Callable(self, "greet"), "greet", "greet someone")
```

Subcommands are supported by using spaces in the name:

```gdscript
TinyConsole.register_command(multiply, "math multiply", "multiply two numbers")
# Usage: math multiply 2 4
```

## Registering commands from Rust (gdext)

Commands are registered via `Callable::from_object_method`, which requires a Godot object with `#[func]` methods. Create a class to hold your commands:

```rust
use godot::prelude::*;

#[derive(GodotClass)]
#[class(base=RefCounted)]
struct MyCommands {
    base: Base<RefCounted>,
}

#[godot_api]
impl IRefCounted for MyCommands {
    fn init(base: Base<RefCounted>) -> Self {
        Self { base }
    }
}

#[godot_api]
impl MyCommands {
    #[func]
    fn greet(&self, name: GString) {
        let mut console = TinyConsole::singleton();
        let msg = format!("Hello, {}!", name);
        console.bind_mut().info(GString::from(msg.as_str()));
    }

    #[func]
    fn multiply(&self, a: f64, b: f64) {
        let mut console = TinyConsole::singleton();
        let msg = format!("{} * {} = {}", a, b, a * b);
        console.bind_mut().info(GString::from(msg.as_str()));
    }
}
```

Then register the commands by creating an instance and pointing callables at it:

```rust
let commands = MyCommands::new_gd();

let mut console = TinyConsole::singleton();
let mut c = console.bind_mut();

c.register_command(
    Callable::from_object_method(&commands, "greet"),
    "greet".into(),
    "greet someone".into(),
);
c.register_command(
    Callable::from_object_method(&commands, "multiply"),
    "multiply".into(),
    "multiply two numbers".into(),
);
```

> **Why a separate object?** `Callable::from_object_method` requires `#[func]` methods. Putting commands on a dedicated `RefCounted` keeps them out of your main class's Godot API. See `builtin_commands.rs` in the addon source for a complete example.

> **Important:** Hold onto the `Gd<MyCommands>` for as long as the commands are registered. If the object is freed, the callables become invalid.

## add_argument_autocomplete_source

```gdscript
TinyConsole.add_argument_autocomplete_source(command: String, argument_index: int, callable: Callable)
```

Registers a callable that provides autocomplete suggestions for a specific argument (index 0-4). The callable should return an `Array` of strings.

```gdscript
TinyConsole.add_argument_autocomplete_source("teleport", 0, get_locations)

func get_locations() -> Array:
    return ["entrance", "caves", "boss"]
```
