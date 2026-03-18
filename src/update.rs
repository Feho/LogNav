use log::{error, info};
use tokio::sync::oneshot;

pub fn check_and_update(tx: oneshot::Sender<Option<String>>) {
    info!(
        "checking for updates (current: v{})",
        env!("CARGO_PKG_VERSION")
    );

    let result = self_update::backends::github::Update::configure()
        .repo_owner("feho")
        .repo_name("lognav")
        .bin_name("lognav")
        .current_version(env!("CARGO_PKG_VERSION"))
        .show_download_progress(false)
        .show_output(false)
        .no_confirm(true)
        .build()
        .and_then(|u| u.update());

    let msg = match result {
        Ok(self_update::Status::Updated(v)) => {
            info!("updated to v{v}");
            Some(format!("Updated to v{v} — restart to apply"))
        }
        Ok(self_update::Status::UpToDate(v)) => {
            info!("already up to date (v{v})");
            None
        }
        Err(e) => {
            error!("update check failed: {e}");
            None
        }
    };
    let _ = tx.send(msg);
}
