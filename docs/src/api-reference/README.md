# API Reference

The `TinyConsole` singleton is available globally in GDScript. All methods below are called on `TinyConsole`.

```gdscript
TinyConsole.register_command(Callable(self, "my_func"), "my_command", "description")
TinyConsole.info("Hello from the console!")
```

The API is organized into the following sections:

- [Command Registration](./command-registration.md) -- registering commands and autocomplete sources
- [Output](./output.md) -- printing messages to the console
- [Console Control](./console-control.md) -- opening, closing, and clearing the console
- [Command Execution](./command-execution.md) -- executing commands and scripts programmatically
- [Aliases](./aliases.md) -- managing command aliases
- [Expression Evaluation](./eval.md) -- configuring the eval system
- [Signals](./signals.md) -- reacting to console events
