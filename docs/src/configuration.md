# Configuration

All settings are available in **Project > Project Settings** under the `addons/tiny_console` section.

## General

| Setting | Default | Description |
|---------|---------|-------------|
| `disable_in_release_build` | `false` | Disable the console in release builds |
| `enable_in_editor` | `false` | Allow the console to run in the editor |
| `print_to_stdout` | `false` | Mirror console output to stdout |
| `pause_when_open` | `true` | Pause the game while the console is open |
| `aliases` | `{exit: quit, source: exec, usage: help}` | Default command aliases |
| `commands_disabled_in_release` | `[eval]` | Commands disabled in release builds |

## Appearance

| Setting | Default | Description |
|---------|---------|-------------|
| `appearance/custom_theme` | `res://addons/tiny_console/res/default_theme.tres` | Path to theme resource |
| `appearance/height_ratio` | `0.5` | Console height as a ratio of the screen (0.1 - 1.0) |
| `appearance/open_speed` | `5.0` | Slide animation speed (0.1 - 20.0) |
| `appearance/opacity` | `1.0` | Console panel opacity (0.0 - 1.0) |
| `appearance/sparse_mode` | `false` | Add extra spacing between output lines |

## Greeting

| Setting | Default | Description |
|---------|---------|-------------|
| `greet/greet_user` | `true` | Show a greeting when the console first opens |
| `greet/greeting_message` | `Tiny Console` | Greeting text. Supports `{project_name}` and `{project_version}` placeholders |
| `greet/greet_using_ascii_art` | `true` | Render the greeting as ASCII art |

## History

| Setting | Default | Description |
|---------|---------|-------------|
| `history/persist_history` | `true` | Save command history to disk |
| `history/history_lines` | `1000` | Maximum number of history entries (10 - 10000) |

## Autocomplete

| Setting | Default | Description |
|---------|---------|-------------|
| `autocomplete/use_history_with_matches` | `true` | Include history entries in autocomplete suggestions |

## Autoexec

| Setting | Default | Description |
|---------|---------|-------------|
| `autoexec/script` | `user://autoexec.lcs` | Script to execute on startup |
| `autoexec/auto_create` | `true` | Create the autoexec file if it doesn't exist |

## Theming

Duplicate `addons/tiny_console/res/default_theme.tres` and point the `appearance/custom_theme` setting to your copy. Open the theme in Godot to customize fonts, colors, and styles. Console text colors are defined under the `ConsoleColors` theme type:

- `output_command_color` -- command echo color
- `output_command_mention_color` -- command name references
- `output_text_color` -- default text
- `output_error_color` -- error messages
- `output_warning_color` -- warning messages
- `output_debug_color` -- debug messages and tips
- `entry_text_color` -- input text
- `entry_hint_color` -- inline hint text
- `entry_command_found_color` -- recognized command
- `entry_subcommand_color` -- subcommand highlight
- `entry_command_not_found_color` -- unrecognized command
