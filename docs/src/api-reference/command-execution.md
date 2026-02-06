# Command Execution

| Method | Description |
|--------|-------------|
| `execute_command(command_line)` | Execute a command string |
| `execute_command_silent(command_line)` | Execute without printing the input |
| `execute_script(path, silent)` | Execute commands from a file |

## execute_command

```gdscript
TinyConsole.execute_command("fps_max 60")
```

Executes a command as if the user typed it into the console. The command and its output are printed.

## execute_command_silent

```gdscript
TinyConsole.execute_command_silent("alias tp teleport")
```

Same as `execute_command`, but the command input itself is not echoed to the console.

## execute_script

```gdscript
TinyConsole.execute_script("user://setup.lcs", false)
```

Loads a `.lcs` script file and executes each line as a command. See [Scripting](../scripting.md) for the file format.
