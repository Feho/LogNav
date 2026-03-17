use std::path::{Path, PathBuf};
use tokio::sync::mpsc;
use semver::Version;

/// Check for updates in a local or network folder and apply them if available.
pub async fn check_and_update(tx: mpsc::Sender<String>, update_path: String) {
    let current_version_str = env!("CARGO_PKG_VERSION");
    let current_version = Version::parse(current_version_str).unwrap_or_else(|_| Version::new(0, 0, 0));

    let result = tokio::task::spawn_blocking(move || -> Result<Option<(String, PathBuf)>, Box<dyn std::error::Error + Send + Sync>> {
        let path = Path::new(&update_path);
        if !path.is_dir() {
            return Err(format!("Update path is not a directory: {}", update_path).into());
        }

        let mut latest: Option<(Version, PathBuf)> = None;

        // Expecting files like lognav-v1.2.3 or lognav-1.2.3(.exe)
        for entry in std::fs::read_dir(path)? {
            let entry = entry?;
            let name = entry.file_name().to_string_lossy().to_string();
            
            if !name.starts_with("lognav") {
                continue;
            }

            // Extract version string: look for digits and dots
            let version_part = name.trim_start_matches("lognav")
                .trim_start_matches("-v")
                .trim_start_matches("-")
                .trim_end_matches(".exe");

            if let Ok(v) = Version::parse(version_part) {
                if v > current_version {
                    if let Some((ref lv, _)) = latest {
                        if v > *lv {
                            latest = Some((v, entry.path()));
                        }
                    } else {
                        latest = Some((v, entry.path()));
                    }
                }
            }
        }

        if let Some((v, path)) = latest {
            let new_version = v.to_string();
            
            // Perform self-replace
            self_replace::self_replace(&path)?;
            
            Ok(Some((new_version, path)))
        } else {
            Ok(None)
        }
    }).await;

    match result {
        Ok(Ok(Some((v, _)))) => {
            let _ = tx.send(format!("Update available: v{} (installed)", v)).await;
        }
        Ok(Ok(None)) => {
            // Keep it quiet if up to date on startup
        }
        Ok(Err(e)) => {
            let _ = tx.send(format!("Update check failed: {}", e)).await;
        }
        Err(e) => {
            let _ = tx.send(format!("Update task panicked: {}", e)).await;
        }
    }
}
