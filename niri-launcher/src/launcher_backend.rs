// Backend logic for niri-launcher.
// All business logic lives here — zero GTK4 imports.

use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use thiserror::Error;

// ── Error type ────────────────────────────────────────────────────────────────

#[derive(Debug, Error)]
pub enum LauncherError {
    #[error("failed to launch app '{name}': {source}")]
    LaunchFailed {
        name: String,
        #[source]
        source: std::io::Error,
    },
    #[error("empty exec command for app '{0}'")]
    EmptyExec(String),
}

// ── Core types ────────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct AppEntry {
    pub id: String,
    pub name: String,
    pub exec: String,
    pub icon: String,
    pub description: String,
    pub keywords: Vec<String>,
}

/// A fuzzy-search result with scoring metadata for highlight rendering.
#[derive(Debug, Clone)]
pub struct SearchResult {
    pub entry: AppEntry,
    pub score: f32,
    /// Byte ranges within `entry.name` that matched query characters.
    pub match_ranges: Vec<(usize, usize)>,
}

#[derive(Debug, Clone)]
pub struct CalcResult {
    pub expression: String,
    pub result: String,
    pub is_error: bool,
}

// ── App loading ───────────────────────────────────────────────────────────────

/// Scan XDG data directories for `.desktop` files and return parsed app entries.
pub fn load_apps() -> Vec<AppEntry> {
    let mut apps = Vec::new();
    for dir in xdg_data_dirs() {
        let apps_dir = dir.join("applications");
        let Ok(entries) = fs::read_dir(&apps_dir) else { continue };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) == Some("desktop") {
                if let Some(app) = parse_desktop_file(&path) {
                    apps.push(app);
                }
            }
        }
    }
    apps
}

fn xdg_data_dirs() -> Vec<PathBuf> {
    let mut dirs = Vec::new();

    if let Ok(xdg_home) = env::var("XDG_DATA_HOME") {
        dirs.push(PathBuf::from(xdg_home));
    } else if let Some(home) = env::var_os("HOME") {
        dirs.push(PathBuf::from(home).join(".local/share"));
    }

    let data_dirs = env::var("XDG_DATA_DIRS")
        .unwrap_or_else(|_| "/usr/local/share:/usr/share".to_owned());
    for dir in data_dirs.split(':').filter(|s| !s.is_empty()) {
        dirs.push(PathBuf::from(dir));
    }

    dirs
}

fn parse_desktop_file(path: &Path) -> Option<AppEntry> {
    let content = fs::read_to_string(path).ok()?;
    let mut in_entry = false;
    let mut entry_type = String::new();
    let mut name = String::new();
    let mut exec = String::new();
    let mut icon = String::new();
    let mut description = String::new();
    let mut keywords: Vec<String> = Vec::new();
    let mut no_display = false;
    let mut hidden = false;

    for line in content.lines() {
        let line = line.trim();

        if line == "[Desktop Entry]" {
            in_entry = true;
            continue;
        }
        if line.starts_with('[') {
            in_entry = false;
            continue;
        }
        if !in_entry || line.starts_with('#') {
            continue;
        }

        let Some((key, value)) = line.split_once('=') else { continue };
        let (key, value) = (key.trim(), value.trim());

        match key {
            "Type" => entry_type = value.to_owned(),
            "Name" if name.is_empty() => name = value.to_owned(),
            "Exec" => exec = value.to_owned(),
            "Icon" => icon = value.to_owned(),
            "Comment" if description.is_empty() => description = value.to_owned(),
            "Keywords" => {
                keywords = value
                    .split(';')
                    .filter(|s| !s.is_empty())
                    .map(str::to_owned)
                    .collect();
            }
            "NoDisplay" => no_display = value.eq_ignore_ascii_case("true"),
            "Hidden" => hidden = value.eq_ignore_ascii_case("true"),
            _ => {}
        }
    }

    if entry_type != "Application" || no_display || hidden || name.is_empty() || exec.is_empty() {
        return None;
    }

    let id = path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .to_owned();

    Some(AppEntry { id, name, exec, icon, description, keywords })
}

// ── Fuzzy search ──────────────────────────────────────────────────────────────

/// Case-insensitive fuzzy match over app entries. Returns results sorted by
/// score descending; non-matching entries are excluded.
pub fn fuzzy_search(query: &str, apps: &[AppEntry]) -> Vec<SearchResult> {
    if query.is_empty() {
        return apps
            .iter()
            .map(|e| SearchResult {
                entry: e.clone(),
                score: 0.0,
                match_ranges: Vec::new(),
            })
            .collect();
    }

    let query_lower = query.to_lowercase();
    let mut results: Vec<SearchResult> =
        apps.iter().filter_map(|app| score_app(&query_lower, app)).collect();

    results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
    results
}

fn score_app(query: &str, app: &AppEntry) -> Option<SearchResult> {
    let name_lower = app.name.to_lowercase();

    if let Some((score, ranges)) = fuzzy_match(query, &name_lower) {
        return Some(SearchResult {
            entry: app.clone(),
            score: score * 2.0,
            match_ranges: ranges,
        });
    }

    let exec_lower = app.exec.to_lowercase();
    let exec_cmd = exec_lower.split_whitespace().next().unwrap_or("");
    let exec_base = exec_cmd.rsplit('/').next().unwrap_or(exec_cmd);
    if let Some((score, ranges)) = fuzzy_match(query, exec_base) {
        return Some(SearchResult {
            entry: app.clone(),
            score: score * 1.5,
            match_ranges: ranges,
        });
    }

    let desc_lower = app.description.to_lowercase();
    if let Some((score, ranges)) = fuzzy_match(query, &desc_lower) {
        return Some(SearchResult { entry: app.clone(), score, match_ranges: ranges });
    }

    for kw in &app.keywords {
        let kw_lower = kw.to_lowercase();
        if let Some((score, ranges)) = fuzzy_match(query, &kw_lower) {
            return Some(SearchResult {
                entry: app.clone(),
                score: score * 1.2,
                match_ranges: ranges,
            });
        }
    }

    None
}

/// Returns `(score, match_ranges)` if all characters of `query` appear in
/// `target` in order (subsequence match). `match_ranges` are byte ranges.
fn fuzzy_match(query: &str, target: &str) -> Option<(f32, Vec<(usize, usize)>)> {
    if query.is_empty() {
        return Some((1.0, Vec::new()));
    }

    let query_chars: Vec<char> = query.chars().collect();
    let target_chars: Vec<char> = target.chars().collect();
    let mut qi = 0;
    let mut match_positions: Vec<usize> = Vec::new();

    for (ti, &tc) in target_chars.iter().enumerate() {
        if qi < query_chars.len() && tc == query_chars[qi] {
            match_positions.push(ti);
            qi += 1;
        }
    }

    if qi < query_chars.len() {
        return None;
    }

    let consecutive_bonus = match_positions
        .windows(2)
        .filter(|w| w[1] == w[0] + 1)
        .count() as f32
        * 0.3;
    let start_bonus = if match_positions.first() == Some(&0) { 0.5 } else { 0.0 };
    let coverage = query_chars.len() as f32 / target_chars.len() as f32;
    let score = coverage + consecutive_bonus + start_bonus;

    // Convert char positions to byte ranges, then merge consecutive spans.
    let char_byte_offsets: Vec<usize> = target
        .char_indices()
        .map(|(byte_pos, _)| byte_pos)
        .collect();
    let byte_len = target.len();

    let byte_positions: Vec<usize> = match_positions
        .iter()
        .map(|&ci| char_byte_offsets.get(ci).copied().unwrap_or(byte_len))
        .collect();

    let ranges = merge_positions_to_ranges(&byte_positions);
    Some((score, ranges))
}

fn merge_positions_to_ranges(positions: &[usize]) -> Vec<(usize, usize)> {
    if positions.is_empty() {
        return Vec::new();
    }

    let mut ranges = Vec::new();
    let mut start = positions[0];
    let mut end = positions[0];

    for &pos in &positions[1..] {
        if pos == end + 1 {
            end = pos;
        } else {
            ranges.push((start, end + 1));
            start = pos;
            end = pos;
        }
    }
    ranges.push((start, end + 1));
    ranges
}

// ── Calculator ────────────────────────────────────────────────────────────────

/// Evaluate a basic math expression (+ - * / ^ % and parentheses).
/// No external crates required.
pub fn evaluate_expression(expr: &str) -> CalcResult {
    let expression = expr.to_owned();
    match parse_and_eval(expr.trim()) {
        Ok(value) => CalcResult {
            expression,
            result: format_number(value),
            is_error: false,
        },
        Err(msg) => CalcResult { expression, result: msg, is_error: true },
    }
}

fn format_number(n: f64) -> String {
    if n.fract() == 0.0 && n.abs() < 1e15 {
        format!("{}", n as i64)
    } else {
        format!("{n}")
    }
}

struct Parser<'a> {
    input: &'a [u8],
    pos: usize,
}

impl<'a> Parser<'a> {
    fn new(input: &'a str) -> Self {
        Self { input: input.as_bytes(), pos: 0 }
    }

    fn peek(&self) -> Option<u8> {
        self.input.get(self.pos).copied()
    }

    fn consume(&mut self) -> Option<u8> {
        let ch = self.input.get(self.pos).copied();
        if ch.is_some() {
            self.pos += 1;
        }
        ch
    }

    fn skip_ws(&mut self) {
        while matches!(self.peek(), Some(b' ' | b'\t')) {
            self.pos += 1;
        }
    }

    fn parse_expr(&mut self) -> Result<f64, String> {
        self.parse_additive()
    }

    fn parse_additive(&mut self) -> Result<f64, String> {
        let mut left = self.parse_multiplicative()?;
        loop {
            self.skip_ws();
            match self.peek() {
                Some(b'+') => {
                    self.consume();
                    left += self.parse_multiplicative()?;
                }
                Some(b'-') => {
                    self.consume();
                    left -= self.parse_multiplicative()?;
                }
                _ => break,
            }
        }
        Ok(left)
    }

    fn parse_multiplicative(&mut self) -> Result<f64, String> {
        let mut left = self.parse_power()?;
        loop {
            self.skip_ws();
            match self.peek() {
                Some(b'*') => {
                    self.consume();
                    left *= self.parse_power()?;
                }
                Some(b'/') => {
                    self.consume();
                    let right = self.parse_power()?;
                    if right == 0.0 {
                        return Err("division by zero".to_owned());
                    }
                    left /= right;
                }
                Some(b'%') => {
                    self.consume();
                    let right = self.parse_power()?;
                    if right == 0.0 {
                        return Err("modulo by zero".to_owned());
                    }
                    left %= right;
                }
                _ => break,
            }
        }
        Ok(left)
    }

    fn parse_power(&mut self) -> Result<f64, String> {
        let base = self.parse_unary()?;
        self.skip_ws();
        if self.peek() == Some(b'^') {
            self.consume();
            let exp = self.parse_power()?; // right-associative
            Ok(base.powf(exp))
        } else {
            Ok(base)
        }
    }

    fn parse_unary(&mut self) -> Result<f64, String> {
        self.skip_ws();
        match self.peek() {
            Some(b'-') => {
                self.consume();
                Ok(-self.parse_primary()?)
            }
            Some(b'+') => {
                self.consume();
                self.parse_primary()
            }
            _ => self.parse_primary(),
        }
    }

    fn parse_primary(&mut self) -> Result<f64, String> {
        self.skip_ws();
        match self.peek() {
            Some(b'(') => {
                self.consume();
                let val = self.parse_expr()?;
                self.skip_ws();
                if self.consume() != Some(b')') {
                    return Err("expected ')'".to_owned());
                }
                Ok(val)
            }
            Some(b'0'..=b'9') | Some(b'.') => self.parse_number(),
            Some(ch) => Err(format!("unexpected character '{}'", ch as char)),
            None => Err("unexpected end of expression".to_owned()),
        }
    }

    fn parse_number(&mut self) -> Result<f64, String> {
        let start = self.pos;
        // Consume digits, optional dot, optional exponent.
        while matches!(self.peek(), Some(b'0'..=b'9') | Some(b'.')) {
            self.pos += 1;
        }
        if matches!(self.peek(), Some(b'e') | Some(b'E')) {
            self.pos += 1;
            if matches!(self.peek(), Some(b'+') | Some(b'-')) {
                self.pos += 1;
            }
            while matches!(self.peek(), Some(b'0'..=b'9')) {
                self.pos += 1;
            }
        }
        let s = std::str::from_utf8(&self.input[start..self.pos])
            .map_err(|_| "invalid number encoding".to_owned())?;
        s.parse::<f64>().map_err(|_| format!("invalid number '{s}'"))
    }
}

fn parse_and_eval(expr: &str) -> Result<f64, String> {
    if expr.is_empty() {
        return Err("empty expression".to_owned());
    }
    let mut parser = Parser::new(expr);
    let result = parser.parse_expr()?;
    parser.skip_ws();
    if parser.pos < parser.input.len() {
        return Err(format!("unexpected character '{}'", parser.input[parser.pos] as char));
    }
    Ok(result)
}

// ── App launcher ──────────────────────────────────────────────────────────────

/// Spawn the application described by `entry` as a detached child process.
pub fn launch_app(entry: &AppEntry) -> Result<(), LauncherError> {
    let exec = clean_exec(&entry.exec);
    if exec.is_empty() {
        return Err(LauncherError::EmptyExec(entry.name.clone()));
    }

    let mut parts = exec.split_whitespace();
    let cmd = parts.next().unwrap(); // safe: exec is non-empty
    let args: Vec<&str> = parts.collect();

    std::process::Command::new(cmd)
        .args(&args)
        .spawn()
        .map_err(|e| LauncherError::LaunchFailed { name: entry.name.clone(), source: e })?;

    Ok(())
}

/// Strip `.desktop` field codes (%f, %F, %u, %U, etc.) from an exec string.
fn clean_exec(exec: &str) -> String {
    exec.split_whitespace()
        .filter(|tok| !tok.starts_with('%'))
        .collect::<Vec<_>>()
        .join(" ")
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn make_app(name: &str, exec: &str) -> AppEntry {
        AppEntry {
            id: name.to_lowercase(),
            name: name.to_owned(),
            exec: exec.to_owned(),
            icon: String::new(),
            description: String::new(),
            keywords: Vec::new(),
        }
    }

    // --- fuzzy_search ---

    #[test]
    fn fuzzy_search_empty_query_returns_all() {
        let apps = vec![make_app("Firefox", "firefox"), make_app("Gedit", "gedit")];
        let results = fuzzy_search("", &apps);
        assert_eq!(results.len(), 2);
    }

    #[test]
    fn fuzzy_search_matches_subsequence() {
        let apps = vec![make_app("Firefox", "firefox"), make_app("Files", "nautilus")];
        let results = fuzzy_search("fi", &apps);
        assert!(!results.is_empty());
        assert!(results.iter().any(|r| r.entry.name == "Firefox"));
    }

    #[test]
    fn fuzzy_search_no_match_returns_empty() {
        let apps = vec![make_app("Firefox", "firefox")];
        let results = fuzzy_search("zzz", &apps);
        assert!(results.is_empty());
    }

    #[test]
    fn fuzzy_search_sorted_by_score_desc() {
        let apps = vec![
            make_app("Foo Bar", "foo"),
            make_app("fo", "fo"),
        ];
        let results = fuzzy_search("fo", &apps);
        assert!(results.len() >= 2);
        // "fo" should score higher than "Foo Bar" for query "fo"
        assert_eq!(results[0].entry.name, "fo");
    }

    // --- evaluate_expression ---

    #[test]
    fn eval_addition() {
        let r = evaluate_expression("1 + 2");
        assert!(!r.is_error);
        assert_eq!(r.result, "3");
    }

    #[test]
    fn eval_subtraction() {
        let r = evaluate_expression("10 - 4");
        assert!(!r.is_error);
        assert_eq!(r.result, "6");
    }

    #[test]
    fn eval_multiplication() {
        let r = evaluate_expression("3 * 4");
        assert!(!r.is_error);
        assert_eq!(r.result, "12");
    }

    #[test]
    fn eval_division() {
        let r = evaluate_expression("10 / 4");
        assert!(!r.is_error);
        assert_eq!(r.result, "2.5");
    }

    #[test]
    fn eval_power() {
        let r = evaluate_expression("2 ^ 10");
        assert!(!r.is_error);
        assert_eq!(r.result, "1024");
    }

    #[test]
    fn eval_parentheses() {
        let r = evaluate_expression("(1 + 2) * 3");
        assert!(!r.is_error);
        assert_eq!(r.result, "9");
    }

    #[test]
    fn eval_unary_minus() {
        let r = evaluate_expression("-5 + 3");
        assert!(!r.is_error);
        assert_eq!(r.result, "-2");
    }

    #[test]
    fn eval_division_by_zero() {
        let r = evaluate_expression("1 / 0");
        assert!(r.is_error);
    }

    #[test]
    fn eval_empty_expression() {
        let r = evaluate_expression("");
        assert!(r.is_error);
    }

    #[test]
    fn eval_invalid_expression() {
        let r = evaluate_expression("1 +");
        assert!(r.is_error);
    }

    // --- clean_exec ---

    #[test]
    fn clean_exec_strips_field_codes() {
        let result = clean_exec("code %F");
        assert_eq!(result, "code");
    }

    #[test]
    fn clean_exec_preserves_plain_args() {
        let result = clean_exec("gedit --new-window");
        assert_eq!(result, "gedit --new-window");
    }

    // --- match_ranges ---

    #[test]
    fn fuzzy_match_ranges_non_empty_for_match() {
        let result = fuzzy_match("fi", "firefox");
        assert!(result.is_some());
        let (_, ranges) = result.unwrap();
        assert!(!ranges.is_empty());
    }
}
