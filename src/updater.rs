use std::sync::mpsc;
use std::time::{Duration, Instant};

const REPO_OWNER: &str = "matutetandil";
const REPO_NAME: &str = "queuepeek";

const CHECK_INTERVAL: Duration = Duration::from_secs(3600); // 1 hour

pub enum UpdateStatus {
    Available(String), // new version string
    UpToDate,
    Error(#[allow(dead_code)] String),
}

pub struct UpdateChecker {
    pub latest_version: Option<String>,
    pub update_available: bool,
    pub checking: bool,
    pub last_check: Option<Instant>,
    tx: mpsc::Sender<UpdateStatus>,
    rx: mpsc::Receiver<UpdateStatus>,
}

impl UpdateChecker {
    pub fn new() -> Self {
        let (tx, rx) = mpsc::channel();
        Self {
            latest_version: None,
            update_available: false,
            checking: false,
            last_check: None,
            tx,
            rx,
        }
    }

    pub fn should_check(&self) -> bool {
        if self.checking {
            return false;
        }
        match self.last_check {
            None => true,
            Some(last) => last.elapsed() >= CHECK_INTERVAL,
        }
    }

    pub fn start_check(&mut self) {
        if self.checking {
            return;
        }
        self.checking = true;
        let tx = self.tx.clone();
        let current = env!("CARGO_PKG_VERSION").to_string();

        std::thread::spawn(move || {
            let status = match check_latest_version(&current) {
                Ok(Some(version)) => UpdateStatus::Available(version),
                Ok(None) => UpdateStatus::UpToDate,
                Err(e) => UpdateStatus::Error(e),
            };
            let _ = tx.send(status);
        });
    }

    pub fn poll(&mut self) {
        if let Ok(status) = self.rx.try_recv() {
            self.checking = false;
            self.last_check = Some(Instant::now());
            match status {
                UpdateStatus::Available(version) => {
                    self.latest_version = Some(version);
                    self.update_available = true;
                }
                UpdateStatus::UpToDate => {
                    self.update_available = false;
                }
                UpdateStatus::Error(_) => {
                    // Silent fail — don't bother the user
                }
            }
        }
    }
}

fn check_latest_version(current: &str) -> Result<Option<String>, String> {
    let releases = self_update::backends::github::ReleaseList::configure()
        .repo_owner(REPO_OWNER)
        .repo_name(REPO_NAME)
        .build()
        .map_err(|e| format!("Building release list: {}", e))?
        .fetch()
        .map_err(|e| format!("Fetching releases: {}", e))?;

    if let Some(latest) = releases.first() {
        let latest_ver = latest.version.trim_start_matches('v');
        if version_newer(latest_ver, current) {
            return Ok(Some(latest_ver.to_string()));
        }
    }

    Ok(None)
}

fn version_newer(latest: &str, current: &str) -> bool {
    let parse = |v: &str| -> Vec<u32> {
        v.split('.')
            .filter_map(|s| s.parse().ok())
            .collect()
    };
    let l = parse(latest);
    let c = parse(current);
    l > c
}

pub fn perform_update() -> Result<String, String> {
    // Redirect stdout to /dev/null during update to prevent TUI corruption.
    // self_update writes progress text to stdout even with show_download_progress(false).
    use std::os::unix::io::AsRawFd;
    let devnull = std::fs::File::open("/dev/null").map_err(|e| format!("Opening /dev/null: {}", e))?;
    let stdout_fd = std::io::stdout().as_raw_fd();
    let saved_stdout = unsafe { libc::dup(stdout_fd) };
    unsafe { libc::dup2(devnull.as_raw_fd(), stdout_fd) };

    let result = self_update::backends::github::Update::configure()
        .repo_owner(REPO_OWNER)
        .repo_name(REPO_NAME)
        .bin_name("queuepeek")
        .show_download_progress(false)
        .no_confirm(true)
        .current_version(env!("CARGO_PKG_VERSION"))
        .build()
        .map_err(|e| format!("Configuring update: {}", e))?
        .update()
        .map_err(|e| format!("Performing update: {}", e));

    // Restore stdout
    unsafe { libc::dup2(saved_stdout, stdout_fd) };
    unsafe { libc::close(saved_stdout) };

    let status = result?;
    Ok(format!("Updated to v{}. Restart to apply.", status.version()))
}
