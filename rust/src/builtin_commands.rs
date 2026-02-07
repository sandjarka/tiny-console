/// BuiltinCommands: A RefCounted object that holds all builtin console command implementations.
///
/// This keeps builtin command logic out of TinyConsole's public API. Commands are registered
/// as Callables pointing to #[func] methods on this object, so Godot's method introspection
/// (argument names, types, defaults) works automatically for `help` display and type checking.
///
/// Each method accesses TinyConsole via the singleton — this is safe because command execution
/// uses the prepare/callv/finish pattern that releases the mutable borrow before calling callv().
use godot::classes::display_server::VSyncMode;
use godot::classes::{file_access::ModeFlags, Engine, Expression, FileAccess, ProjectSettings};
use godot::prelude::*;

use crate::tiny_console::TinyConsole;
use crate::util;

#[derive(GodotClass)]
#[class(base=RefCounted)]
pub struct BuiltinCommands {
    base: Base<RefCounted>,
}

#[godot_api]
impl IRefCounted for BuiltinCommands {
    fn init(base: Base<RefCounted>) -> Self {
        Self { base }
    }
}

#[godot_api]
impl BuiltinCommands {
    #[func]
    fn cmd_alias(&self, alias: GString, command: GString) {
        let mut console = TinyConsole::singleton();
        let mut s = console.bind_mut();
        let formatted = s.format_name(alias.clone());
        let msg = format!("Adding {} => {}", formatted, command);
        s.print_line_internal(&msg, false);
        s.add_alias(alias, command);
    }

    #[func]
    fn cmd_aliases(&self) {
        let mut console = TinyConsole::singleton();
        let mut s = console.bind_mut();
        let mut alias_names: Vec<String> = s.aliases.keys().cloned().collect();
        alias_names.sort();
        for alias in alias_names {
            let argv = s.aliases.get(&alias).unwrap().clone();
            let cmd_name = &argv[0];
            let desc = s.command_descriptions.get(cmd_name).cloned().unwrap_or_default();
            let color = s.output_command_mention_color.to_html();
            let formatted_alias = format!("[color={}]{}[/color]", color, alias);
            if desc.is_empty() {
                s.print_line_internal(&formatted_alias, false);
            } else {
                let formatted_cmd = format!("[color={}]{}[/color]", color, cmd_name);
                let rest = argv[1..].join(" ");
                let debug_color = s.output_debug_color.to_html();
                let tip = format!("[i][color={}] // {}[/color][/i]", debug_color, desc);
                let msg = format!("{} is alias of: {} {} {}", formatted_alias, formatted_cmd, rest, tip);
                s.print_line_internal(&msg, false);
            }
        }
    }

    #[func]
    fn cmd_commands(&self) {
        let mut console = TinyConsole::singleton();
        let mut s = console.bind_mut();
        s.print_line_internal("Available commands:", false);
        let mut names: Vec<String> = s.commands.keys().cloned().collect();
        names.sort();
        let color = s.output_command_mention_color.to_html();
        for name in &names {
            let desc = s.command_descriptions.get(name).cloned().unwrap_or_default();
            let formatted = format!("[color={}]{}[/color]", color, name);
            if desc.is_empty() {
                s.print_line_internal(&formatted, false);
            } else {
                let msg = format!("{} -- {}", formatted, desc);
                s.print_line_internal(&msg, false);
            }
        }
    }

    #[func]
    fn cmd_eval(&self, expression: GString) {
        let mut console = TinyConsole::singleton();
        let mut s = console.bind_mut();

        let mut exp = Expression::new_gd();
        let input_names = s.get_eval_input_names();
        let err = exp.parse_ex(&expression).input_names(&input_names).done();
        if err != godot::global::Error::OK {
            let err_text = exp.get_error_text();
            s.error(err_text);
            return;
        }
        let inputs = s.get_eval_inputs();
        let base = s.get_eval_base_instance();
        let base_obj: Option<Gd<godot::classes::Object>> = if base.is_nil() { None } else { base.try_to::<Gd<godot::classes::Object>>().ok() };

        let result = if let Some(ref base_obj) = base_obj {
            exp.execute_ex().inputs(&inputs).base_instance(base_obj).done()
        } else {
            exp.execute_ex().inputs(&inputs).done()
        };

        if !exp.has_execute_failed() {
            if !result.is_nil() {
                let msg = result.to_string();
                s.print_line_internal(&msg, false);
            }
        } else {
            let err_text = exp.get_error_text();
            s.error(err_text);
        }
    }

    #[func]
    fn cmd_exec(&self, file: GString) {
        let mut file_str = file.to_string();
        if !file_str.ends_with(".lcs") {
            file_str.push_str(".lcs");
        }
        let path: GString = GString::from(file_str.as_str());
        if !FileAccess::file_exists(&path) {
            file_str = format!("user://{}", file_str);
        }
        // execute_script is a deferred call, safe to invoke
        TinyConsole::singleton().bind().execute_script(GString::from(file_str.as_str()), true);
    }

    #[func]
    fn cmd_fps_max(&self, limit: i32) {
        let mut console = TinyConsole::singleton();
        let mut s = console.bind_mut();
        if limit < 0 {
            let current = Engine::singleton().get_max_fps();
            if current == 0 {
                s.print_line_internal("Framerate is unlimited.", false);
            } else {
                let msg = format!("Framerate is limited to {} FPS.", current);
                s.print_line_internal(&msg, false);
            }
            return;
        }
        Engine::singleton().set_max_fps(limit);
        if limit > 0 {
            let msg = format!("Limiting framerate to {} FPS.", limit);
            s.print_line_internal(&msg, false);
        } else {
            s.print_line_internal("Removing framerate limits.", false);
        }
    }

    #[func]
    fn cmd_fullscreen(&self) {
        let mut console = TinyConsole::singleton();
        let mut s = console.bind_mut();
        let tree = TinyConsole::get_scene_tree();
        if let Some(viewport) = tree.get_root() {
            if let Some(mut win) = viewport.get_window() {
                let mode = win.get_mode();
                if mode == godot::classes::window::Mode::WINDOWED {
                    win.set_mode(godot::classes::window::Mode::FULLSCREEN);
                    s.print_line_internal("Window switched to fullscreen mode.", false);
                } else {
                    win.set_mode(godot::classes::window::Mode::WINDOWED);
                    s.print_line_internal("Window switched to windowed mode.", false);
                }
            }
        }
    }

    #[func]
    fn cmd_help(&self, command_name: GString) {
        let mut console = TinyConsole::singleton();
        let mut s = console.bind_mut();
        if command_name.is_empty() {
            let color = s.output_command_mention_color.to_html();
            let debug_color = s.output_debug_color.to_html();
            let tip1 = format!("[i][color={}]Type [color={}]commands[/color] to list all available commands.[/color][/i]", debug_color, color);
            s.print_line_internal(&tip1, false);
            let tip2 = format!("[i][color={}]Type [color={}]help command[/color] to get more info about the command.[/color][/i]", debug_color, color);
            s.print_line_internal(&tip2, false);
        } else {
            s.usage(command_name);
        }
    }

    #[func]
    fn cmd_log(&self, num_lines: i32) {
        let mut console = TinyConsole::singleton();
        let mut s = console.bind_mut();
        let fn_path = ProjectSettings::singleton().get_setting("debug/file_logging/log_path").to::<GString>();
        if let Some(file) = FileAccess::open(&fn_path, ModeFlags::READ) {
            let contents = file.get_as_text().to_string();
            let mut lines: Vec<&str> = contents.split('\n').collect();
            if let Some(last) = lines.last() {
                if last.trim().is_empty() {
                    lines.pop();
                }
            }
            let start = lines.len().saturating_sub(num_lines.max(0) as usize);
            for line in &lines[start..] {
                let escaped = util::bbcode_escape(line);
                s.print_line_internal(&escaped, false);
            }
        } else {
            let msg = format!("Can't open file: {}", fn_path);
            s.error(GString::from(msg.as_str()));
        }
    }

    #[func]
    fn cmd_quit(&self) {
        let mut tree = TinyConsole::get_scene_tree();
        tree.quit();
    }

    #[func]
    fn cmd_unalias(&self, alias: GString) {
        let mut console = TinyConsole::singleton();
        let mut s = console.bind_mut();
        let name = alias.to_string();
        if s.aliases.contains_key(&name) {
            s.aliases.remove(&name);
            s.print_line_internal("Alias removed.", false);
        } else {
            let color = s.output_warning_color.to_html();
            let msg = format!("[color={}]WARNING:[/color] Alias not found.", color);
            s.print_line_internal(&msg, false);
        }
    }

    #[func]
    fn cmd_vsync(&self, mode: i32) {
        let mut console = TinyConsole::singleton();
        let mut s = console.bind_mut();
        if mode < 0 {
            let current = godot::classes::DisplayServer::singleton().window_get_vsync_mode();
            match current {
                VSyncMode::DISABLED => s.print_line_internal("V-Sync: disabled.", false),
                VSyncMode::ENABLED => s.print_line_internal("V-Sync: enabled.", false),
                VSyncMode::ADAPTIVE => s.print_line_internal("Current V-Sync mode: adaptive.", false),
                _ => {}
            }
            s.print_line_internal("Adjust V-Sync mode with an argument: 0 - disabled, 1 - enabled, 2 - adaptive.", false);
        } else {
            match mode {
                0 => {
                    s.print_line_internal("Changing to disabled.", false);
                    godot::classes::DisplayServer::singleton().window_set_vsync_mode(VSyncMode::DISABLED);
                }
                1 => {
                    s.print_line_internal("Changing to default V-Sync.", false);
                    godot::classes::DisplayServer::singleton().window_set_vsync_mode(VSyncMode::ENABLED);
                }
                2 => {
                    s.print_line_internal("Changing to adaptive V-Sync.", false);
                    godot::classes::DisplayServer::singleton().window_set_vsync_mode(VSyncMode::ADAPTIVE);
                }
                _ => {
                    s.error("Invalid mode.".into());
                    s.print_line_internal("Acceptable modes: 0 - disabled, 1 - enabled, 2 - adaptive.", false);
                }
            }
        }
    }
}

/// Register all builtin commands on TinyConsole.
pub fn register(console: &mut TinyConsole, builtin: &Gd<BuiltinCommands>) {
    let register = |console: &mut TinyConsole, method: &str, name: &str, desc: &str| {
        let callable = Callable::from_object_method(builtin, method);
        console.register_command(callable, GString::from(name), GString::from(desc));
    };

    register(console, "cmd_alias", "alias", "add command alias");
    register(console, "cmd_aliases", "aliases", "list all aliases");
    register(console, "cmd_commands", "commands", "list all commands");
    register(console, "cmd_eval", "eval", "evaluate an expression");
    register(console, "cmd_exec", "exec", "execute commands from file");
    register(console, "cmd_fps_max", "fps_max", "limit framerate");
    register(console, "cmd_fullscreen", "fullscreen", "toggle fullscreen mode");
    register(console, "cmd_help", "help", "show command info");
    register(console, "cmd_log", "log", "show recent log entries");
    register(console, "cmd_quit", "quit", "exit the application");
    register(console, "cmd_unalias", "unalias", "remove command alias");
    register(console, "cmd_vsync", "vsync", "adjust V-Sync");

    // These point to TinyConsole methods since they are part of the public API
    let console_gd = console.to_gd();
    console.register_command(Callable::from_object_method(&console_gd, "clear_console"), "clear".into(), "clear console".into());
    console.register_command(Callable::from_object_method(&console_gd, "info"), "echo".into(), "display a line of text".into());
    console.register_command(
        Callable::from_object_method(&console_gd, "erase_history"),
        "erase_history".into(),
        "erases current history and persisted history".into(),
    );

    // Note: help command autocomplete is handled inline in get_autocomplete_values()
    // to avoid re-entrant borrow panic (calling get_command_names on self while self is &mut borrowed).
}
