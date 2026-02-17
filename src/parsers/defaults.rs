use std::fs;
use std::path::Path;

const WD_YAML: &str = r#"name: "wd"

# Format: [prefix 1-2 chars][level] [MM-dd HH:mm:ss.fff] [thread] component|sub "msg"
# Prefixes: "  " (normal), "* " (error), "! " (warn), "? " (audit), "**" (fatal)
pattern: '^[*!?# ]{1,2}(?P<level>~~~~~|=====|TRACE|AUDIT|INFO|WARN|ERROR|FATAL)\s+(?P<timestamp>\d{2}-\d{2}\s+\d{2}:\d{2}:\d{2}\.\d{3})'

level_map:
  "TRACE": trace
  "AUDIT": trace
  "INFO": info
  "WARN": warn
  "ERROR": error
  "FATAL": error
  "=====": debug
  "~~~~~": profile

timestamp_format: "%m-%d %H:%M:%S%.3f"
auto_year: true
"#;

const WPC_YAML: &str = r#"name: "wpc"

# Format: [level] [MM-dd HH:mm:ss.fff] message
pattern: '^(?P<level>VRB|DBG|INF|WRN|ERR)\s+(?P<timestamp>\d{2}-\d{2}\s+\d{2}:\d{2}:\d{2}\.\d{3})'

level_map:
  "VRB": trace
  "DBG": debug
  "INF": info
  "WRN": warn
  "ERR": error

timestamp_format: "%m-%d %H:%M:%S%.3f"
auto_year: true
trim_for_detect: true
"#;

const QCONSOLE_YAML: &str = r#"name: "qconsole"

# Format: [YYYY-MM-DD HH:MM:SS UTC±H.000] message
# Level is not in the format; all lines default to Info.
pattern: '^\[(?P<timestamp>\d{4}-\d{2}-\d{2}\s+\d{2}:\d{2}:\d{2})\s+UTC[+-][\d.]+\]'

timestamp_format: "%Y-%m-%d %H:%M:%S"
clean_pattern: '\^[0-9]'
detect_hint: "logfile opened on"
"#;

const DEFAULTS: &[(&str, &str)] = &[
    ("wd.yaml", WD_YAML),
    ("wpc.yaml", WPC_YAML),
    ("qconsole.yaml", QCONSOLE_YAML),
];

/// Write default format files into `dir` if they don't already exist.
pub fn ensure_default_formats(dir: &Path) {
    if let Err(e) = fs::create_dir_all(dir) {
        eprintln!("Warning: can't create formats dir {}: {}", dir.display(), e);
        return;
    }

    for (filename, content) in DEFAULTS {
        let path = dir.join(filename);
        if !path.exists()
            && let Err(e) = fs::write(&path, content)
        {
            eprintln!("Warning: can't write {}: {}", path.display(), e);
        }
    }
}
