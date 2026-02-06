# Tiny Console

A fast, native in-game developer console for **Godot 4**, written in Rust via [gdext](https://github.com/godot-rust/gdext).

Originally a port of [LimboConsole](https://github.com/limbonaut/limbo_console) by limbonaut, rewritten from GDScript to Rust and delivered as a GDExtension.

> [!NOTE]
> This plugin is in active development. Expect changes.

## Features

- Command registration with automatic argument parsing (`bool`, `int`, `float`, `String`, `Vector2/3/4`)
- Subcommand support (`math multiply 2 4`)
- Tab autocompletion for commands, arguments, and history
- Inline hints and syntax highlighting
- Fuzzy history search (Ctrl+R)
- Command aliases
- Expression evaluation (`eval`)
- Script execution from `.lcs` files
- Autoexec script on startup
- Custom theming
- All settings accessible from **Project > Project Settings**
- Cross-platform: Windows, Linux, macOS (x86_64 + ARM64)

## Installation

1. Copy the `addons/tiny_console/` directory into your project's `addons/` folder.
2. Reload the project. The `TinyConsole` singleton is registered automatically.

Toggle the console with the **backtick** key (`` ` ``, the key to the left of `1`).

## Quick Start

```gdscript
func _ready() -> void:
    TinyConsole.register_command(multiply, "multiply", "multiply two numbers")

func multiply(a: float, b: float) -> void:
    TinyConsole.info("%.2f * %.2f = %.2f" % [a, b, a * b])
```

Type `multiply 2 4` in the console to see the result.

## Documentation

For the full documentation including API reference, configuration, theming, scripting, and more, see the [Tiny Console Book](https://sandjarka.github.io/tiny-console/).

## Credits

- Based on [LimboConsole](https://github.com/limbonaut/limbo_console) by [limbonaut](https://github.com/limbonaut)
- Built with [godot-rust/gdext](https://github.com/godot-rust/gdext)
- Font: [Monaspace Argon](https://monaspace.githubnext.com/) by GitHub Next
