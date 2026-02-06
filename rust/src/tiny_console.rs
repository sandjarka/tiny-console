/// TinyConsole: Main console engine singleton (extends Object).
/// Port of LimboConsole from GDScript to Rust via gdext.
/// Registered as an engine singleton automatically by gdext.
/// Creates an internal CanvasLayer and adds it to the scene tree.
use godot::classes::control::{FocusMode, LayoutPreset, SizeFlags};
use godot::classes::{
    file_access::ModeFlags, CanvasLayer, Control, Engine, FileAccess, IObject, InputEvent, InputEventKey, InputMap, Os, PanelContainer, ProjectSettings, ResourceLoader, RichTextLabel, SceneTree,
    Theme, VBoxContainer,
};
use godot::global::{ease, remap, Error as GError, Key};
use godot::prelude::*;

use std::collections::HashMap;

use crate::ascii_art;
use crate::builtin_commands;
use crate::command_entry::CommandEntry;
use crate::command_entry_highlighter::CommandEntryHighlighter;
use crate::command_history::{self, CommandHistory, WrappingIterator};
use crate::console_options::ConsoleOptions;
use crate::history_gui::HistoryGui;
use crate::util;

const THEME_DEFAULT: &str = "res://addons/tiny_console/res/default_theme.tres";
const MAX_SUBCOMMANDS: usize = 4;
const CONSOLE_COLORS_THEME_TYPE: &str = "ConsoleColors";

#[derive(GodotClass)]
#[class(base=Object, singleton)]
pub struct TinyConsole {
    base: Base<Object>,

    // The CanvasLayer node that holds all GUI (added to scene tree root)
    canvas_layer: Option<Gd<CanvasLayer>>,

    // GUI nodes
    control: Option<Gd<PanelContainer>>,
    control_block: Option<Gd<Control>>,
    output: Option<Gd<RichTextLabel>>,
    entry: Option<Gd<CommandEntry>>,
    history_gui: Option<Gd<HistoryGui>>,
    previous_gui_focus: Option<Gd<Control>>,

    // Theme colors
    output_command_color: Color,
    output_command_mention_color: Color,
    output_error_color: Color,
    output_warning_color: Color,
    output_text_color: Color,
    output_debug_color: Color,
    entry_text_color: Color,
    entry_hint_color: Color,
    entry_command_found_color: Color,
    entry_subcommand_color: Color,
    entry_command_not_found_color: Color,

    // State
    enabled: bool,
    initialized: bool,
    options: ConsoleOptions,
    commands: HashMap<String, Callable>,
    aliases: HashMap<String, Vec<String>>,
    command_descriptions: HashMap<String, String>,
    argument_autocomplete_sources: HashMap<(String, usize), Callable>,
    history: CommandHistory,
    history_iter: WrappingIterator,
    autocomplete_matches: Vec<String>,
    eval_inputs: HashMap<String, Variant>,
    silent: bool,
    was_already_paused: bool,
    open_t: f32,
    open_speed: f32,
    is_open: bool,
    // Pending command from signal callback — executed in on_process_frame
    // to avoid re-entrant borrow issues with #[func] dispatch.
    pending_command: Option<String>,
}

// === Public API (exposed to GDScript via #[func]) ===

#[godot_api]
impl TinyConsole {
    #[signal]
    fn toggled(is_shown: bool);

    // --- Initialization ---

    pub fn is_initialized(&self) -> bool {
        self.initialized
    }

    /// Initialize the console. Must be called with a Gd<TinyConsole> (not from &self or &mut self)
    /// because it needs to release/re-acquire the borrow around user callable invocations.
    pub fn initialize_impl(this: &mut Gd<TinyConsole>) {
        // Clone the Gd handle before borrowing — needed for Callable creation
        let gd_ref: Gd<TinyConsole> = this.clone();

        // Phase 1: Setup (needs &mut self)
        {
            let mut s = this.bind_mut();
            if s.initialized {
                return;
            }
            s.initialized = true;

            s.open_speed = s.options.open_speed;

            if s.options.disable_in_release_build {
                s.enabled = Os::singleton().is_debug_build();
            }

            if Engine::singleton().is_editor_hint() && !s.options.enable_in_editor {
                s.enabled = false;
            }

            // Register input actions
            s.register_input_actions();

            // Build GUI
            s.build_gui();
            s.init_theme();

            // Add canvas layer to scene tree
            if let Some(ref mut cl) = s.canvas_layer {
                cl.set_layer(9999);
                cl.set_process_mode(godot::classes::node::ProcessMode::ALWAYS);
                cl.set_process(false);

                let tree = Self::get_scene_tree();
                if let Some(mut root) = tree.get_root() {
                    root.add_child(&*cl);
                }
            }

            // Hide console initially
            if let Some(ref mut control) = s.control {
                control.set_visible(false);
            }
            if let Some(ref mut block) = s.control_block {
                block.set_visible(false);
            }

            // Connect signals
            if let Some(ref entry) = s.entry {
                let mut entry_node: Gd<godot::classes::Node> = entry.clone().upcast();
                entry_node.connect("text_submitted", &Callable::from_object_method(&gd_ref, "on_entry_text_submitted"));
                entry_node.connect("text_changed", &Callable::from_object_method(&gd_ref, "on_entry_text_changed"));
                entry_node.connect("autocomplete_requested", &Callable::from_object_method(&gd_ref, "on_autocomplete_requested"));
                entry_node.connect("reverse_autocomplete_requested", &Callable::from_object_method(&gd_ref, "on_reverse_autocomplete_requested"));
                entry_node.connect("history_up_requested", &Callable::from_object_method(&gd_ref, "on_history_up_requested"));
                entry_node.connect("history_down_requested", &Callable::from_object_method(&gd_ref, "on_history_down_requested"));
                entry_node.connect("scroll_up_requested", &Callable::from_object_method(&gd_ref, "on_scroll_up_requested"));
                entry_node.connect("scroll_down_requested", &Callable::from_object_method(&gd_ref, "on_scroll_down_requested"));
            }

            // Connect canvas_layer process and input
            if s.canvas_layer.is_some() {
                let mut tree = Self::get_scene_tree();
                tree.connect("process_frame", &Callable::from_object_method(&gd_ref, "on_process_frame"));
            }

            // Register builtin commands
            builtin_commands::register(&mut s, &gd_ref);

            // Greet
            if s.options.greet_user {
                s.greet();
            }

            // Add aliases (no user callables involved)
            s.add_aliases_from_config();
        }
        // bind_mut dropped here

        // Phase 2: Autoexec script (may call user commands that call back into TinyConsole)
        let autoexec = this.bind().options.autoexec_script.clone();
        if !autoexec.is_empty() {
            let path = GString::from(autoexec.as_str());
            let auto_create = this.bind().options.autoexec_auto_create;
            if auto_create && !FileAccess::file_exists(&path) {
                let _ = FileAccess::open(&path, ModeFlags::WRITE);
            }
            if FileAccess::file_exists(&path) {
                if let Some(fa) = FileAccess::open(&path, ModeFlags::READ) {
                    let mut lines = Vec::new();
                    while !fa.eof_reached() {
                        lines.push(fa.get_line().to_string());
                    }
                    for line in lines {
                        Self::execute_command_on(this, &line, true);
                    }
                }
            }
        }
    }

    // --- Console visibility ---

    #[func]
    pub fn open_console(&mut self) {
        if self.enabled {
            self.is_open = true;
            if let Some(ref mut cl) = self.canvas_layer {
                cl.set_process(true);
            }
            self.show_console();
        }
    }

    #[func]
    pub fn close_console(&mut self) {
        if self.enabled {
            self.is_open = false;
            if let Some(ref mut cl) = self.canvas_layer {
                cl.set_process(true);
            }
            if let Some(ref mut hg) = self.history_gui {
                hg.set_visible(false);
            }
            if self.options.persist_history {
                self.history.save(command_history::HISTORY_FILE);
            }
        }
    }

    #[func]
    pub fn is_console_open(&self) -> bool {
        self.is_open
    }

    #[func]
    pub fn toggle_console(&mut self) {
        if self.is_open {
            self.close_console();
        } else {
            self.open_console();
        }
    }

    #[func]
    pub fn toggle_history(&mut self) {
        let was_visible = self.history_gui.as_ref().map_or(false, |hg| hg.is_visible());

        if let Some(ref mut hg) = self.history_gui {
            hg.set_visible(!was_visible);
        }

        if !was_visible {
            let entry_text = self.get_entry_text();
            let results = self.history.fuzzy_match(&entry_text);
            let packed: PackedStringArray = results.iter().map(|s| GString::from(s.as_str())).collect();
            if let Some(ref mut hg) = self.history_gui {
                hg.bind_mut().set_search_results(packed);
            }
        }
    }

    #[func]
    pub fn clear_console(&mut self) {
        if let Some(ref mut output) = self.output {
            output.set_text("");
        }
    }

    #[func]
    pub fn erase_history(&mut self) {
        self.history.clear();
        let path: GString = command_history::HISTORY_FILE.into();
        if let Some(mut file) = FileAccess::open(&path, ModeFlags::WRITE) {
            file.store_string("");
        }
    }

    // --- Output methods ---

    #[func]
    pub fn info(&mut self, line: GString) {
        let stdout = self.options.print_to_stdout;
        self.print_line_internal(&line.to_string(), stdout);
    }

    #[func]
    pub fn error(&mut self, line: GString) {
        let color = self.output_error_color.to_html();
        let msg = format!("[color={}]ERROR:[/color] {}", color, line);
        let stdout = self.options.print_to_stdout;
        self.print_line_internal(&msg, stdout);
    }

    #[func]
    pub fn warn(&mut self, line: GString) {
        let color = self.output_warning_color.to_html();
        let msg = format!("[color={}]WARNING:[/color] {}", color, line);
        let stdout = self.options.print_to_stdout;
        self.print_line_internal(&msg, stdout);
    }

    #[func]
    pub fn debug_msg(&mut self, line: GString) {
        let color = self.output_debug_color.to_html();
        let msg = format!("[color={}]DEBUG: {}[/color]", color, line);
        let stdout = self.options.print_to_stdout;
        self.print_line_internal(&msg, stdout);
    }

    #[func]
    pub fn print_boxed(&mut self, line: GString) {
        let lines = ascii_art::str_to_boxed_art(&line.to_string());
        let stdout = self.options.print_to_stdout;
        for l in lines {
            self.print_line_internal(&l, stdout);
        }
    }

    #[func]
    pub fn print_line(&mut self, line: GString) {
        let stdout = self.options.print_to_stdout;
        self.print_line_internal(&line.to_string(), stdout);
    }

    #[func]
    pub fn print_line_ex(&mut self, line: GString, stdout: bool) {
        self.print_line_internal(&line.to_string(), stdout);
    }

    // --- Command registration ---

    #[func]
    pub fn register_command(&mut self, callable: Callable, name: GString, desc: GString) {
        let name_str = name.to_string();

        if !name_str.is_empty() && !util::is_valid_command_sequence(&name_str) {
            godot_error!("TinyConsole: Failed to register command: {}. Name must use valid identifiers.", name_str);
            return;
        }

        let cmd_name = if name_str.is_empty() {
            let method = callable.method_name().map(|n| n.to_string()).unwrap_or_default();
            if method.is_empty() {
                godot_error!("TinyConsole: Failed to register command: no method name and no name provided");
                return;
            }
            method.trim_start_matches('_').trim_start_matches("cmd_").to_string()
        } else {
            name_str
        };

        if !Os::singleton().is_debug_build() && self.options.commands_disabled_in_release.contains(&cmd_name) {
            return;
        }

        if self.commands.contains_key(&cmd_name) {
            godot_error!("TinyConsole: Command already registered: {}", cmd_name);
            return;
        }

        self.commands.insert(cmd_name.clone(), callable);
        self.command_descriptions.insert(cmd_name, desc.to_string());
    }

    #[func]
    pub fn unregister_command(&mut self, name: GString) {
        let name_str = name.to_string();
        if !self.commands.contains_key(&name_str) {
            godot_error!("TinyConsole: Unregister failed - command not found: {}", name_str);
            return;
        }
        self.commands.remove(&name_str);
        self.command_descriptions.remove(&name_str);
        for i in 0..5 {
            self.argument_autocomplete_sources.remove(&(name_str.clone(), i));
        }
    }

    #[func]
    pub fn has_command(&self, name: GString) -> bool {
        self.commands.contains_key(&name.to_string())
    }

    #[func]
    pub fn has_alias(&self, name: GString) -> bool {
        self.aliases.contains_key(&name.to_string())
    }

    #[func]
    pub fn get_command_names(&self, include_aliases: bool) -> PackedStringArray {
        let mut names: Vec<String> = self.commands.keys().cloned().collect();
        if include_aliases {
            names.extend(self.aliases.keys().cloned());
        }
        names.sort();
        names.iter().map(|s| GString::from(s.as_str())).collect()
    }

    #[func]
    pub fn get_command_description(&self, name: GString) -> GString {
        GString::from(self.command_descriptions.get(&name.to_string()).map(|s| s.as_str()).unwrap_or(""))
    }

    // --- Aliases ---

    #[func]
    pub fn add_alias(&mut self, alias: GString, command_to_run: GString) {
        let argv = self.parse_command_line(&command_to_run.to_string());
        self.aliases.insert(alias.to_string(), argv);
    }

    #[func]
    pub fn remove_alias(&mut self, name: GString) {
        self.aliases.remove(&name.to_string());
    }

    #[func]
    pub fn get_aliases(&self) -> PackedStringArray {
        self.aliases.keys().map(|s| GString::from(s.as_str())).collect()
    }

    #[func]
    pub fn get_alias_argv(&self, alias: GString) -> PackedStringArray {
        let alias_str = alias.to_string();
        match self.aliases.get(&alias_str) {
            Some(argv) => argv.iter().map(|s| GString::from(s.as_str())).collect(),
            None => {
                let mut arr = PackedStringArray::new();
                arr.push(&alias);
                arr
            }
        }
    }

    // --- Autocomplete sources ---

    #[func]
    pub fn add_argument_autocomplete_source(&mut self, command: GString, argument: i32, source: Callable) {
        let cmd = command.to_string();
        if !source.is_valid() {
            godot_error!("TinyConsole: Can't add autocomplete source: callable is not valid");
            return;
        }
        if !self.commands.contains_key(&cmd) {
            godot_error!("TinyConsole: Can't add autocomplete source: command doesn't exist: {}", cmd);
            return;
        }
        if argument < 0 || argument > 4 {
            godot_error!("TinyConsole: Can't add autocomplete source: argument index out of bounds");
            return;
        }
        self.argument_autocomplete_sources.insert((cmd, argument as usize), source);
    }

    // --- Command execution ---
    // Note: Command execution uses prepare/callv/finish pattern to avoid
    // re-entrant borrow panics. The user's callable may call back into
    // TinyConsole (e.g. TinyConsole.info()), so we must release the
    // mutable borrow before invoking callv().

    #[func]
    pub fn execute_command(&self, command_line: GString) {
        let mut gd = self.to_gd();
        let cmd = command_line.to_string();
        Callable::from_fn("_exec_cmd", move |_args| {
            Self::execute_command_on(&mut gd, &cmd, false);
            Variant::nil()
        })
        .call_deferred(&[]);
    }

    #[func]
    pub fn execute_command_silent(&self, command_line: GString) {
        let mut gd = self.to_gd();
        let cmd = command_line.to_string();
        Callable::from_fn("_exec_cmd_silent", move |_args| {
            Self::execute_command_on(&mut gd, &cmd, true);
            Variant::nil()
        })
        .call_deferred(&[]);
    }

    #[func]
    pub fn execute_script(&self, file: GString, silent: bool) {
        let mut gd = self.to_gd();
        let file_str = file.to_string();
        Callable::from_fn("_exec_script", move |_args| {
            let path: GString = GString::from(file_str.as_str());
            if FileAccess::file_exists(&path) {
                if !silent {
                    let msg = format!("Executing {}", file_str);
                    gd.bind_mut().print_line_internal(&msg, false);
                }
                if let Some(fa) = FileAccess::open(&path, ModeFlags::READ) {
                    let mut lines = Vec::new();
                    while !fa.eof_reached() {
                        lines.push(fa.get_line().to_string());
                    }
                    for line in lines {
                        Self::execute_command_on(&mut gd, &line, silent);
                    }
                }
            } else {
                let trimmed = file_str.trim_start_matches("user://");
                let msg = format!("File not found: {}", trimmed);
                gd.bind_mut().error(GString::from(msg.as_str()));
            }
            Variant::nil()
        })
        .call_deferred(&[]);
    }

    // --- Formatting ---

    #[func]
    pub fn format_tip(&self, text: GString) -> GString {
        let color = self.output_debug_color.to_html();
        GString::from(format!("[i][color={}]{}[/color][/i]", color, text).as_str())
    }

    #[func]
    pub fn format_name(&self, name: GString) -> GString {
        let color = self.output_command_mention_color.to_html();
        GString::from(format!("[color={}]{}[/color]", color, name).as_str())
    }

    #[func]
    pub fn usage(&mut self, command: GString) -> i32 {
        let cmd_str = command.to_string();

        // If it's an alias, show what it resolves to
        if self.aliases.contains_key(&cmd_str) {
            let alias_argv = self.aliases.get(&cmd_str).unwrap().clone();
            let formatted_cmd_name = format!("[color={}]{}[/color]", self.output_command_mention_color.to_html(), alias_argv[0]);
            let rest = alias_argv[1..].join(" ");
            let msg = format!("Alias of: {} {}", formatted_cmd_name, rest);
            self.print_line_internal(&msg, false);
        }

        let actual_cmd = if let Some(argv) = self.aliases.get(&cmd_str) { argv[0].clone() } else { cmd_str.clone() };

        if !self.commands.contains_key(&actual_cmd) {
            let msg = format!("Command not found: {}", actual_cmd);
            self.error(GString::from(msg.as_str()));
            return 1;
        }

        let callable = self.commands.get(&actual_cmd).unwrap().clone();
        let method_info = self.get_method_info(&callable);

        let usage_line;
        let mut arg_lines = String::new();
        let mut values_lines = String::new();

        if let Some(ref info) = method_info {
            let required_args = info.args.len().saturating_sub(info.default_count);
            let bound_args = callable.get_bound_arguments_count() as usize;
            let displayable_args = info.args.len().saturating_sub(bound_args);

            let mut usage_str = format!("Usage: {}", actual_cmd);
            for i in 0..displayable_args {
                let arg_name = &info.args[i].name;
                if i < required_args {
                    usage_str.push_str(&format!(" {}", arg_name));
                } else {
                    usage_str.push_str(&format!(" [lb]{}[rb]", arg_name));
                }

                let type_name = variant_type_name(info.args[i].type_id);
                let mut def_spec = String::new();
                if i >= required_args {
                    let def_idx = i - required_args;
                    if def_idx < info.defaults.len() {
                        def_spec = format!(" = {}", info.defaults[def_idx]);
                    }
                }
                arg_lines.push_str(&format!("  {}: {}{}\n", arg_name, type_name, def_spec));

                let key = (actual_cmd.clone(), i);
                if let Some(source) = self.argument_autocomplete_sources.get(&key) {
                    let result = source.callv(&VarArray::new());
                    if let Some(values) = variant_to_string_vec(&result) {
                        if !values.is_empty() {
                            values_lines.push_str(&format!(" {}: {}\n", arg_name, values.join(", ")));
                        }
                    }
                }
            }
            usage_line = usage_str;
        } else {
            usage_line = format!("Usage: {} ???", actual_cmd);
        }

        self.print_line_internal(&usage_line, false);

        if let Some(desc) = self.command_descriptions.get(&actual_cmd) {
            if !desc.is_empty() {
                let mut desc_display = desc.clone();
                if let Some(first_char) = desc_display.chars().next() {
                    let upper: String = first_char.to_uppercase().collect();
                    desc_display = format!("{}{}", upper, &desc_display[first_char.len_utf8()..]);
                }
                if !desc_display.ends_with('.') {
                    desc_display.push('.');
                }
                self.print_line_internal(&desc_display, false);
            }
        }

        let arg_lines_trimmed = arg_lines.trim_end_matches('\n');
        if !arg_lines_trimmed.is_empty() {
            self.print_line_internal("Arguments:", false);
            self.print_line_internal(arg_lines_trimmed, false);
        }
        let values_lines_trimmed = values_lines.trim_end_matches('\n');
        if !values_lines_trimmed.is_empty() {
            self.print_line_internal("Values:", false);
            self.print_line_internal(values_lines_trimmed, false);
        }

        0
    }

    // --- Eval ---

    #[func]
    pub fn add_eval_input(&mut self, name: GString, value: Variant) {
        self.eval_inputs.insert(name.to_string(), value);
    }

    #[func]
    pub fn remove_eval_input(&mut self, name: GString) {
        self.eval_inputs.remove(&name.to_string());
    }

    #[func]
    pub fn get_eval_input_names(&self) -> PackedStringArray {
        self.eval_inputs.keys().filter(|k| *k != "_base_instance").map(|s| GString::from(s.as_str())).collect()
    }

    #[func]
    pub fn get_eval_inputs(&self) -> VarArray {
        let mut arr = VarArray::new();
        for (k, v) in &self.eval_inputs {
            if k != "_base_instance" {
                arr.push(v);
            }
        }
        arr
    }

    #[func]
    pub fn set_eval_base_instance(&mut self, object: Variant) {
        self.eval_inputs.insert("_base_instance".to_string(), object);
    }

    #[func]
    pub fn get_eval_base_instance(&self) -> Variant {
        self.eval_inputs.get("_base_instance").cloned().unwrap_or(Variant::nil())
    }

    // --- Builtin command implementations ---

    #[func]
    pub fn cmd_alias(&mut self, alias: GString, command: GString) {
        let formatted = self.format_name(alias.clone());
        let msg = format!("Adding {} => {}", formatted, command);
        self.print_line_internal(&msg, false);
        self.add_alias(alias, command);
    }

    #[func]
    pub fn cmd_aliases(&mut self) {
        let mut alias_names: Vec<String> = self.aliases.keys().cloned().collect();
        alias_names.sort();
        for alias in alias_names {
            let argv = self.aliases.get(&alias).unwrap().clone();
            let cmd_name = &argv[0];
            let desc = self.command_descriptions.get(cmd_name).cloned().unwrap_or_default();
            let color = self.output_command_mention_color.to_html();
            let formatted_alias = format!("[color={}]{}[/color]", color, alias);
            if desc.is_empty() {
                self.print_line_internal(&formatted_alias, false);
            } else {
                let formatted_cmd = format!("[color={}]{}[/color]", color, cmd_name);
                let rest = argv[1..].join(" ");
                let debug_color = self.output_debug_color.to_html();
                let tip = format!("[i][color={}] // {}[/color][/i]", debug_color, desc);
                let msg = format!("{} is alias of: {} {} {}", formatted_alias, formatted_cmd, rest, tip);
                self.print_line_internal(&msg, false);
            }
        }
    }

    #[func]
    pub fn cmd_commands(&mut self) {
        self.print_line_internal("Available commands:", false);
        let mut names: Vec<String> = self.commands.keys().cloned().collect();
        names.sort();
        let color = self.output_command_mention_color.to_html();
        for name in &names {
            let desc = self.command_descriptions.get(name).cloned().unwrap_or_default();
            let formatted = format!("[color={}]{}[/color]", color, name);
            if desc.is_empty() {
                self.print_line_internal(&formatted, false);
            } else {
                let msg = format!("{} -- {}", formatted, desc);
                self.print_line_internal(&msg, false);
            }
        }
    }

    #[func]
    pub fn cmd_eval(&mut self, expression: GString) {
        let mut exp = godot::classes::Expression::new_gd();
        let input_names = self.get_eval_input_names();
        let err = exp.parse_ex(&expression).input_names(&input_names).done();
        if err != GError::OK {
            let err_text = exp.get_error_text();
            self.error(err_text);
            return;
        }
        let inputs = self.get_eval_inputs();
        let base = self.get_eval_base_instance();
        let base_obj: Option<Gd<godot::classes::Object>> = if base.is_nil() { None } else { base.try_to::<Gd<godot::classes::Object>>().ok() };

        let result = if let Some(ref base_obj) = base_obj {
            exp.execute_ex().inputs(&inputs).base_instance(base_obj).done()
        } else {
            exp.execute_ex().inputs(&inputs).done()
        };

        if !exp.has_execute_failed() {
            if !result.is_nil() {
                let msg = result.to_string();
                self.print_line_internal(&msg, false);
            }
        } else {
            let err_text = exp.get_error_text();
            self.error(err_text);
        }
    }

    #[func]
    pub fn cmd_exec(&mut self, file: GString) {
        let mut file_str = file.to_string();
        if !file_str.ends_with(".lcs") {
            file_str.push_str(".lcs");
        }
        let path: GString = GString::from(file_str.as_str());
        if !FileAccess::file_exists(&path) {
            file_str = format!("user://{}", file_str);
        }
        self.execute_script(GString::from(file_str.as_str()), true);
    }

    #[func]
    pub fn cmd_fps_max(&mut self, limit: i32) {
        if limit < 0 {
            let current = Engine::singleton().get_max_fps();
            if current == 0 {
                self.print_line_internal("Framerate is unlimited.", false);
            } else {
                let msg = format!("Framerate is limited to {} FPS.", current);
                self.print_line_internal(&msg, false);
            }
            return;
        }
        Engine::singleton().set_max_fps(limit);
        if limit > 0 {
            let msg = format!("Limiting framerate to {} FPS.", limit);
            self.print_line_internal(&msg, false);
        } else {
            self.print_line_internal("Removing framerate limits.", false);
        }
    }

    #[func]
    pub fn cmd_fullscreen(&mut self) {
        let tree = Self::get_scene_tree();
        if let Some(viewport) = tree.get_root() {
            if let Some(mut win) = viewport.get_window() {
                let mode = win.get_mode();
                if mode == godot::classes::window::Mode::WINDOWED {
                    win.set_mode(godot::classes::window::Mode::FULLSCREEN);
                    self.print_line_internal("Window switched to fullscreen mode.", false);
                } else {
                    win.set_mode(godot::classes::window::Mode::WINDOWED);
                    self.print_line_internal("Window switched to windowed mode.", false);
                }
            }
        }
    }

    #[func]
    pub fn cmd_help(&mut self, command_name: GString) {
        if command_name.is_empty() {
            let color = self.output_command_mention_color.to_html();
            let debug_color = self.output_debug_color.to_html();
            let tip1 = format!("[i][color={}]Type [color={}]commands[/color] to list all available commands.[/color][/i]", debug_color, color);
            self.print_line_internal(&tip1, false);
            let tip2 = format!("[i][color={}]Type [color={}]help command[/color] to get more info about the command.[/color][/i]", debug_color, color);
            self.print_line_internal(&tip2, false);
        } else {
            self.usage(command_name);
        }
    }

    #[func]
    pub fn cmd_log(&mut self, num_lines: i32) {
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
                self.print_line_internal(&escaped, false);
            }
        } else {
            let msg = format!("Can't open file: {}", fn_path);
            self.error(GString::from(msg.as_str()));
        }
    }

    #[func]
    pub fn cmd_quit(&self) {
        let mut tree = Self::get_scene_tree();
        tree.quit();
    }

    #[func]
    pub fn cmd_unalias(&mut self, alias: GString) {
        let name = alias.to_string();
        if self.aliases.contains_key(&name) {
            self.aliases.remove(&name);
            self.print_line_internal("Alias removed.", false);
        } else {
            let color = self.output_warning_color.to_html();
            let msg = format!("[color={}]WARNING:[/color] Alias not found.", color);
            self.print_line_internal(&msg, false);
        }
    }

    #[func]
    pub fn cmd_vsync(&mut self, mode: i32) {
        use godot::classes::display_server::VSyncMode;
        if mode < 0 {
            let current = godot::classes::DisplayServer::singleton().window_get_vsync_mode();
            match current {
                VSyncMode::DISABLED => self.print_line_internal("V-Sync: disabled.", false),
                VSyncMode::ENABLED => self.print_line_internal("V-Sync: enabled.", false),
                VSyncMode::ADAPTIVE => self.print_line_internal("Current V-Sync mode: adaptive.", false),
                _ => {}
            }
            self.print_line_internal("Adjust V-Sync mode with an argument: 0 - disabled, 1 - enabled, 2 - adaptive.", false);
        } else {
            match mode {
                0 => {
                    self.print_line_internal("Changing to disabled.", false);
                    godot::classes::DisplayServer::singleton().window_set_vsync_mode(VSyncMode::DISABLED);
                }
                1 => {
                    self.print_line_internal("Changing to default V-Sync.", false);
                    godot::classes::DisplayServer::singleton().window_set_vsync_mode(VSyncMode::ENABLED);
                }
                2 => {
                    self.print_line_internal("Changing to adaptive V-Sync.", false);
                    godot::classes::DisplayServer::singleton().window_set_vsync_mode(VSyncMode::ADAPTIVE);
                }
                _ => {
                    self.error("Invalid mode.".into());
                    self.print_line_internal("Acceptable modes: 0 - disabled, 1 - enabled, 2 - adaptive.", false);
                }
            }
        }
    }

    // --- Signal callbacks ---

    #[func]
    fn on_entry_text_submitted(&mut self, command: GString) {
        let hg_visible = self.history_gui.as_ref().map_or(false, |hg| hg.is_visible());

        if hg_visible {
            let current_text = self.history_gui.as_ref().map(|hg| hg.bind().get_current_text().to_string()).unwrap_or_default();
            if let Some(ref mut hg) = self.history_gui {
                hg.set_visible(false);
            }
            self.clear_autocomplete();
            self.fill_entry(&current_text);
            self.update_autocomplete();
            return;
        }
        self.clear_autocomplete();
        self.fill_entry("");
        // Store command for deferred execution in on_process_frame.
        // We can't call the user's callable here because this #[func]
        // holds bind_mut(), and the callable may call back into TinyConsole.
        self.pending_command = Some(command.to_string());
    }

    #[func]
    fn on_entry_text_changed(&mut self) {
        self.clear_autocomplete();
        let text = self.get_entry_text();
        if !text.is_empty() {
            self.update_autocomplete();
        } else {
            self.history_iter.reset();
        }
    }

    #[func]
    fn on_autocomplete_requested(&mut self) {
        self.autocomplete();
    }

    #[func]
    fn on_reverse_autocomplete_requested(&mut self) {
        self.reverse_autocomplete();
    }

    #[func]
    fn on_history_up_requested(&mut self) {
        let prev = self.history_iter.prev();
        self.fill_entry(&prev);
        self.clear_autocomplete();
        self.update_autocomplete();
    }

    #[func]
    fn on_history_down_requested(&mut self) {
        let next = self.history_iter.next();
        self.fill_entry(&next);
        self.clear_autocomplete();
        self.update_autocomplete();
    }

    #[func]
    fn on_scroll_up_requested(&mut self) {
        if let Some(ref mut output) = self.output {
            if let Some(mut scroll_bar) = output.get_v_scroll_bar() {
                let val = scroll_bar.get_value();
                let page = scroll_bar.get_page();
                scroll_bar.set_value(val - page);
            }
        }
    }

    #[func]
    fn on_scroll_down_requested(&mut self) {
        if let Some(ref mut output) = self.output {
            if let Some(mut scroll_bar) = output.get_v_scroll_bar() {
                let val = scroll_bar.get_value();
                let page = scroll_bar.get_page();
                scroll_bar.set_value(val + page);
            }
        }
    }

    // --- Process + Input callbacks (connected to scene tree) ---

    #[func]
    fn on_process_frame(&mut self) {
        if !self.initialized {
            return;
        }

        // Execute pending command from on_entry_text_submitted.
        // We take it out of self, then schedule a Callable::from_fn that runs
        // execute_command_on OUTSIDE of any #[func] borrow — avoiding re-entrancy.
        if let Some(cmd) = self.pending_command.take() {
            let mut gd = self.to_gd();
            let callable = Callable::from_fn("_dispatch_cmd", move |_args| {
                Self::execute_command_on(&mut gd, &cmd, false);
                gd.bind_mut().update_autocomplete();
                Variant::nil()
            });
            callable.call_deferred(&[]);
        }

        // Handle input polling
        self.poll_input();

        // Handle animation
        let is_processing = self.canvas_layer.as_ref().map_or(false, |cl| cl.is_processing());
        if !is_processing {
            return;
        }

        let delta = 1.0 / Engine::singleton().get_frames_per_second().max(1.0) as f32;
        let time_scale = Engine::singleton().get_time_scale() as f32;
        let mut done_sliding = false;

        if self.is_open {
            self.open_t += self.open_speed * delta * (1.0 / time_scale);
            if self.open_t >= 1.0 {
                self.open_t = 1.0;
                done_sliding = true;
            }
        } else {
            self.open_t -= self.open_speed * delta * 1.5 * (1.0 / time_scale);
            if self.open_t <= 0.0 {
                self.open_t = 0.0;
                done_sliding = true;
            }
        }

        let eased = ease(self.open_t as f64, -1.75) as f32;
        if let Some(ref mut control) = self.control {
            let height = control.get_size().y;
            let new_y = remap(eased as f64, 0.0, 1.0, -height as f64, 0.0) as f32;
            control.set_position(Vector2::new(0.0, new_y));
        }

        if done_sliding {
            if let Some(ref mut cl) = self.canvas_layer {
                cl.set_process(false);
            }
            if !self.is_open {
                self.hide_console();
            }
        }
    }

    #[func]
    fn on_unhandled_input(&mut self, event: Gd<InputEvent>) {
        if !self.enabled || !self.initialized {
            return;
        }

        if event.is_action_pressed("tiny_console_toggle") {
            self.toggle_console();
            let tree = Self::get_scene_tree();
            if let Some(mut vp) = tree.get_root() {
                vp.set_input_as_handled();
            }
            return;
        }

        let control_visible = self.control.as_ref().map_or(false, |c| c.is_visible());
        let history_visible = self.history_gui.as_ref().map_or(false, |h| h.is_visible());

        if control_visible && event.is_action_pressed("tiny_console_search_history") {
            self.toggle_history();
            let tree = Self::get_scene_tree();
            if let Some(mut vp) = tree.get_root() {
                vp.set_input_as_handled();
            }
            return;
        }

        if let Ok(key_event) = event.try_cast::<InputEventKey>() {
            if !key_event.is_pressed() {
                return;
            }
            if history_visible {
                self.handle_history_input(&key_event);
            } else if control_visible {
                self.handle_command_input(&key_event);
            }
        }
    }
}

// === Helpers called by highlighter ===
impl TinyConsole {
    pub fn has_command_str(&self, name: &str) -> bool {
        self.commands.contains_key(name)
    }

    pub fn has_alias_str(&self, name: &str) -> bool {
        self.aliases.contains_key(name)
    }

    fn get_scene_tree() -> Gd<SceneTree> {
        Engine::singleton().get_main_loop().unwrap().cast::<SceneTree>()
    }
}

// === Private implementation ===

impl TinyConsole {
    fn print_line_internal(&mut self, line: &str, stdout: bool) {
        if self.silent {
            return;
        }
        if let Some(ref mut output) = self.output {
            output.append_text(&GString::from(format!("{}\n", line).as_str()));
            let line_count = output.get_line_count();
            output.scroll_to_line(line_count);
        }
        if stdout {
            godot_print!("{}", util::bbcode_strip(line));
        }
    }

    fn get_entry_text(&self) -> String {
        match &self.entry {
            Some(entry) => entry.get_text().to_string(),
            None => String::new(),
        }
    }

    fn register_input_actions(&self) {
        let mut input_map = InputMap::singleton();

        // tiny_console_toggle - backtick key
        if !input_map.has_action("tiny_console_toggle") {
            input_map.add_action("tiny_console_toggle");
            let mut ev = InputEventKey::new_gd();
            ev.set_keycode(Key::QUOTELEFT);
            input_map.action_add_event("tiny_console_toggle", &ev);
        }

        // tiny_auto_complete_reverse - Shift+Tab
        if !input_map.has_action("tiny_auto_complete_reverse") {
            input_map.add_action("tiny_auto_complete_reverse");
            let mut ev = InputEventKey::new_gd();
            ev.set_keycode(Key::TAB);
            ev.set_shift_pressed(true);
            input_map.action_add_event("tiny_auto_complete_reverse", &ev);
        }

        // tiny_console_search_history - Ctrl+R
        if !input_map.has_action("tiny_console_search_history") {
            input_map.add_action("tiny_console_search_history");
            let mut ev = InputEventKey::new_gd();
            ev.set_keycode(Key::R);
            ev.set_ctrl_pressed(true);
            input_map.action_add_event("tiny_console_search_history", &ev);
        }
    }

    fn poll_input(&mut self) {
        if !self.enabled {
            return;
        }

        // We use Input singleton to check actions each frame
        let input = godot::classes::Input::singleton();

        if input.is_action_just_pressed("tiny_console_toggle") {
            self.toggle_console();
        }

        let control_visible = self.control.as_ref().map_or(false, |c| c.is_visible());
        if !control_visible {
            return;
        }

        if input.is_action_just_pressed("tiny_console_search_history") {
            self.toggle_history();
        }
    }

    fn build_gui(&mut self) {
        // Create the canvas layer
        let canvas_layer = CanvasLayer::new_alloc();
        self.canvas_layer = Some(canvas_layer);

        let cl = self.canvas_layer.as_mut().unwrap();

        // Create control block (to block mouse input)
        let mut con = Control::new_alloc();
        con.set_anchors_preset(LayoutPreset::FULL_RECT);
        cl.add_child(&con);
        self.control_block = Some(con);

        // Create PanelContainer
        let mut panel = PanelContainer::new_alloc();
        panel.set_anchor_ex(Side::BOTTOM, self.options.height_ratio).done();
        panel.set_anchor_ex(Side::RIGHT, 1.0).done();
        cl.add_child(&panel);

        // Create VBoxContainer
        let mut vbox = VBoxContainer::new_alloc();
        vbox.set_anchors_preset(LayoutPreset::FULL_RECT);
        panel.add_child(&vbox);

        // Create RichTextLabel (output)
        let mut output = RichTextLabel::new_alloc();
        output.set_v_size_flags(SizeFlags::EXPAND_FILL);
        output.set_scroll_active(true);
        output.set_scroll_follow(true);
        output.set_use_bbcode(true);
        output.set_focus_mode(FocusMode::CLICK);
        vbox.add_child(&output);

        // Create CommandEntry (input)
        let entry = CommandEntry::new_alloc();
        vbox.add_child(&entry);

        // Set opacity
        panel.set_modulate(Color::from_rgba(1.0, 1.0, 1.0, self.options.opacity));

        // Create HistoryGui
        let mut history_gui = HistoryGui::new_alloc();
        output.add_child(&history_gui);
        history_gui.set_visible(false);

        self.control = Some(panel);
        self.output = Some(output);
        self.entry = Some(entry);
        self.history_gui = Some(history_gui);
    }

    fn init_theme(&mut self) {
        let custom_theme_path = GString::from(self.options.custom_theme.as_str());
        let default_theme_path = GString::from(THEME_DEFAULT);

        let theme: Option<Gd<Theme>> = if ResourceLoader::singleton().exists_ex(&custom_theme_path).type_hint("Theme").done() {
            ResourceLoader::singleton().load_ex(&custom_theme_path).done().and_then(|r| r.try_cast::<Theme>().ok())
        } else {
            ResourceLoader::singleton().load_ex(&default_theme_path).done().and_then(|r| r.try_cast::<Theme>().ok())
        };

        if let Some(theme) = theme {
            if let Some(ref mut panel) = self.control {
                panel.set_theme(&theme);
            }

            let ctype = &StringName::from(CONSOLE_COLORS_THEME_TYPE);
            self.output_command_color = theme.get_color(&StringName::from("output_command_color"), ctype);
            self.output_command_mention_color = theme.get_color(&StringName::from("output_command_mention_color"), ctype);
            self.output_text_color = theme.get_color(&StringName::from("output_text_color"), ctype);
            self.output_error_color = theme.get_color(&StringName::from("output_error_color"), ctype);
            self.output_warning_color = theme.get_color(&StringName::from("output_warning_color"), ctype);
            self.output_debug_color = theme.get_color(&StringName::from("output_debug_color"), ctype);
            self.entry_text_color = theme.get_color(&StringName::from("entry_text_color"), ctype);
            self.entry_hint_color = theme.get_color(&StringName::from("entry_hint_color"), ctype);
            self.entry_command_found_color = theme.get_color(&StringName::from("entry_command_found_color"), ctype);
            self.entry_subcommand_color = theme.get_color(&StringName::from("entry_subcommand_color"), ctype);
            self.entry_command_not_found_color = theme.get_color(&StringName::from("entry_command_not_found_color"), ctype);

            // Apply to output
            if let Some(ref mut output) = self.output {
                output.add_theme_color_override("default_color", self.output_text_color);
            }

            // Apply to entry
            if let Some(ref mut entry) = self.entry {
                entry.add_theme_color_override("font_color", self.entry_text_color);
                entry.add_theme_color_override("hint_color", self.entry_hint_color);

                if let Some(highlighter) = entry.get_syntax_highlighter() {
                    if let Ok(mut hl) = highlighter.try_cast::<CommandEntryHighlighter>() {
                        {
                            let mut hl_ref = hl.bind_mut();
                            hl_ref.command_found_color = self.entry_command_found_color;
                            hl_ref.command_not_found_color = self.entry_command_not_found_color;
                            hl_ref.subcommand_color = self.entry_subcommand_color;
                            hl_ref.text_color = self.entry_text_color;
                        }
                    }
                }
            }
        }
    }

    fn greet(&mut self) {
        let mut message = self.options.greeting_message.clone();
        let project_name = ProjectSettings::singleton().get_setting("application/config/name").to::<GString>().to_string();
        let project_version = ProjectSettings::singleton().get_setting_with_override("application/config/version").to::<GString>().to_string();
        message = message.replace("{project_name}", &project_name);
        message = message.replace("{project_version}", &project_version);

        if !message.is_empty() {
            if self.options.greet_using_ascii_art && ascii_art::is_boxed_art_supported(&message) {
                self.print_boxed(GString::from(message.as_str()));
                self.print_line_internal("", false);
            } else {
                let msg = format!("[b]{}[/b]", message);
                self.print_line_internal(&msg, false);
            }
        }

        self.cmd_help(GString::new());
        let debug_color = self.output_debug_color.to_html();
        let tip = format!("[i][color={}]-----[/color][/i]", debug_color);
        self.print_line_internal(&tip, false);
    }

    fn add_aliases_from_config(&mut self) {
        let aliases = self.options.aliases.clone();
        for (alias, target) in aliases {
            if self.commands.contains_key(&alias) {
                godot_error!("TinyConsole: Config error: Alias or command already registered: {}", alias);
            } else if !self.commands.contains_key(&target) {
                godot_error!("TinyConsole: Config error: Alias target not found: {}", target);
            } else {
                self.add_alias(GString::from(alias.as_str()), GString::from(target.as_str()));
            }
        }
    }

    // --- Parsing ---

    fn parse_command_line(&self, line: &str) -> Vec<String> {
        let mut argv = Vec::new();
        let line = line.trim();
        if line.is_empty() {
            return argv;
        }
        let mut in_quotes = false;
        let mut in_brackets = false;
        let mut start = 0usize;
        let chars: Vec<char> = line.chars().collect();

        for (cur, &ch) in chars.iter().enumerate() {
            match ch {
                '"' => in_quotes = !in_quotes,
                '(' => in_brackets = true,
                ')' => in_brackets = false,
                ' ' if !in_quotes && !in_brackets => {
                    if cur > start {
                        let byte_start = chars[..start].iter().map(|c| c.len_utf8()).sum::<usize>();
                        let byte_end = chars[..cur].iter().map(|c| c.len_utf8()).sum::<usize>();
                        argv.push(line[byte_start..byte_end].to_string());
                    }
                    start = cur + 1;
                }
                _ => {}
            }
        }
        if chars.len() > start {
            let byte_start = chars[..start].iter().map(|c| c.len_utf8()).sum::<usize>();
            argv.push(line[byte_start..].to_string());
        }
        argv
    }

    fn join_subcommands(&self, argv: Vec<String>) -> Vec<String> {
        for num_parts in (2..=MAX_SUBCOMMANDS).rev() {
            if argv.len() >= num_parts {
                let cmd = argv[..num_parts].join(" ");
                if self.commands.contains_key(&cmd) || self.aliases.contains_key(&cmd) {
                    let mut result = vec![cmd];
                    result.extend(argv[num_parts..].iter().cloned());
                    return result;
                }
            }
        }
        argv
    }

    fn expand_alias(&self, argv: Vec<String>) -> Vec<String> {
        let mut argv = argv;
        let mut result = Vec::new();
        let max_depth = 1000;
        let mut current_depth = 0;

        while !argv.is_empty() && current_depth < max_depth {
            argv = self.join_subcommands(argv);
            let current = argv.remove(0);
            current_depth += 1;

            if let Some(alias_argv) = self.aliases.get(&current) {
                let mut expanded = alias_argv.clone();
                expanded.extend(argv);
                argv = expanded;
            } else {
                result.push(current);
            }
        }

        if current_depth >= max_depth {
            godot_error!("TinyConsole: Max depth for alias reached. Loop in aliasing?");
            return argv;
        }
        result
    }

    /// Prepares a command for execution: parses, validates, echoes to output.
    /// Returns `Some((callable, args, argv))` if a user callable should be invoked,
    /// or `None` if the command was handled entirely (error, empty, etc.).
    fn prepare_command(&mut self, command_line: &str, silent: bool) -> Option<(Callable, VarArray, Vec<String>)> {
        let command_line = command_line.trim();
        if command_line.is_empty() || command_line.starts_with('#') {
            return None;
        }

        let argv = self.parse_command_line(command_line);
        let expanded_argv = self.expand_alias(argv.clone());
        let expanded_argv = self.join_subcommands(expanded_argv);

        if expanded_argv.is_empty() {
            return None;
        }

        let command_name = expanded_argv[0].clone();

        self.silent = silent;
        if !silent {
            let history_line = argv.join(" ");
            self.history.push_entry(history_line);
            self.history.reassign_iterator(&mut self.history_iter);

            let color = self.output_command_color.to_html();
            let rest = argv[1..].join(" ");
            let msg = format!("[color={}][b]>[/b] {}[/color] {}", color, argv[0], rest);
            self.print_line_internal(&msg, false);
        }

        if !self.commands.contains_key(&command_name) {
            let msg = format!("Unknown command: {}", command_name);
            let color = self.output_error_color.to_html();
            let err_msg = format!("[color={}]ERROR:[/color] {}", color, msg);
            self.print_line_internal(&err_msg, false);
            self.suggest_similar_command(&expanded_argv);
            self.silent = false;
            return None;
        }

        let callable = self.commands.get(&command_name).unwrap().clone();
        let method_info = self.get_method_info(&callable);

        let call_args = self.parse_argv(&expanded_argv, &callable, &method_info);
        match call_args {
            Some(args) => Some((callable, args, expanded_argv)),
            None => {
                self.usage(GString::from(argv[0].as_str()));
                self.silent = false;
                None
            }
        }
    }

    /// Called after the user callable has been invoked (outside the mutable borrow).
    fn finish_command(&mut self, result: &Variant, expanded_argv: &[String]) {
        if let Ok(err_code) = result.try_to::<i32>() {
            if err_code > 0 {
                self.suggest_argument_corrections(expanded_argv);
            }
        }

        if self.options.sparse_mode {
            self.print_line_internal("", false);
        }
        self.silent = false;
    }

    /// Executes a command, properly releasing the mutable borrow before calling
    /// the user's callable (which may call back into TinyConsole).
    /// Must be called on a `Gd<TinyConsole>`, not on `&mut self`.
    pub fn execute_command_on(this: &mut Gd<TinyConsole>, command_line: &str, silent: bool) {
        let pending = this.bind_mut().prepare_command(command_line, silent);
        // bind_mut() is dropped here — self is no longer borrowed

        if let Some((callable, args, expanded_argv)) = pending {
            // Safe: the singleton is not borrowed during callv
            let result = callable.callv(&args);

            // Re-borrow to finish up
            this.bind_mut().finish_command(&result, &expanded_argv);
        }
    }

    fn parse_argv(&mut self, argv: &[String], callable: &Callable, method_info: &Option<MethodInfo>) -> Option<VarArray> {
        let info = match method_info {
            Some(i) => i,
            None => {
                let mut args = VarArray::new();
                for arg in &argv[1..] {
                    args.push(&arg.to_variant());
                }
                return Some(args);
            }
        };

        let bound_args = callable.get_bound_arguments_count() as usize;
        let num_args = argv.len() + bound_args - 1;
        let max_args = info.args.len();
        let required_args = max_args.saturating_sub(info.default_count);

        // If callable accepts a single String argument, join all args
        if max_args.saturating_sub(bound_args) == 1 && !info.args.is_empty() && info.args[0].type_id == 4 {
            let mut joined = argv[1..].join(" ");
            if joined.starts_with('"') && joined.ends_with('"') && joined.len() >= 2 {
                joined = joined[1..joined.len() - 1].to_string();
            }
            let mut args = VarArray::new();
            args.push(&joined.to_variant());
            return Some(args);
        }

        if num_args < required_args {
            self.error("Missing arguments.".into());
            return None;
        }
        if num_args > max_args {
            self.error("Too many arguments.".into());
            return None;
        }

        let mut args = VarArray::new();
        for (i, arg_str) in argv[1..].iter().enumerate() {
            let expected_type = if i < info.args.len() { info.args[i].type_id } else { 0 };

            let parsed = self.parse_single_arg(arg_str, expected_type);
            match parsed {
                Some(v) => args.push(&v),
                None => return None,
            }
        }

        Some(args)
    }

    fn parse_single_arg(&mut self, arg: &str, expected_type: i32) -> Option<Variant> {
        if expected_type == 4 {
            let cleaned = if arg.starts_with('"') && arg.ends_with('"') && arg.len() >= 2 {
                &arg[1..arg.len() - 1]
            } else {
                arg
            };
            return Some(cleaned.to_variant());
        }

        if arg.starts_with('(') && arg.ends_with(')') {
            return self.parse_vector_arg(arg);
        }

        if let Ok(f) = arg.parse::<f64>() {
            if !arg.contains('.') && !arg.contains('e') && !arg.contains('E') {
                if let Ok(i) = arg.parse::<i64>() {
                    return Some(i.to_variant());
                }
            }
            return Some(f.to_variant());
        }

        if arg.starts_with("0x") || arg.starts_with("0X") {
            if let Ok(i) = i64::from_str_radix(&arg[2..], 16) {
                return Some(i.to_variant());
            }
        }

        match arg.to_lowercase().as_str() {
            "true" | "yes" => return Some(true.to_variant()),
            "false" | "no" => return Some(false.to_variant()),
            _ => {}
        }

        let cleaned = if arg.starts_with('"') && arg.ends_with('"') && arg.len() >= 2 {
            &arg[1..arg.len() - 1]
        } else {
            arg
        };
        Some(cleaned.to_variant())
    }

    fn parse_vector_arg(&mut self, text: &str) -> Option<Variant> {
        let inner = &text[1..text.len() - 1];
        let mut components = Vec::new();
        let mut token = String::new();

        for ch in inner.chars() {
            if ch.is_ascii_digit() || ch == '.' || ch == '-' {
                token.push(ch);
            } else if ch == ',' || ch == ' ' {
                if token.is_empty() && ch == ',' {
                    token = "0".to_string();
                }
                if !token.is_empty() {
                    match token.parse::<f32>() {
                        Ok(f) => components.push(f),
                        Err(_) => {
                            let msg = format!("Failed to parse vector: Not a number: \"{}\"", token);
                            self.error(GString::from(msg.as_str()));
                            return None;
                        }
                    }
                    token.clear();
                }
            } else {
                let msg = format!("Failed to parse vector: Bad formatting: \"{}\"", text);
                self.error(GString::from(msg.as_str()));
                return None;
            }
        }
        if !token.is_empty() {
            match token.parse::<f32>() {
                Ok(f) => components.push(f),
                Err(_) => {
                    let msg = format!("Failed to parse vector: Not a number: \"{}\"", token);
                    self.error(GString::from(msg.as_str()));
                    return None;
                }
            }
        }

        match components.len() {
            2 => Some(Vector2::new(components[0], components[1]).to_variant()),
            3 => Some(Vector3::new(components[0], components[1], components[2]).to_variant()),
            4 => Some(Vector4::new(components[0], components[1], components[2], components[3]).to_variant()),
            _ => {
                let msg = format!("Supports 2,3,4-element vectors, but {}-element given.", components.len());
                self.error(GString::from(msg.as_str()));
                None
            }
        }
    }

    fn get_method_info(&self, callable: &Callable) -> Option<MethodInfo> {
        let method_name = callable.method_name()?;
        if method_name.is_empty() {
            return None;
        }

        if let Some(obj) = callable.object() {
            let method_list = obj.get_method_list();
            for m in method_list.iter_shared() {
                let name = m.get("name").unwrap_or_default().to::<GString>();
                if name.to_string() == method_name.to_string() {
                    return Some(parse_method_dict(&m));
                }
            }
        }

        None
    }

    // --- Autocomplete ---

    fn autocomplete(&mut self) {
        if !self.autocomplete_matches.is_empty() {
            let match_str = self.autocomplete_matches.remove(0);
            self.autocomplete_matches.push(match_str.clone());
            self.fill_entry(&match_str);
            self.update_autocomplete();
        }
    }

    fn reverse_autocomplete(&mut self) {
        if !self.autocomplete_matches.is_empty() {
            let last = self.autocomplete_matches.pop().unwrap();
            self.autocomplete_matches.insert(0, last);
            let match_str = self.autocomplete_matches.last().unwrap().clone();
            self.fill_entry(&match_str);
            self.update_autocomplete();
        }
    }

    fn update_autocomplete(&mut self) {
        let entry_text = self.get_entry_text();
        let mut argv = self.expand_alias(self.parse_command_line(&entry_text));
        if entry_text.ends_with(' ') || argv.is_empty() {
            argv.push(String::new());
        }
        let command_name = argv[0].clone();
        let last_arg = argv.len() - 1;

        if self.autocomplete_matches.is_empty() && !entry_text.is_empty() {
            if last_arg == 0 && !argv[0].is_empty() && !argv[0].contains(' ') {
                self.add_first_input_autocompletes(&command_name);
            } else if last_arg != 0 {
                self.add_argument_autocompletes(&argv);
                self.add_subcommand_autocompletes(&entry_text);
                self.add_history_autocompletes();
            }
        }

        if !self.autocomplete_matches.is_empty() {
            let first = &self.autocomplete_matches[0];
            if first.len() > entry_text.len() && first.starts_with(&entry_text) {
                let hint = first[entry_text.len()..].to_string();
                if let Some(ref mut entry) = self.entry {
                    entry.bind_mut().set_autocomplete_hint_value(GString::from(hint.as_str()));
                }
                return;
            }
        }

        if let Some(ref mut entry) = self.entry {
            entry.bind_mut().set_autocomplete_hint_value(GString::new());
        }
    }

    fn add_first_input_autocompletes(&mut self, command_name: &str) {
        let mut matches = Vec::new();
        let all_names = self.get_all_command_names_with_aliases();
        for cmd_name in &all_names {
            let first_input = cmd_name.split(' ').next().unwrap_or("");
            if first_input.starts_with(command_name) && !matches.contains(&first_input.to_string()) {
                matches.push(first_input.to_string());
            }
        }
        matches.sort();
        self.autocomplete_matches.extend(matches);
    }

    fn add_argument_autocompletes(&mut self, argv: &[String]) {
        if argv.is_empty() {
            return;
        }
        let command = &argv[0];
        let last_arg = argv.len() - 1;
        let key = (command.clone(), last_arg - 1);

        if let Some(source) = self.argument_autocomplete_sources.get(&key).cloned() {
            let result = source.callv(&VarArray::new());
            if let Some(values) = variant_to_string_vec(&result) {
                let entry_text = self.get_entry_text();
                let typed_arg = &argv[last_arg];
                let mut matches = Vec::new();
                for val_str in &values {
                    if val_str.starts_with(typed_arg) {
                        let prefix_len = entry_text.len() - typed_arg.len();
                        let full_match = format!("{}{}", &entry_text[..prefix_len], val_str);
                        matches.push(full_match);
                    }
                }
                matches.sort();
                self.autocomplete_matches.extend(matches);
            }
        }
    }

    fn add_history_autocompletes(&mut self) {
        if self.options.autocomplete_use_history_with_matches || self.autocomplete_matches.is_empty() {
            let entry_text = self.get_entry_text();
            let entries = self.history.entries().to_vec();
            for entry in entries.iter().rev() {
                if entry.starts_with(&entry_text) {
                    self.autocomplete_matches.push(entry.clone());
                }
            }
        }
    }

    fn add_subcommand_autocompletes(&mut self, typed_val: &str) {
        let all_names = self.get_all_command_names_with_aliases();
        let typed_tokens: Vec<&str> = typed_val.split(' ').collect();
        let mut result_set: Vec<String> = Vec::new();

        for cmd in &all_names {
            let cmd_tokens: Vec<&str> = cmd.split(' ').collect();
            if cmd_tokens.len() < typed_tokens.len() {
                continue;
            }

            let mut last_match = 0;
            for i in 0..typed_tokens.len() {
                if i < cmd_tokens.len() && cmd_tokens[i] != typed_tokens[i] {
                    break;
                }
                last_match += 1;
            }

            if last_match < typed_tokens.len().saturating_sub(1) {
                continue;
            }

            if cmd_tokens.len() >= typed_tokens.len() && last_match < cmd_tokens.len() && cmd_tokens[last_match].starts_with(typed_tokens.last().unwrap_or(&"")) {
                let partial = cmd_tokens[..last_match + 1].join(" ");
                if !result_set.contains(&partial) {
                    result_set.push(partial);
                }
            }
        }

        result_set.sort();
        self.autocomplete_matches.extend(result_set);
    }

    fn clear_autocomplete(&mut self) {
        self.autocomplete_matches.clear();
        if let Some(ref mut entry) = self.entry {
            entry.bind_mut().set_autocomplete_hint_value(GString::new());
        }
    }

    fn suggest_similar_command(&mut self, argv: &[String]) {
        if self.silent || argv.is_empty() {
            return;
        }
        let all_names = self.get_all_command_names_with_aliases();
        if let Some(fuzzy_hit) = util::fuzzy_match_string(&argv[0], 2, &all_names) {
            let color = self.output_command_mention_color.to_html();
            let debug_color = self.output_debug_color.to_html();
            let tip = format!("[i][color={}]Did you mean [color={}]{}[/color]? ([b]TAB[/b] to fill)[/color][/i]", debug_color, color, fuzzy_hit);
            self.print_line_internal(&tip, false);

            let mut suggest = argv.to_vec();
            suggest[0] = fuzzy_hit;
            let suggest_command = suggest.join(" ").trim().to_string();
            self.autocomplete_matches.push(suggest_command);
        }
    }

    fn suggest_argument_corrections(&mut self, argv: &[String]) {
        if self.silent || argv.is_empty() {
            return;
        }
        let command_name = &argv[0];
        let actual_cmd = if let Some(alias_argv) = self.aliases.get(command_name) {
            alias_argv[0].clone()
        } else {
            command_name.clone()
        };

        let mut corrected_argv = argv.to_vec();
        corrected_argv[0] = actual_cmd.clone();
        let mut any_corrected = false;

        for i in 1..argv.len() {
            let key = (actual_cmd.clone(), i);
            if let Some(source) = self.argument_autocomplete_sources.get(&key).cloned() {
                let result = source.callv(&VarArray::new());
                if let Some(values) = variant_to_string_vec(&result) {
                    if let Some(hit) = util::fuzzy_match_string(&argv[i], 2, &values) {
                        corrected_argv[i] = hit;
                        any_corrected = true;
                    }
                }
            }
        }

        if any_corrected {
            let color = self.output_command_mention_color.to_html();
            let debug_color = self.output_debug_color.to_html();
            let args_str = corrected_argv[1..].join(" ");
            let tip = format!(
                "[i][color={}]Did you mean \"[color={}]{}[/color] {}\"? ([b]TAB[/b] to fill)[/color][/i]",
                debug_color, color, actual_cmd, args_str
            );
            self.print_line_internal(&tip, false);
            let suggest = corrected_argv.join(" ").trim().to_string();
            self.autocomplete_matches.push(suggest);
        }
    }

    fn get_all_command_names_with_aliases(&self) -> Vec<String> {
        let mut names: Vec<String> = self.commands.keys().cloned().collect();
        names.extend(self.aliases.keys().cloned());
        names.sort();
        names
    }

    // --- Console show/hide ---

    fn show_console(&mut self) {
        let is_visible = self.control.as_ref().map_or(false, |c| c.is_visible());
        if !is_visible && self.enabled {
            if let Some(ref mut control) = self.control {
                control.set_visible(true);
            }
            if let Some(ref mut block) = self.control_block {
                block.set_visible(true);
            }

            if self.options.pause_when_open {
                let mut tree = Self::get_scene_tree();
                self.was_already_paused = tree.is_paused();
                if !self.was_already_paused {
                    tree.set_pause(true);
                }
            }

            let tree = Self::get_scene_tree();
            if let Some(root) = tree.get_root() {
                self.previous_gui_focus = root.gui_get_focus_owner().and_then(|f| f.try_cast::<Control>().ok());
            }

            if let Some(ref mut entry) = self.entry {
                entry.grab_focus();
            }

            self.base_mut().emit_signal("toggled", &[true.to_variant()]);
        }
    }

    fn hide_console(&mut self) {
        let is_visible = self.control.as_ref().map_or(false, |c| c.is_visible());
        if is_visible {
            if let Some(ref mut control) = self.control {
                control.set_visible(false);
            }
            if let Some(ref mut block) = self.control_block {
                block.set_visible(false);
            }

            if self.options.pause_when_open && !self.was_already_paused {
                let mut tree = Self::get_scene_tree();
                tree.set_pause(false);
            }

            if let Some(ref prev) = self.previous_gui_focus {
                if prev.is_instance_valid() {
                    prev.clone().grab_focus();
                }
            }
            self.previous_gui_focus = None;

            self.base_mut().emit_signal("toggled", &[false.to_variant()]);
        }
    }

    fn fill_entry(&mut self, line: &str) {
        if let Some(ref mut entry) = self.entry {
            entry.set_text(&GString::from(line));
            entry.set_caret_column(line.len() as i32);
        }
    }

    // --- Input handling ---

    fn handle_command_input(&mut self, event: &Gd<InputEventKey>) {
        if !self.is_open {
            return;
        }

        let keycode = event.get_keycode();
        let mut handled = true;

        if keycode == Key::UP {
            let prev = self.history_iter.prev();
            self.fill_entry(&prev);
            self.clear_autocomplete();
            self.update_autocomplete();
        } else if keycode == Key::DOWN {
            let next = self.history_iter.next();
            self.fill_entry(&next);
            self.clear_autocomplete();
            self.update_autocomplete();
        } else if event.is_action_pressed("tiny_auto_complete_reverse") {
            self.reverse_autocomplete();
        } else if keycode == Key::TAB {
            self.autocomplete();
        } else if keycode == Key::PAGEUP {
            if let Some(ref mut output) = self.output {
                if let Some(mut scroll_bar) = output.get_v_scroll_bar() {
                    let val = scroll_bar.get_value();
                    let page = scroll_bar.get_page();
                    scroll_bar.set_value(val - page);
                }
            }
        } else if keycode == Key::PAGEDOWN {
            if let Some(ref mut output) = self.output {
                if let Some(mut scroll_bar) = output.get_v_scroll_bar() {
                    let val = scroll_bar.get_value();
                    let page = scroll_bar.get_page();
                    scroll_bar.set_value(val + page);
                }
            }
        } else {
            handled = false;
        }

        if handled {
            let tree = Self::get_scene_tree();
            if let Some(mut vp) = tree.get_root() {
                vp.set_input_as_handled();
            }
        }
    }

    fn handle_history_input(&mut self, event: &Gd<InputEventKey>) {
        if event.is_action_pressed("tiny_auto_complete_reverse") {
            self.reverse_autocomplete();
            let tree = Self::get_scene_tree();
            if let Some(mut vp) = tree.get_root() {
                vp.set_input_as_handled();
            }
        } else if event.get_keycode() == Key::TAB && event.is_pressed() {
            self.autocomplete();
            let tree = Self::get_scene_tree();
            if let Some(mut vp) = tree.get_root() {
                vp.set_input_as_handled();
            }
        } else {
            let entry_text = self.get_entry_text();
            let results = self.history.fuzzy_match(&entry_text);
            let packed: PackedStringArray = results.iter().map(|s| GString::from(s.as_str())).collect();
            if let Some(ref mut hg) = self.history_gui {
                hg.bind_mut().set_search_results(packed);
            }
        }

        if let Some(ref mut entry) = self.entry {
            entry.grab_focus();
        }
    }

    pub fn cleanup(&mut self) {
        if self.options.persist_history {
            self.history.trim(self.options.history_lines as usize);
            self.history.save(command_history::HISTORY_FILE);
        }

        self.initialized = false;

        // Disconnect from scene tree
        let this = self.to_gd();
        let mut tree = Self::get_scene_tree();
        if tree.is_connected("process_frame", &Callable::from_object_method(&this, "on_process_frame")) {
            tree.disconnect("process_frame", &Callable::from_object_method(&this, "on_process_frame"));
        }

        // Clear callables and state that may reference external objects
        self.commands.clear();
        self.aliases.clear();
        self.command_descriptions.clear();
        self.argument_autocomplete_sources.clear();
        self.pending_command = None;

        // Drop all Gd references to child nodes before freeing the canvas layer
        self.entry = None;
        self.output = None;
        self.control = None;
        self.control_block = None;
        self.history_gui = None;
        self.previous_gui_focus = None;

        // Remove canvas layer from tree and free it immediately.
        // Using free() instead of queue_free() because during engine shutdown
        // the scene tree won't process another frame to handle deferred frees.
        if let Some(cl) = self.canvas_layer.take() {
            if cl.is_inside_tree() {
                if let Some(mut parent) = cl.get_parent() {
                    parent.remove_child(&cl);
                }
            }
            cl.free();
        }
    }
}

// === IObject implementation ===

#[godot_api]
impl IObject for TinyConsole {
    fn init(base: Base<Object>) -> Self {
        let mut options = ConsoleOptions::new();
        options.register_project_settings();
        options.load_from_project_settings();

        let mut history = CommandHistory::new();
        if options.persist_history {
            history.load(command_history::HISTORY_FILE);
        }
        let history_iter = history.create_iterator();

        Self {
            base,
            canvas_layer: None,

            control: None,
            control_block: None,
            output: None,
            entry: None,
            history_gui: None,
            previous_gui_focus: None,

            output_command_color: Color::WHITE,
            output_command_mention_color: Color::WHITE,
            output_error_color: Color::from_rgba(1.0, 0.3, 0.3, 1.0),
            output_warning_color: Color::from_rgba(1.0, 1.0, 0.3, 1.0),
            output_text_color: Color::WHITE,
            output_debug_color: Color::from_rgba(0.6, 0.6, 0.6, 1.0),
            entry_text_color: Color::WHITE,
            entry_hint_color: Color::from_rgba(0.5, 0.5, 0.5, 1.0),
            entry_command_found_color: Color::from_rgba(0.73, 0.90, 0.49, 1.0),
            entry_subcommand_color: Color::from_rgba(0.58, 0.90, 0.80, 1.0),
            entry_command_not_found_color: Color::from_rgba(1.0, 0.2, 0.2, 1.0),

            enabled: true,
            initialized: false,
            options,
            commands: HashMap::new(),
            aliases: HashMap::new(),
            command_descriptions: HashMap::new(),
            argument_autocomplete_sources: HashMap::new(),
            history,
            history_iter,
            autocomplete_matches: Vec::new(),
            eval_inputs: HashMap::new(),
            silent: false,
            was_already_paused: false,
            open_t: 0.0,
            open_speed: 5.0,
            is_open: false,
            pending_command: None,
        }
    }
}

// === Helper types ===

pub struct MethodInfo {
    pub args: Vec<ArgInfo>,
    pub default_count: usize,
    pub defaults: Vec<String>,
}

pub struct ArgInfo {
    pub name: String,
    pub type_id: i32,
}

fn parse_method_dict(dict: &VarDictionary) -> MethodInfo {
    let args_variant = dict.get("args").unwrap_or_default();
    let defaults_variant = dict.get("default_args").unwrap_or_default();

    let mut args = Vec::new();
    // args is a typed Array[Dictionary] — iterate via Variant to avoid type mismatch
    if let Ok(args_array) = args_variant.try_to::<Array<VarDictionary>>() {
        for arg_dict in args_array.iter_shared() {
            let name = arg_dict.get("name").unwrap_or_default().to::<GString>().to_string().trim_start_matches("p_").to_string();
            let type_id = arg_dict.get("type").unwrap_or_default().to::<i32>();
            args.push(ArgInfo { name, type_id });
        }
    }

    let mut defaults = Vec::new();
    if let Ok(defaults_array) = defaults_variant.try_to::<VarArray>() {
        for d in defaults_array.iter_shared() {
            defaults.push(d.to_string());
        }
    }

    MethodInfo {
        default_count: defaults.len(),
        args,
        defaults,
    }
}

/// Extract string elements from a Variant that holds an Array.
/// Works with both typed (Array[String]) and untyped (Array) arrays,
/// avoiding the gdext 0.4.x issue where try_to::<VarArray>() fails
/// on typed arrays (godot-rust/gdext#727).
fn variant_to_string_vec(variant: &Variant) -> Option<Vec<String>> {
    use godot::builtin::VariantType;
    if variant.get_type() != VariantType::ARRAY {
        return None;
    }
    let size = variant.call("size", &[]).try_to::<i64>().unwrap_or(0);
    let mut result = Vec::with_capacity(size as usize);
    for i in 0..size {
        let elem = variant.call("get", &[Variant::from(i)]);
        result.push(elem.to_string());
    }
    Some(result)
}

fn variant_type_name(type_id: i32) -> &'static str {
    match type_id {
        0 => "Variant",
        1 => "bool",
        2 => "int",
        3 => "float",
        4 => "String",
        5 => "Vector2",
        6 => "Vector2i",
        9 => "Vector3",
        10 => "Vector3i",
        12 => "Vector4",
        13 => "Vector4i",
        _ => "Variant",
    }
}
