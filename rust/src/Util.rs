/// Utility functions: BBCode processing, fuzzy matching, validation.

/// Escapes BBCode brackets in text for safe display in RichTextLabel.
pub fn bbcode_escape(text: &str) -> String {
    text.replace('[', "~LB~")
        .replace(']', "~RB~")
        .replace("~LB~", "[lb]")
        .replace("~RB~", "[rb]")
}

/// Strips all BBCode tags from text, returning plain text.
pub fn bbcode_strip(text: &str) -> String {
    let mut result = String::with_capacity(text.len());
    let mut in_brackets = false;
    for c in text.chars() {
        match c {
            '[' => in_brackets = true,
            ']' => in_brackets = false,
            _ if !in_brackets => result.push(c),
            _ => {}
        }
    }
    result
}

/// Finds the most similar string in a slice, within the given edit distance.
/// Returns `None` if no match is close enough.
pub fn fuzzy_match_string(
    needle: &str,
    max_edit_distance: usize,
    haystack: &[String],
) -> Option<String> {
    if haystack.is_empty() {
        return None;
    }
    let mut best_distance = usize::MAX;
    let mut best_match = String::new();
    for elem in haystack {
        let dist = calculate_osa_distance(needle, elem);
        if dist < best_distance {
            best_distance = dist;
            best_match = elem.clone();
        }
    }
    if best_distance <= max_edit_distance {
        Some(best_match)
    } else {
        None
    }
}

/// Calculates the Optimal String Alignment distance between two strings.
/// See: https://en.wikipedia.org/wiki/Levenshtein_distance
fn calculate_osa_distance(s1: &str, s2: &str) -> usize {
    let s1_chars: Vec<char> = s1.chars().collect();
    let s2_chars: Vec<char> = s2.chars().collect();
    let s1_len = s1_chars.len();
    let s2_len = s2_chars.len();

    // Iterative approach with 3 matrix rows.
    let mut row0 = vec![0usize; s2_len + 1]; // previous-previous
    let mut row1 = vec![0usize; s2_len + 1]; // previous
    let mut row2 = vec![0usize; s2_len + 1]; // current

    for i in 0..=s2_len {
        row1[i] = i;
    }

    for i in 0..s1_len {
        row2[0] = i + 1;

        for j in 0..s2_len {
            let deletion_cost = row1[j + 1] + 1;
            let insertion_cost = row2[j] + 1;
            let substitution_cost = if s1_chars[i] == s2_chars[j] {
                row1[j]
            } else {
                row1[j] + 1
            };

            row2[j + 1] = deletion_cost.min(insertion_cost).min(substitution_cost);

            if i > 0 && j > 0 && s1_chars[i] == s2_chars[j - 1] && s1_chars[i - 1] == s2_chars[j] {
                let transposition_cost = row0[j - 1] + 1;
                row2[j + 1] = row2[j + 1].min(transposition_cost);
            }
        }

        // Swap rows
        let tmp = std::mem::replace(&mut row0, std::mem::take(&mut row1));
        row1 = std::mem::replace(&mut row2, tmp);
    }
    row1[s2_len]
}

/// Returns true if the string is a valid command sequence:
/// one or more space-separated identifiers (letters, digits, underscores; first char not digit).
pub fn is_valid_command_sequence(s: &str) -> bool {
    if s.is_empty() {
        return false;
    }
    s.split(' ').all(is_valid_ascii_identifier)
}

fn is_valid_ascii_identifier(s: &str) -> bool {
    !s.is_empty()
        && !s.starts_with(|c: char| c.is_ascii_digit())
        && s.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
}
