/// CommandEntryHighlighter: Syntax highlighter for the command entry.
/// Colors the command name green if recognized, red if not.
/// Subcommands get a distinct color.
use godot::classes::{ISyntaxHighlighter, SyntaxHighlighter};
use godot::prelude::*;

use crate::tiny_console::TinyConsole;

#[derive(GodotClass)]
#[class(base=SyntaxHighlighter)]
pub struct CommandEntryHighlighter {
    base: Base<SyntaxHighlighter>,

    pub command_found_color: Color,
    pub subcommand_color: Color,
    pub command_not_found_color: Color,
    pub text_color: Color,
}

#[godot_api]
impl CommandEntryHighlighter {
    #[func]
    pub fn set_command_found_color(&mut self, color: Color) {
        self.command_found_color = color;
    }

    #[func]
    pub fn set_command_not_found_color(&mut self, color: Color) {
        self.command_not_found_color = color;
    }

    #[func]
    pub fn set_subcommand_color(&mut self, color: Color) {
        self.subcommand_color = color;
    }

    #[func]
    pub fn set_text_color(&mut self, color: Color) {
        self.text_color = color;
    }
}

#[godot_api]
impl ISyntaxHighlighter for CommandEntryHighlighter {
    fn init(base: Base<SyntaxHighlighter>) -> Self {
        Self {
            base,
            command_found_color: Color::from_rgba(0.73, 0.90, 0.49, 1.0),
            subcommand_color: Color::from_rgba(0.58, 0.90, 0.80, 1.0),
            command_not_found_color: Color::from_rgba(1.0, 0.2, 0.2, 1.0),
            text_color: Color::from_rgba(0.80, 0.80, 0.78, 1.0),
        }
    }

    fn get_line_syntax_highlighting(&self, _line: i32) -> VarDictionary {
        let mut result = VarDictionary::new();

        let text_edit = match self.base().get_text_edit() {
            Some(te) => te,
            None => return result,
        };

        let text = text_edit.get_text().to_string();
        if text.is_empty() {
            return result;
        }

        // Try to find TinyConsole autoload
        let console = get_tiny_console(&text_edit);

        // Tokenize into argv with starting indices
        let mut argv: Vec<String> = Vec::new();
        let mut argi: Vec<usize> = Vec::new();
        let mut start = 0usize;
        let mut cur = 0usize;
        let text_with_space = format!("{} ", text);

        for ch in text_with_space.chars() {
            if ch == ' ' {
                if cur > start {
                    argv.push(text[start..cur].to_string());
                    argi.push(start);
                }
                start = cur + 1;
            }
            cur += ch.len_utf8();
        }

        if argv.is_empty() {
            return result;
        }

        // Check progressively longer command sequences
        let mut command_end_idx: Option<usize> = None;
        if let Some(console) = &console {
            let console_ref = console.bind();
            for i in 1..=argv.len() {
                let maybe_command = argv[..i].join(" ");
                if console_ref.has_command_str(&maybe_command)
                    || console_ref.has_alias_str(&maybe_command)
                {
                    let last_token_start = argi[i - 1];
                    command_end_idx = Some(last_token_start + argv[i - 1].len());
                }
            }
        }

        let command_color;
        let arg_start_idx;

        if let Some(end_idx) = command_end_idx {
            command_color = self.command_found_color;
            arg_start_idx = if end_idx < text.len() {
                end_idx + 1
            } else {
                text.len()
            };
        } else {
            command_color = self.command_not_found_color;
            arg_start_idx = if argi.len() > 1 { argi[1] } else { text.len() };
        }

        // Build result dictionary
        let mut color_dict = VarDictionary::new();
        color_dict.set("color", command_color.to_variant());
        result.set(0i32.to_variant(), color_dict.to_variant());

        // Subcommand coloring
        if command_end_idx.is_some() && argi.len() > 1 {
            let mut sub_dict = VarDictionary::new();
            sub_dict.set("color", self.subcommand_color.to_variant());
            result.set((argi[1] as i32).to_variant(), sub_dict.to_variant());
        }

        let mut text_dict = VarDictionary::new();
        text_dict.set("color", self.text_color.to_variant());
        result.set((arg_start_idx as i32).to_variant(), text_dict.to_variant());

        result
    }
}

/// Helper to find the TinyConsole autoload from a node.
fn get_tiny_console(_node: &Gd<godot::classes::TextEdit>) -> Option<Gd<TinyConsole>> {
    if godot::classes::Engine::singleton().has_singleton("TinyConsole") {
        Some(TinyConsole::singleton())
    } else {
        None
    }
}
