# Tiny Console

A fast, native in-game developer console for **Godot 4**, written in Rust via [gdext](https://github.com/godot-rust/gdext).

Originally a port of [LimboConsole](https://github.com/limbonaut/limbo_console) by limbonaut, rewritten from GDScript to Rust for better performance and delivered as a GDExtension.

> **Note:** This plugin is in active development. Expect breaking changes.

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

## Supported Platforms

| Platform | Architecture |
|----------|-------------|
| Windows | x86_64 |
| Linux | x86_64 |
| macOS | x86_64, ARM64 |

Requires Godot 4.1 or later.
