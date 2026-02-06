# Scripting

Execute a sequence of commands from a file:

```
exec my_script
```

## Script Rules

- One command per line, same syntax as the console prompt.
- Files must have the `.lcs` extension (can be omitted in the `exec` command).
- Scripts are looked up in `user://` by default.
- Lines starting with `#` are comments.

## Autoexec

The autoexec script (`user://autoexec.lcs` by default) runs automatically on every game start.

You can configure the autoexec path and whether it's auto-created in [Project Settings](./configuration.md#autoexec).
