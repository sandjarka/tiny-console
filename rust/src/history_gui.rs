/// HistoryGui: Fuzzy search UI for command history (fzf-like).
/// Shows matching history entries as a scrollable list of labels.
use godot::classes::control::{LayoutPreset, SizeFlags};
use godot::classes::{
    IPanel, InputEvent, InputEventKey, InputEventMouseButton, Label, Panel, StyleBoxFlat,
    VScrollBar,
};
use godot::global::{Key, MouseButton};
use godot::prelude::*;

#[derive(GodotClass)]
#[class(base=Panel)]
pub struct HistoryGui {
    base: Base<Panel>,

    history_labels: Vec<Gd<Label>>,
    scroll_bar: Option<Gd<VScrollBar>>,
    scroll_bar_width: i32,
    last_highlighted_label: Option<Gd<Label>>,

    command: String,
    filter_results: Vec<String>,
    display_count: usize,
    offset: usize,
    sub_index: usize,

    highlight_color: Color,
}

#[godot_api]
impl HistoryGui {
    #[signal]
    fn dummy_signal();

    /// Set the search results externally (called by TinyConsole).
    #[func]
    pub fn set_search_results(&mut self, results: PackedStringArray) {
        self.filter_results.clear();
        for s in results.as_slice() {
            self.filter_results.push(s.to_string());
        }
        self.reset_indexes();
        self.update_highlight();
        self.update_scroll_list();
    }

    /// Search for the command in the history.
    #[func]
    pub fn search(&mut self, command: GString) {
        let cmd = command.to_string();
        if cmd == self.command {
            return;
        }
        self.command = cmd;
    }

    /// Get the currently selected text.
    #[func]
    pub fn get_current_text(&self) -> GString {
        if !self.history_labels.is_empty() && !self.filter_results.is_empty() {
            let idx = self.get_current_index();
            if idx < self.filter_results.len() {
                return GString::from(self.filter_results[idx].as_str());
            }
        }
        GString::from(self.command.as_str())
    }

    /// Set visibility of the history search panel.
    #[func]
    pub fn set_gui_visibility(&mut self, visible: bool) {
        self.base_mut().set_visible(visible);
    }

    /// Get the current search query.
    #[func]
    pub fn get_command(&self) -> GString {
        GString::from(self.command.as_str())
    }

    /// Set the command string (for external sync).
    #[func]
    pub fn set_command(&mut self, command: GString) {
        self.command = command.to_string();
    }

    #[func]
    fn on_visibility_changed(&mut self) {
        self.calculate_display_count();
    }

    pub fn increment_index(&mut self) {
        let current_index = self.get_current_index();
        if current_index + 1 >= self.filter_results.len() {
            return;
        }
        if self.sub_index >= self.display_count.saturating_sub(1) {
            self.offset += 1;
            self.update_scroll_list();
        } else {
            self.sub_index += 1;
            self.update_highlight();
        }
    }

    pub fn decrement_index(&mut self) {
        let current_index = self.get_current_index();
        if current_index == 0 {
            return;
        }
        if self.sub_index == 0 {
            self.offset = self.offset.saturating_sub(1);
            self.update_scroll_list();
        } else {
            self.sub_index -= 1;
            self.update_highlight();
        }
    }
}

// Private methods
impl HistoryGui {
    fn get_current_index(&self) -> usize {
        self.offset + self.sub_index
    }

    fn reset_indexes(&mut self) {
        self.offset = 0;
        self.sub_index = 0;
    }

    fn update_scroll_list(&mut self) {
        for i in 0..self.display_count {
            if i >= self.history_labels.len() {
                break;
            }
            let filter_index = self.offset + i;
            if filter_index < self.filter_results.len() {
                self.history_labels[i]
                    .set_text(&GString::from(self.filter_results[filter_index].as_str()));
            } else {
                self.history_labels[i].set_text(&GString::new());
            }
        }
        self.update_scroll_bar();
        self.update_highlight();
    }

    fn update_highlight(&mut self) {
        if self.filter_results.is_empty() {
            return;
        }

        let mut style = StyleBoxFlat::new_gd();
        style.set_bg_color(self.highlight_color);

        // Clear previous highlight
        if let Some(ref mut last) = self.last_highlighted_label {
            if last.is_instance_valid() {
                last.remove_theme_stylebox_override("normal");
            }
        }

        if self.sub_index < self.history_labels.len() && !self.filter_results.is_empty() {
            self.history_labels[self.sub_index].add_theme_stylebox_override("normal", &style);
            self.last_highlighted_label = Some(self.history_labels[self.sub_index].clone());
        }
    }

    fn update_scroll_bar(&mut self) {
        if self.display_count > 0 {
            if let Some(ref mut scroll_bar) = self.scroll_bar {
                let max_size = self.filter_results.len() as f64;
                scroll_bar.set_max(max_size);
                scroll_bar.set_page(self.display_count as f64);
                let val = (max_size - self.display_count as f64) - self.offset as f64;
                scroll_bar.set_value_no_signal(val);
            }
        }
    }

    fn calculate_display_count(&mut self) {
        if !self.base().is_visible() {
            return;
        }

        let panel_size = self.base().get_size();
        let max_y = panel_size.y;

        if self.history_labels.is_empty() {
            return;
        }

        let label_size_y = self.history_labels[0].get_size().y;
        if label_size_y <= 0.0 {
            return;
        }
        let label_size_x = panel_size.x - self.scroll_bar_width as f32;

        let new_display_count = (max_y / label_size_y) as usize;
        if new_display_count == 0 || new_display_count <= self.display_count {
            self.reposition_labels(label_size_x, label_size_y, panel_size.y);
            return;
        }

        self.display_count = new_display_count;

        // Reposition first label
        self.history_labels[0].set_position(Vector2::new(0.0, panel_size.y - label_size_y));
        self.history_labels[0].set_size(Vector2::new(label_size_x, label_size_y));

        // Create additional labels as needed
        let labels_needed = self.display_count.saturating_sub(self.history_labels.len());
        for _ in 0..labels_needed {
            let mut new_label = Label::new_alloc();
            new_label.set_v_size_flags(SizeFlags::SHRINK_END);
            new_label.set_h_size_flags(SizeFlags::EXPAND_FILL);

            let position_offset = (self.history_labels.len() + 1) as f32;
            new_label.set_position(Vector2::new(
                0.0,
                panel_size.y - position_offset * label_size_y,
            ));
            new_label.set_size(Vector2::new(label_size_x, label_size_y));

            self.base_mut().add_child(&new_label);
            self.history_labels.push(new_label);
        }

        self.reposition_labels(label_size_x, label_size_y, panel_size.y);

        // Update scroll bar position
        if let Some(ref mut scroll_bar) = self.scroll_bar {
            scroll_bar.set_size(Vector2::new(self.scroll_bar_width as f32, panel_size.y));
            scroll_bar.set_position(Vector2::new(label_size_x, 0.0));
        }

        self.reset_indexes();
        self.update_highlight();
        self.update_scroll_list();
    }

    fn reposition_labels(&mut self, label_size_x: f32, label_size_y: f32, panel_height: f32) {
        for (i, label) in self.history_labels.iter_mut().enumerate() {
            let position_offset = (i + 1) as f32;
            label.set_position(Vector2::new(
                0.0,
                panel_height - position_offset * label_size_y,
            ));
            label.set_size(Vector2::new(label_size_x, label_size_y));
        }
    }
}

#[godot_api]
impl IPanel for HistoryGui {
    fn init(base: Base<Panel>) -> Self {
        Self {
            base,
            history_labels: Vec::new(),
            scroll_bar: None,
            scroll_bar_width: 12,
            last_highlighted_label: None,
            command: "<placeholder>".to_string(),
            filter_results: Vec::new(),
            display_count: 0,
            offset: 0,
            sub_index: 0,
            highlight_color: Color::from_rgba(0.3, 0.3, 0.4, 0.6),
        }
    }

    fn ready(&mut self) {
        // Set anchors for full rect
        self.base_mut().set_anchors_preset(LayoutPreset::FULL_RECT);
        self.base_mut().set_h_size_flags(SizeFlags::EXPAND_FILL);
        self.base_mut().set_v_size_flags(SizeFlags::EXPAND_FILL);

        // Create first label
        let mut first_label = Label::new_alloc();
        first_label.set_v_size_flags(SizeFlags::SHRINK_END);
        first_label.set_h_size_flags(SizeFlags::EXPAND_FILL);
        first_label.set_text("<Placeholder>");
        self.base_mut().add_child(&first_label);
        self.history_labels.push(first_label);

        // Create scroll bar
        let scroll_bar = VScrollBar::new_alloc();
        self.base_mut().add_child(&scroll_bar);
        self.scroll_bar = Some(scroll_bar);

        // Try to load highlight color from theme
        if self
            .base()
            .has_theme_color_ex("history_highlight_color")
            .theme_type("ConsoleColors")
            .done()
        {
            self.highlight_color = self
                .base()
                .get_theme_color_ex("history_highlight_color")
                .theme_type("ConsoleColors")
                .done();
        }

        // Connect visibility_changed to calculate_display_count
        let this = self.to_gd();
        self.base_mut().connect(
            "visibility_changed",
            &Callable::from_object_method(&this, "on_visibility_changed"),
        );
    }

    fn input(&mut self, event: Gd<InputEvent>) {
        if !self.base().is_visible_in_tree() {
            return;
        }

        // Mouse wheel scrolling
        if let Ok(mouse_event) = event.clone().try_cast::<InputEventMouseButton>() {
            if mouse_event.get_button_index() == MouseButton::WHEEL_UP {
                self.increment_index();
            } else if mouse_event.get_button_index() == MouseButton::WHEEL_DOWN {
                self.decrement_index();
            }
        }

        // Keyboard navigation
        if let Ok(key_event) = event.try_cast::<InputEventKey>() {
            if !key_event.is_pressed() {
                return;
            }
            let keycode = key_event.get_keycode();
            if keycode == Key::UP {
                self.increment_index();
                self.base_mut()
                    .get_viewport()
                    .unwrap()
                    .set_input_as_handled();
            } else if keycode == Key::DOWN {
                self.decrement_index();
                self.base_mut()
                    .get_viewport()
                    .unwrap()
                    .set_input_as_handled();
            }
        }
    }
}
