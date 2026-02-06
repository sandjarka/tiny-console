# Aliases

| Method | Description |
|--------|-------------|
| `add_alias(alias, command)` | Create an alias |
| `remove_alias(name)` | Remove an alias |
| `get_aliases() -> PackedStringArray` | List all aliases |

## Example

```gdscript
# Create an alias
TinyConsole.add_alias("hp", "set_health 100")

# List all aliases
var all_aliases = TinyConsole.get_aliases()

# Remove an alias
TinyConsole.remove_alias("hp")
```

Default aliases are configured in [Project Settings](../configuration.md#general): `exit` -> `quit`, `source` -> `exec`, `usage` -> `help`.
