/// CommandEntry: Custom TextEdit for console command input.
/// Handles special keys (ENTER, TAB, arrows, etc.) before TextEdit processes them,
/// emitting signals so TinyConsole can respond. Draws autocomplete hint text after the cursor.
use godot::classes::notify::ControlNotification;
use godot::classes::{Font, ITextEdit, InputEvent, InputEventKey, InputMap, StyleBox, TextEdit};
use godot::global::Key;
use godot::prelude::*;

use crate::command_entry_highlighter::CommandEntryHighlighter;

#[derive(GodotClass)]
#[class(base=TextEdit)]
pub struct CommandEntry {
    base: Base<TextEdit>,

    pub autocomplete_hint: GString,

    font: Option<Gd<Font>>,
    font_size: i32,
    hint_color: Color,
    sb_normal: Option<Gd<StyleBox>>,
}

#[godot_api]
impl CommandEntry {
    #[signal]
    fn text_submitted(command_line: GString);

    #[signal]
    fn autocomplete_requested();

    #[signal]
    fn reverse_autocomplete_requested();

    #[signal]
    fn history_up_requested();

    #[signal]
    fn history_down_requested();

    #[signal]
    fn scroll_up_requested();

    #[signal]
    fn scroll_down_requested();

    #[func]
    pub fn submit_text(&mut self) {
        let text = self.base().get_text();
        self.base_mut()
            .emit_signal("text_submitted", &[text.to_variant()]);
    }

    #[func]
    pub fn set_autocomplete_hint_value(&mut self, hint: GString) {
        if self.autocomplete_hint != hint {
            self.autocomplete_hint = hint;
            self.base_mut().queue_redraw();
        }
    }

    #[func]
    pub fn get_autocomplete_hint_value(&self) -> GString {
        self.autocomplete_hint.clone()
    }
}

#[godot_api]
impl ITextEdit for CommandEntry {
    fn init(base: Base<TextEdit>) -> Self {
        Self {
            base,
            autocomplete_hint: GString::new(),
            font: None,
            font_size: 0,
            hint_color: Color::from_rgba(0.5, 0.5, 0.5, 1.0),
            sb_normal: None,
        }
    }

    fn ready(&mut self) {
        // Configure TextEdit
        self.base_mut().set_multiple_carets_enabled(false);
        self.base_mut()
            .set_autowrap_mode(godot::classes::text_server::AutowrapMode::OFF);
        self.base_mut().set_fit_content_height_enabled(true);

        // Hide scrollbars
        {
            let mut vscroll = self.base().get_v_scroll_bar().unwrap();
            vscroll.set_visible(false);
        }
        {
            let mut hscroll = self.base().get_h_scroll_bar().unwrap();
            hscroll.set_visible(false);
        }

        // Cache theme properties
        let font = self.base().get_theme_font_ex("font").done();
        let font_size = self.base().get_theme_font_size_ex("font_size").done();
        if let Some(f) = font {
            self.font = Some(f);
        }
        self.font_size = font_size;

        if self.base().has_theme_color_ex("hint_color").done() {
            self.hint_color = self.base().get_theme_color_ex("hint_color").done();
        }

        let sb = self.base().get_theme_stylebox_ex("normal").done();
        if let Some(s) = sb {
            self.sb_normal = Some(s);
        }

        // Set syntax highlighter
        let highlighter = CommandEntryHighlighter::new_gd();
        self.base_mut().set_syntax_highlighter(&highlighter);
    }

    fn input(&mut self, event: Gd<InputEvent>) {
        if !self.base().has_focus() {
            return;
        }

        // Consume the console toggle key so it doesn't get typed into the input
        let input_map = InputMap::singleton();
        if input_map.has_action("tiny_console_toggle") && event.is_action("tiny_console_toggle") {
            self.base_mut()
                .get_viewport()
                .unwrap()
                .set_input_as_handled();
            return;
        }

        if let Ok(key_event) = event.try_cast::<InputEventKey>() {
            let keycode = key_event.get_keycode();
            let pressed = key_event.is_pressed();

            if keycode == Key::ENTER || keycode == Key::KP_ENTER {
                if pressed {
                    self.submit_text();
                }
                self.base_mut()
                    .get_viewport()
                    .unwrap()
                    .set_input_as_handled();
            } else if keycode == Key::C
                && key_event.is_ctrl_pressed()
                && self.base_mut().get_selected_text().is_empty()
            {
                // Clear input on CTRL+C when no text selected
                if pressed {
                    self.base_mut().set_text("");
                    self.base_mut().emit_signal("text_changed", &[]);
                }
                self.base_mut()
                    .get_viewport()
                    .unwrap()
                    .set_input_as_handled();
            } else if keycode == Key::TAB && key_event.is_shift_pressed() {
                if pressed {
                    self.base_mut()
                        .emit_signal("reverse_autocomplete_requested", &[]);
                }
                self.base_mut()
                    .get_viewport()
                    .unwrap()
                    .set_input_as_handled();
            } else if keycode == Key::TAB {
                if pressed {
                    self.base_mut().emit_signal("autocomplete_requested", &[]);
                }
                self.base_mut()
                    .get_viewport()
                    .unwrap()
                    .set_input_as_handled();
            } else if (keycode == Key::RIGHT || keycode == Key::END)
                && self.base().get_caret_column() == self.base().get_text().len() as i32
            {
                if pressed && !self.autocomplete_hint.is_empty() {
                    self.base_mut().emit_signal("autocomplete_requested", &[]);
                }
                self.base_mut()
                    .get_viewport()
                    .unwrap()
                    .set_input_as_handled();
            } else if keycode == Key::UP {
                if pressed {
                    self.base_mut().emit_signal("history_up_requested", &[]);
                }
                self.base_mut()
                    .get_viewport()
                    .unwrap()
                    .set_input_as_handled();
            } else if keycode == Key::DOWN {
                if pressed {
                    self.base_mut().emit_signal("history_down_requested", &[]);
                }
                self.base_mut()
                    .get_viewport()
                    .unwrap()
                    .set_input_as_handled();
            } else if keycode == Key::PAGEUP {
                if pressed {
                    self.base_mut().emit_signal("scroll_up_requested", &[]);
                }
                self.base_mut()
                    .get_viewport()
                    .unwrap()
                    .set_input_as_handled();
            } else if keycode == Key::PAGEDOWN {
                if pressed {
                    self.base_mut().emit_signal("scroll_down_requested", &[]);
                }
                self.base_mut()
                    .get_viewport()
                    .unwrap()
                    .set_input_as_handled();
            }
        }
    }

    fn draw(&mut self) {
        if self.autocomplete_hint.is_empty() {
            return;
        }

        let font = match &self.font {
            Some(f) => f.clone(),
            None => return,
        };

        let sb_offset = match &self.sb_normal {
            Some(sb) => sb.get_offset(),
            None => Vector2::ZERO,
        };

        let offset_x = (sb_offset.x * 0.5) + self.base().get_line_width(0) as f32;
        let offset_y = (sb_offset.y * 0.5) + self.base().get_line_height() as f32 + 0.5
            - font.get_descent() as f32;

        // Copy values to avoid borrow conflict with base_mut()
        let hint = self.autocomplete_hint.clone();
        let font_size = self.font_size;
        let hint_color = self.hint_color;

        self.base_mut()
            .draw_string_ex(&font, Vector2::new(offset_x, offset_y), &hint)
            .font_size(font_size)
            .modulate(hint_color)
            .done();
    }

    fn on_notification(&mut self, what: ControlNotification) {
        match what {
            ControlNotification::FOCUS_ENTER => self.base_mut().set_process_input(true),
            ControlNotification::FOCUS_EXIT => self.base_mut().set_process_input(false),
            _ => {}
        }
    }
}
