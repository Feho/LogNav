use super::LogParser;
use crate::log_entry::LogLevel;
use chrono::NaiveDateTime;
use regex::Regex;
use serde::Deserialize;
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::sync::Arc;

/// YAML-defined log format
#[derive(Debug, Deserialize)]
pub struct YamlFormatDef {
    pub name: String,
    /// Regex with optional named groups: `level`, `timestamp`
    pub pattern: String,
    /// Map level tokens (captured by `level` group) → LogLevel names
    #[serde(default)]
    pub level_map: HashMap<String, String>,
    /// Chrono strftime format for the `timestamp` group
    pub timestamp_format: Option<String>,
    /// If true, prepend current year to timestamp before parsing
    #[serde(default)]
    pub auto_year: bool,
    /// Regex to strip artifacts from lines (e.g. color codes)
    pub clean_pattern: Option<String>,
    /// String hint for detection fallback (0.8 confidence if first line contains this)
    pub detect_hint: Option<String>,
    /// If true, trim whitespace before detection (needed for wpc format)
    #[serde(default)]
    pub trim_for_detect: bool,
}

/// Runtime parser compiled from a YamlFormatDef
pub struct YamlParser {
    #[allow(dead_code)]
    name: String,
    pattern: Regex,
    level_map: HashMap<String, LogLevel>,
    timestamp_format: Option<String>,
    auto_year: bool,
    clean_pattern: Option<Regex>,
    detect_hint: Option<String>,
    trim_for_detect: bool,
}

impl YamlParser {
    /// Compile a YamlFormatDef into a usable parser
    pub fn from_def(def: YamlFormatDef) -> Result<Self, String> {
        let pattern =
            Regex::new(&def.pattern).map_err(|e| format!("{}: bad pattern: {}", def.name, e))?;

        let clean_pattern = def
            .clean_pattern
            .as_deref()
            .map(Regex::new)
            .transpose()
            .map_err(|e| format!("{}: bad clean_pattern: {}", def.name, e))?;

        let level_map = def
            .level_map
            .iter()
            .map(|(k, v)| (k.clone(), parse_level_name(v)))
            .collect();

        Ok(Self {
            name: def.name,
            pattern,
            level_map,
            timestamp_format: def.timestamp_format,
            auto_year: def.auto_year,
            clean_pattern,
            detect_hint: def.detect_hint,
            trim_for_detect: def.trim_for_detect,
        })
    }

    fn parse_timestamp_str(&self, ts: &str) -> Option<NaiveDateTime> {
        let fmt = self.timestamp_format.as_deref()?;
        if self.auto_year {
            let year = chrono::Local::now().format("%Y").to_string();
            let full_ts = format!("{}-{}", year, ts);
            let full_fmt = format!("%Y-{}", fmt);
            NaiveDateTime::parse_from_str(&full_ts, &full_fmt).ok()
        } else {
            NaiveDateTime::parse_from_str(ts, fmt).ok()
        }
    }
}

impl LogParser for YamlParser {
    fn name(&self) -> &str {
        &self.name
    }

    fn detect(&self, first_line: &str) -> f64 {
        let line = if self.trim_for_detect {
            first_line.trim()
        } else {
            first_line
        };

        if self.pattern.is_match(line) {
            return 1.0;
        }

        if let Some(hint) = &self.detect_hint
            && first_line.contains(hint.as_str())
        {
            return 0.8;
        }

        0.0
    }

    fn parse_line(&self, line: &str) -> Option<(LogLevel, Option<NaiveDateTime>)> {
        let caps = self.pattern.captures(line)?;

        let level = caps
            .name("level")
            .and_then(|m| self.level_map.get(m.as_str()))
            .copied()
            .unwrap_or(LogLevel::Info);

        let timestamp = caps
            .name("timestamp")
            .and_then(|m| self.parse_timestamp_str(m.as_str()));

        Some((level, timestamp))
    }

    fn clean_line(&self, line: &str) -> String {
        if let Some(re) = &self.clean_pattern {
            re.replace_all(line, "").into_owned()
        } else {
            line.to_string()
        }
    }
}

/// Map level name string to LogLevel enum
fn parse_level_name(name: &str) -> LogLevel {
    match name.to_lowercase().as_str() {
        "trace" => LogLevel::Trace,
        "debug" => LogLevel::Debug,
        "info" => LogLevel::Info,
        "warn" | "warning" => LogLevel::Warn,
        "error" => LogLevel::Error,
        "profile" => LogLevel::Profile,
        _ => LogLevel::Unknown,
    }
}

/// Load all YAML format definitions from a directory
pub fn load_formats(dir: &Path) -> Vec<Arc<dyn LogParser>> {
    let mut parsers: Vec<Arc<dyn LogParser>> = Vec::new();

    let entries = match fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return parsers,
    };

    for entry in entries.flatten() {
        let path = entry.path();
        let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
        if ext != "yaml" && ext != "yml" {
            continue;
        }

        match load_format_file(&path) {
            Ok(parser) => parsers.push(Arc::new(parser)),
            Err(e) => eprintln!("Warning: skipping {}: {}", path.display(), e),
        }
    }

    parsers
}

/// Load and compile a single YAML format file
fn load_format_file(path: &Path) -> Result<YamlParser, String> {
    let content = fs::read_to_string(path).map_err(|e| format!("read error: {}", e))?;
    load_format_str(&content)
}

/// Parse and compile a YAML format definition from a string
pub fn load_format_str(yaml: &str) -> Result<YamlParser, String> {
    let def: YamlFormatDef =
        serde_yaml::from_str(yaml).map_err(|e| format!("parse error: {}", e))?;
    YamlParser::from_def(def)
}
