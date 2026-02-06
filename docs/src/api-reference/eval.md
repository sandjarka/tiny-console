# Expression Evaluation

| Method | Description |
|--------|-------------|
| `add_eval_input(name, value)` | Add a variable to the eval context |
| `remove_eval_input(name)` | Remove a variable |
| `set_eval_base_instance(object)` | Set the base object for eval expressions |

The `eval` command evaluates GDScript expressions at runtime. You can inject variables and set a base instance for `self` references.

## Adding Variables

```gdscript
TinyConsole.add_eval_input("player", $Player)
TinyConsole.add_eval_input("score", 42)
```

Now in the console:

```
eval player.position
eval score * 2
```

## Setting the Base Instance

```gdscript
TinyConsole.set_eval_base_instance($Player)
```

This allows `eval` expressions to call methods on the base instance directly:

```
eval get_health()
eval position.x
```
