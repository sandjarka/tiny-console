/// ASCII art rendering using Unicode block elements.
/// Each character maps to a 2-line boxed art representation.
use std::collections::HashMap;
use std::sync::OnceLock;

fn boxed_map() -> &'static HashMap<char, [&'static str; 2]> {
    static MAP: OnceLock<HashMap<char, [&'static str; 2]>> = OnceLock::new();
    MAP.get_or_init(|| {
        let mut m = HashMap::new();
        m.insert('a', ["▒▄▀█", "░█▀█"]);
        m.insert('b', ["░█▄▄", "▒█▄█"]);
        m.insert('c', ["░▄▀▀", "░▀▄▄"]);
        m.insert('d', ["▒█▀▄", "░█▄▀"]);
        m.insert('e', ["░██▀", "▒█▄▄"]);
        m.insert('f', ["░█▀▀", "░█▀░"]);
        m.insert('g', ["▒▄▀▀", "░▀▄█"]);
        m.insert('h', ["░█▄█", "▒█▒█"]);
        m.insert('i', ["░█", "░█"]);
        m.insert('j', ["░░▒█", "░▀▄█"]);
        m.insert('k', ["░█▄▀", "░█▒█"]);
        m.insert('l', ["░█▒░", "▒█▄▄"]);
        m.insert('m', ["▒█▀▄▀█", "░█▒▀▒█"]);
        m.insert('n', ["░█▄░█", "░█▒▀█"]);
        m.insert('o', ["░█▀█", "▒█▄█"]);
        m.insert('p', ["▒█▀█", "░█▀▀"]);
        m.insert('q', ["░▄▀▄", "░▀▄█"]);
        m.insert('r', ["▒█▀█", "░█▀▄"]);
        m.insert('s', ["░▄▀", "▒▄█"]);
        m.insert('t', ["░▀█▀", "░▒█▒"]);
        m.insert('u', ["░█░█", "▒█▄█"]);
        m.insert('v', ["░█░█", "▒▀▄▀"]);
        m.insert('w', ["▒█░█░█", "░▀▄▀▄▀"]);
        m.insert('x', ["░▀▄▀", "░█▒█"]);
        m.insert('y', ["░▀▄▀", "░▒█▒"]);
        m.insert('z', ["░▀█", "▒█▄"]);
        m.insert(' ', ["░", "░"]);
        m.insert('_', ["░░░", "▒▄▄"]);
        m.insert(',', ["░▒", "░█"]);
        m.insert('.', ["░░", "░▄"]);
        m.insert('!', ["░█", "░▄"]);
        m.insert('-', ["░▒░", "░▀▀"]);
        m.insert('?', ["░▀▀▄", "░▒█▀"]);
        m.insert('\'', ["░▀", "░░"]);
        m.insert(':', ["░▄░", "▒▄▒"]);
        m.insert('0', ["░▄▀▄", "░▀▄▀"]);
        m.insert('1', ["░▄█", "░░█"]);
        m.insert('2', ["░▀█", "░█▄"]);
        m.insert('3', ["░▀██", "░▄▄█"]);
        m.insert('4', ["░█▄", "░░█"]);
        m.insert('5', ["░█▀", "░▄█"]);
        m.insert('6', ["░█▀", "░██"]);
        m.insert('7', ["░▀█", "░█░"]);
        m.insert('8', ["░█▄█", "░█▄█"]);
        m.insert('9', ["░██", "░▄█"]);
        m
    })
}

const UNSUPPORTED_CHAR: [&str; 2] = ["░▒░", "▒░▒"];

/// Converts a string to 2-line boxed ASCII art.
pub fn str_to_boxed_art(text: &str) -> Vec<String> {
    let map = boxed_map();
    let mut lines = vec![String::new(), String::new()];
    for c in text.to_lowercase().chars() {
        let art = map.get(&c).unwrap_or(&UNSUPPORTED_CHAR);
        lines[0].push_str(art[0]);
        lines[1].push_str(art[1]);
    }
    lines
}

/// Returns true if all characters in the text are supported for boxed art.
pub fn is_boxed_art_supported(text: &str) -> bool {
    let map = boxed_map();
    text.to_lowercase().chars().all(|c| map.contains_key(&c))
}
