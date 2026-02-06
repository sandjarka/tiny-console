mod ascii_art;
mod builtin_commands;
mod command_entry;
mod command_entry_highlighter;
mod command_history;
mod console_options;
mod history_gui;
mod tiny_console;
mod util;

use godot::init::InitStage;
use godot::prelude::*;

use crate::tiny_console::TinyConsole;

struct SGConsole;

/// #[class(singleton)] on TinyConsole handles register/unregister/free automatically.
/// We only need to defer initialize() to MainLoop stage when the scene tree is ready.
#[gdextension]
unsafe impl ExtensionLibrary for SGConsole {
    fn on_stage_init(stage: InitStage) {
        if stage == InitStage::MainLoop {
            // Call initialize_impl directly with the Gd handle.
            // This avoids going through #[func] dispatch which would hold
            // a borrow that conflicts with internal bind_mut/drop cycles.
            let mut singleton = TinyConsole::singleton();
            TinyConsole::initialize_impl(&mut singleton);
        }
    }

    fn on_stage_deinit(stage: InitStage) {
        if stage == InitStage::MainLoop {
            let mut singleton = TinyConsole::singleton();
            if singleton.bind().is_initialized() {
                singleton.bind_mut().cleanup();
            }
        }
    }
}
