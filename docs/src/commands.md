# Built-in Commands

| Command | Description |
|---------|-------------|
| `help [command]` | Show help or command usage |
| `commands` | List all available commands |
| `clear` | Clear console output |
| `echo <text>` | Print text |
| `alias <name> <command>` | Create a command alias |
| `aliases` | List all aliases |
| `unalias <name>` | Remove an alias |
| `eval <expression>` | Evaluate a GDScript expression |
| `exec <file>` | Execute a script file |
| `log [lines]` | Show recent engine log entries |
| `fps_max <limit>` | Set framerate limit (0 = unlimited) |
| `fullscreen` | Toggle fullscreen mode |
| `vsync <mode>` | Set V-Sync (0=off, 1=on, 2=adaptive) |
| `quit` | Quit the application |
| `erase_history` | Clear command history |

Default aliases: `exit` -> `quit`, `source` -> `exec`, `usage` -> `help`

## Keyboard Shortcuts

| Key | Action |
|-----|--------|
| `` ` `` (backtick) | Toggle console |
| `Enter` | Execute command |
| `Tab` | Autocomplete / cycle suggestions |
| `Shift+Tab` | Cycle suggestions in reverse |
| `Right` (at end of input) | Accept inline hint |
| `Up` / `Down` | Navigate command history |
| `Ctrl+R` | Toggle fuzzy history search |
| `Ctrl+C` (no selection) | Clear input |
