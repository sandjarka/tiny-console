# Using with Other Languages

TinyConsole works with any language that supports GDExtension, since commands are registered via `Callable`. This page covers usage from Rust and C#.

## Rust (gdext)

### Registering commands

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

## C#

### Registering commands

Access the TinyConsole singleton via `Engine.GetSingleton()` and register commands using `Callable`:

```csharp
using Godot;

public partial class MyScene : Node
{
    private GodotObject _console;

    public override void _Ready()
    {
        _console = Engine.GetSingleton("TinyConsole");

        _console.Call("register_command",
            new Callable(this, MethodName.Greet),
            "greet", "greet someone");

        _console.Call("register_command",
            new Callable(this, MethodName.Multiply),
            "multiply", "multiply two numbers");
    }

    private void Greet(string name)
    {
        _console.Call("info", $"Hello, {name}!");
    }

    private void Multiply(double a, double b)
    {
        _console.Call("info", $"{a} * {b} = {a * b}");
    }
}
```

### Using autocomplete sources

```csharp
_console.Call("add_argument_autocomplete_source",
    "greet", 0, new Callable(this, MethodName.GetNameSuggestions));

private Godot.Collections.Array GetNameSuggestions()
{
    return new Godot.Collections.Array { "Alice", "Bob", "Charlie" };
}
```

### Console output methods

All output methods are available through `Call`:

```csharp
_console.Call("info", "informational message");
_console.Call("warn", "warning message");
_console.Call("error", "error message");
_console.Call("debug", "debug message");
```
