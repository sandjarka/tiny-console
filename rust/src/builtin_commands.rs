/// Registers all builtin commands on TinyConsole.
/// Commands are implemented as #[func] methods on TinyConsole.
/// This module registers them by name via Callable::from_object_method.
///
/// Unfortunately, Godot's Callable API requires string method names —
/// there is no way to pass Rust function pointers directly.
use godot::prelude::*;

use crate::tiny_console::TinyConsole;

/// Register a builtin command. The method name is stringified from the identifier,
/// giving a compile error if you typo a non-existent identifier (though it doesn't
/// verify the Godot-side registration — that's a gdext limitation).
macro_rules! cmd {
    ($console:expr, $this:expr, $method:ident, $name:literal, $desc:literal) => {
        // This ensures `TinyConsole::$method` exists at compile time.
        // The actual call goes through Godot's string-based Callable system.
        let _ = TinyConsole::$method;
        let callable = Callable::from_object_method($this, stringify!($method));
        $console.register_command(callable, GString::from($name), GString::from($desc));
    };
}

pub fn register(console: &mut TinyConsole, this: &Gd<TinyConsole>) {
    cmd!(console, this, cmd_alias, "alias", "add command alias");
    cmd!(console, this, cmd_aliases, "aliases", "list all aliases");
    cmd!(console, this, clear_console, "clear", "clear console");
    cmd!(console, this, cmd_commands, "commands", "list all commands");
    cmd!(console, this, info, "echo", "display a line of text");
    cmd!(console, this, cmd_eval, "eval", "evaluate an expression");
    cmd!(console, this, cmd_exec, "exec", "execute commands from file");
    cmd!(console, this, cmd_fps_max, "fps_max", "limit framerate");
    cmd!(console, this, cmd_fullscreen, "fullscreen", "toggle fullscreen mode");
    cmd!(console, this, cmd_help, "help", "show command info");
    cmd!(console, this, cmd_log, "log", "show recent log entries");
    cmd!(console, this, cmd_quit, "quit", "exit the application");
    cmd!(console, this, cmd_unalias, "unalias", "remove command alias");
    cmd!(console, this, cmd_vsync, "vsync", "adjust V-Sync");
    cmd!(console, this, erase_history, "erase_history", "erases current history and persisted history");

    // Note: help command autocomplete is handled inline in get_autocomplete_values()
    // to avoid re-entrant borrow panic (calling get_command_names on self while self is &mut borrowed).
}
