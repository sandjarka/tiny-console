mod ascii_art;
mod builtin_commands;
mod command_entry;
mod command_entry_highlighter;
mod command_history;
mod console_options;
mod history_gui;
mod tiny_console;
mod util;

use godot::prelude::*;

use crate::tiny_console::TinyConsole;

struct SGConsole;

#[gdextension]
unsafe impl ExtensionLibrary for SGConsole {
    #[allow(deprecated)]
    fn on_level_init(level: InitLevel) {
        if level == InitLevel::Scene {
            // Defer initialization until the main loop is ready.
            // Using Callable::from_fn so we get a fresh Gd handle with no active borrows.
            let callable = Callable::from_fn(&TinyConsole::class_id().to_gstring(), |_args| {
                let mut singleton = TinyConsole::singleton();
                TinyConsole::initialize_impl(&mut singleton);
                Variant::nil()
            });
            callable.call_deferred(&[]);
        }
    }

    #[allow(deprecated)]
    fn on_level_deinit(level: InitLevel) {
        if level == InitLevel::Scene {
            let mut singleton = TinyConsole::singleton();
            if singleton.bind().is_initialized() {
                singleton.bind_mut().cleanup();
            }
        }
    }
}
