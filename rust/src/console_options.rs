/// Console configuration via Godot's ProjectSettings.
/// All settings are registered under "addons/tiny_console/" and are editable
/// in the Godot editor via Project > Project Settings.
use godot::classes::ProjectSettings;
use godot::global::PropertyHint;
use godot::prelude::*;
use std::collections::HashMap;

const S: &str = "addons/tiny_console";

pub struct ConsoleOptions {
    // main
    pub aliases: HashMap<String, String>,
    pub disable_in_release_build: bool,
    pub enable_in_editor: bool,
    pub print_to_stdout: bool,
    pub pause_when_open: bool,
    pub commands_disabled_in_release: Vec<String>,

    // appearance
    pub custom_theme: String,
    pub height_ratio: f32,
    pub open_speed: f32,
    pub opacity: f32,
    pub sparse_mode: bool,

    // greet
    pub greet_user: bool,
    pub greeting_message: String,
    pub greet_using_ascii_art: bool,

    // history
    pub persist_history: bool,
    pub history_lines: i32,

    // autocomplete
    pub autocomplete_use_history_with_matches: bool,

    // autoexec
    pub autoexec_script: String,
    pub autoexec_auto_create: bool,
}

impl Default for ConsoleOptions {
    fn default() -> Self {
        let mut aliases = HashMap::new();
        aliases.insert("exit".into(), "quit".into());
        aliases.insert("source".into(), "exec".into());
        aliases.insert("usage".into(), "help".into());

        Self {
            aliases,
            disable_in_release_build: false,
            enable_in_editor: false,
            print_to_stdout: false,
            pause_when_open: true,
            commands_disabled_in_release: vec!["eval".into()],

            custom_theme: "res://addons/tiny_console/res/default_theme.tres".into(),
            height_ratio: 0.5,
            open_speed: 5.0,
            opacity: 1.0,
            sparse_mode: false,

            greet_user: true,
            greeting_message: "Tiny Console".into(),
            greet_using_ascii_art: true,

            persist_history: true,
            history_lines: 1000,

            autocomplete_use_history_with_matches: true,

            autoexec_script: "user://autoexec.lcs".into(),
            autoexec_auto_create: true,
        }
    }
}

impl ConsoleOptions {
    pub fn new() -> Self {
        Self::default()
    }

    /// Registers all console settings in Godot's ProjectSettings.
    /// Settings that already exist (user-configured) are not overwritten.
    /// Property hints are added so the editor shows appropriate UI widgets.
    pub fn register_project_settings(&self) {
        let mut ps = ProjectSettings::singleton();

        // -- main --
        define_bool(
            &mut ps,
            &key("disable_in_release_build"),
            self.disable_in_release_build,
        );
        define_bool(&mut ps, &key("enable_in_editor"), self.enable_in_editor);
        define_bool(&mut ps, &key("print_to_stdout"), self.print_to_stdout);
        define_bool(&mut ps, &key("pause_when_open"), self.pause_when_open);

        // aliases (Dictionary)
        {
            let k = key("aliases");
            let mut dict = VarDictionary::new();
            for (alias, target) in &self.aliases {
                dict.set(
                    GString::from(alias.as_str()).to_variant(),
                    GString::from(target.as_str()).to_variant(),
                );
            }
            let default_val = dict.to_variant();
            if !ps.has_setting(&k) {
                ps.set_setting(&k, &default_val);
            }
            ps.set_initial_value(&k, &default_val);
            add_property_info(&mut ps, &k, VariantType::DICTIONARY, PropertyHint::NONE, "");
        }

        // commands_disabled_in_release (PackedStringArray)
        {
            let k = key("commands_disabled_in_release");
            let arr: PackedStringArray = self
                .commands_disabled_in_release
                .iter()
                .map(|s| GString::from(s.as_str()))
                .collect();
            let default_val = arr.to_variant();
            if !ps.has_setting(&k) {
                ps.set_setting(&k, &default_val);
            }
            ps.set_initial_value(&k, &default_val);
            add_property_info(
                &mut ps,
                &k,
                VariantType::PACKED_STRING_ARRAY,
                PropertyHint::NONE,
                "",
            );
        }

        // -- appearance --
        define_string(
            &mut ps,
            &key("appearance/custom_theme"),
            &self.custom_theme,
            PropertyHint::FILE,
            "*.tres",
        );
        define_float(
            &mut ps,
            &key("appearance/height_ratio"),
            self.height_ratio,
            PropertyHint::RANGE,
            "0.1,1.0,0.05",
        );
        define_float(
            &mut ps,
            &key("appearance/open_speed"),
            self.open_speed,
            PropertyHint::RANGE,
            "0.1,20.0,0.1",
        );
        define_float(
            &mut ps,
            &key("appearance/opacity"),
            self.opacity,
            PropertyHint::RANGE,
            "0.0,1.0,0.05",
        );
        define_bool(&mut ps, &key("appearance/sparse_mode"), self.sparse_mode);

        // -- greet --
        define_bool(&mut ps, &key("greet/greet_user"), self.greet_user);
        define_string(
            &mut ps,
            &key("greet/greeting_message"),
            &self.greeting_message,
            PropertyHint::NONE,
            "",
        );
        define_bool(
            &mut ps,
            &key("greet/greet_using_ascii_art"),
            self.greet_using_ascii_art,
        );

        // -- history --
        define_bool(
            &mut ps,
            &key("history/persist_history"),
            self.persist_history,
        );
        define_int(
            &mut ps,
            &key("history/history_lines"),
            self.history_lines,
            PropertyHint::RANGE,
            "10,10000,10",
        );

        // -- autocomplete --
        define_bool(
            &mut ps,
            &key("autocomplete/use_history_with_matches"),
            self.autocomplete_use_history_with_matches,
        );

        // -- autoexec --
        define_string(
            &mut ps,
            &key("autoexec/script"),
            &self.autoexec_script,
            PropertyHint::NONE,
            "",
        );
        define_bool(
            &mut ps,
            &key("autoexec/auto_create"),
            self.autoexec_auto_create,
        );
    }

    /// Reads all settings from ProjectSettings into this struct.
    pub fn load_from_project_settings(&mut self) {
        let ps = ProjectSettings::singleton();

        // -- main --
        self.disable_in_release_build = get_bool(&ps, &key("disable_in_release_build"));
        self.enable_in_editor = get_bool(&ps, &key("enable_in_editor"));
        self.print_to_stdout = get_bool(&ps, &key("print_to_stdout"));
        self.pause_when_open = get_bool(&ps, &key("pause_when_open"));

        // aliases
        {
            let val = ps.get_setting(&key("aliases"));
            if let Ok(dict) = val.try_to::<VarDictionary>() {
                self.aliases.clear();
                for k in dict.keys_array().iter_shared() {
                    if let (Ok(ks), Some(v)) = (k.try_to::<GString>(), dict.get(k.clone())) {
                        if let Ok(vs) = v.try_to::<GString>() {
                            self.aliases.insert(ks.to_string(), vs.to_string());
                        }
                    }
                }
            }
        }

        // commands_disabled_in_release
        {
            let val = ps.get_setting(&key("commands_disabled_in_release"));
            if let Ok(arr) = val.try_to::<PackedStringArray>() {
                self.commands_disabled_in_release.clear();
                for s in arr.as_slice() {
                    self.commands_disabled_in_release.push(s.to_string());
                }
            }
        }

        // -- appearance --
        self.custom_theme = get_string(&ps, &key("appearance/custom_theme"));
        self.height_ratio = get_float(&ps, &key("appearance/height_ratio"));
        self.open_speed = get_float(&ps, &key("appearance/open_speed"));
        self.opacity = get_float(&ps, &key("appearance/opacity"));
        self.sparse_mode = get_bool(&ps, &key("appearance/sparse_mode"));

        // -- greet --
        self.greet_user = get_bool(&ps, &key("greet/greet_user"));
        self.greeting_message = get_string(&ps, &key("greet/greeting_message"));
        self.greet_using_ascii_art = get_bool(&ps, &key("greet/greet_using_ascii_art"));

        // -- history --
        self.persist_history = get_bool(&ps, &key("history/persist_history"));
        self.history_lines = get_int(&ps, &key("history/history_lines"));

        // -- autocomplete --
        self.autocomplete_use_history_with_matches =
            get_bool(&ps, &key("autocomplete/use_history_with_matches"));

        // -- autoexec --
        self.autoexec_script = get_string(&ps, &key("autoexec/script"));
        self.autoexec_auto_create = get_bool(&ps, &key("autoexec/auto_create"));
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn key(suffix: &str) -> GString {
    GString::from(format!("{}/{}", S, suffix).as_str())
}

fn add_property_info(
    ps: &mut Gd<ProjectSettings>,
    name: &GString,
    variant_type: VariantType,
    hint: PropertyHint,
    hint_string: &str,
) {
    let mut info = VarDictionary::new();
    info.set("name", name.to_variant());
    info.set("type", (variant_type.ord() as i32).to_variant());
    info.set("hint", (hint.ord() as i32).to_variant());
    info.set("hint_string", GString::from(hint_string).to_variant());
    ps.add_property_info(&info);
}

fn define_bool(ps: &mut Gd<ProjectSettings>, name: &GString, default: bool) {
    let val = default.to_variant();
    if !ps.has_setting(name) {
        ps.set_setting(name, &val);
    }
    ps.set_initial_value(name, &val);
    add_property_info(ps, name, VariantType::BOOL, PropertyHint::NONE, "");
}

fn define_float(
    ps: &mut Gd<ProjectSettings>,
    name: &GString,
    default: f32,
    hint: PropertyHint,
    hint_string: &str,
) {
    let val = (default as f64).to_variant();
    if !ps.has_setting(name) {
        ps.set_setting(name, &val);
    }
    ps.set_initial_value(name, &val);
    add_property_info(ps, name, VariantType::FLOAT, hint, hint_string);
}

fn define_int(
    ps: &mut Gd<ProjectSettings>,
    name: &GString,
    default: i32,
    hint: PropertyHint,
    hint_string: &str,
) {
    let val = (default as i64).to_variant();
    if !ps.has_setting(name) {
        ps.set_setting(name, &val);
    }
    ps.set_initial_value(name, &val);
    add_property_info(ps, name, VariantType::INT, hint, hint_string);
}

fn define_string(
    ps: &mut Gd<ProjectSettings>,
    name: &GString,
    default: &str,
    hint: PropertyHint,
    hint_string: &str,
) {
    let val = GString::from(default).to_variant();
    if !ps.has_setting(name) {
        ps.set_setting(name, &val);
    }
    ps.set_initial_value(name, &val);
    add_property_info(ps, name, VariantType::STRING, hint, hint_string);
}

fn get_bool(ps: &Gd<ProjectSettings>, name: &GString) -> bool {
    ps.get_setting(name).to::<bool>()
}

fn get_float(ps: &Gd<ProjectSettings>, name: &GString) -> f32 {
    ps.get_setting(name).to::<f64>() as f32
}

fn get_int(ps: &Gd<ProjectSettings>, name: &GString) -> i32 {
    ps.get_setting(name).to::<i64>() as i32
}

fn get_string(ps: &Gd<ProjectSettings>, name: &GString) -> String {
    ps.get_setting(name).to::<GString>().to_string()
}
