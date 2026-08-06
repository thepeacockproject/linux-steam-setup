use anyhow::{Context, Result};
use std::fmt;
use std::path::PathBuf;
use std::time::{Duration, Instant};

use super::config::{Config, config_base_dir};

const SERVICE_NAME: &str = "peacock.service";
const STOP_TIMEOUT: Duration = Duration::from_secs(30);
const STOP_POLL_INTERVAL: Duration = Duration::from_millis(100);

/// Current state of the systemd user service.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ServiceState {
    /// Service file exists and is running
    Active,
    /// Service file exists but is not running
    Inactive,
    /// Service file exists but has failed
    Failed,
    /// Service file is not installed
    NotInstalled,
}

impl fmt::Display for ServiceState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ServiceState::Active => write!(f, "Active (running)"),
            ServiceState::Inactive => write!(f, "Inactive (stopped)"),
            ServiceState::Failed => write!(f, "Failed"),
            ServiceState::NotInstalled => write!(f, "Not installed"),
        }
    }
}

/// Full status of the Peacock service.
#[derive(Debug, Clone)]
pub struct ServiceStatus {
    pub state: ServiceState,
    pub enabled: bool,
}

#[derive(Debug, Clone, Default)]
pub struct ServiceReplaceResult {
    pub had_existing_service: bool,
    pub replaced_legacy_service: bool,
    pub restored_enabled: bool,
    pub restored_running: bool,
}

/// Get the path to the systemd user service file.
fn service_file_path() -> PathBuf {
    config_base_dir()
        .join("systemd")
        .join("user")
        .join(SERVICE_NAME)
}

/// Generate the systemd unit file content.
fn generate_unit_file(config: &Config) -> String {
    let install_dir = config.install_dir.display();
    let port = config.port;

    // Note: A previous PR (https://github.com/thepeacockproject/linux-steam-setup/pull/11)
    // notified about the use of `default.target` in the `WantedBy` directive instead of
    // `multi-user.target`.
    // While correct for most system services, `default.target` is
    // actually the recommended target for user services. See:
    // https://man.archlinux.org/man/systemd.special.7#UNITS_MANAGED_BY_THE_USER_SERVICE_MANAGER
    format!(
        r#"[Unit]
Description=Peacock Server
After=network.target

[Service]
Type=simple
WorkingDirectory={install_dir}/Peacock
ExecStart={install_dir}/node/bin/node {install_dir}/Peacock/chunk0.js
Environment=PORT={port}
Restart=on-failure
RestartSec=5

[Install]
WantedBy=default.target
"#
    )
}

fn unit_file_looks_legacy(contents: &str) -> bool {
    contents.contains("linux-steam-setup/start.sh")
        || contents.contains("WorkingDirectory=%h/linux-steam-setup")
        || contents.contains("WantedBy=multi-user.target")
}

/// Check the current status of the Peacock service.
pub fn status() -> ServiceStatus {
    let service_path = service_file_path();

    if !service_path.exists() {
        return ServiceStatus {
            state: ServiceState::NotInstalled,
            enabled: false,
        };
    }

    let state = get_active_state();
    let enabled = is_enabled();

    ServiceStatus { state, enabled }
}

fn get_active_state() -> ServiceState {
    match active_state_name() {
        Ok(state) => match state.as_str() {
            "active" | "activating" | "reloading" => ServiceState::Active,
            "failed" => ServiceState::Failed,
            _ => ServiceState::Inactive,
        },
        Err(_) => ServiceState::Inactive,
    }
}

fn active_state_name() -> Result<String> {
    let output = std::process::Command::new("systemctl")
        .args(["--user", "is-active", "peacock"])
        .output()
        .context("Failed to check whether the Peacock service is running")?;
    let state = String::from_utf8_lossy(&output.stdout).trim().to_owned();

    if state.is_empty() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("Could not determine Peacock service state: {stderr}");
    }

    Ok(state)
}

fn is_fully_stopped(state: &str) -> bool {
    matches!(state, "inactive" | "failed" | "unknown")
}

fn is_enabled() -> bool {
    std::process::Command::new("systemctl")
        .args(["--user", "is-enabled", "peacock"])
        .output()
        .map(|out| String::from_utf8_lossy(&out.stdout).trim() == "enabled")
        .unwrap_or(false)
}

fn daemon_reload() -> Result<()> {
    let output = std::process::Command::new("systemctl")
        .args(["--user", "daemon-reload"])
        .output()
        .context("Failed to run systemctl daemon-reload")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("daemon-reload failed: {stderr}");
    }
    Ok(())
}

fn ensure_stopped_before_changes() -> Result<()> {
    stop_if_running_and_wait()
}

/// Stop the Peacock service if it is running and wait until it is fully off.
///
/// The timeout prevents an update from changing files while systemd is still
/// stopping Peacock. A timeout or state-query failure aborts the caller's update.
pub fn stop_if_running_and_wait() -> Result<()> {
    if !service_file_path().exists() {
        return Ok(());
    }

    let state = active_state_name()?;
    if is_fully_stopped(&state) {
        return Ok(());
    }

    if state != "deactivating" {
        run_systemctl(&["--no-block", "stop", "peacock"])
            .context("Failed to stop Peacock service before updating")?;
    }

    let started = Instant::now();
    loop {
        let state = active_state_name()?;
        if is_fully_stopped(&state) {
            return Ok(());
        }

        if started.elapsed() >= STOP_TIMEOUT {
            anyhow::bail!(
                "Peacock service did not stop within {} seconds (state: {state})",
                STOP_TIMEOUT.as_secs()
            );
        }

        std::thread::sleep(STOP_POLL_INTERVAL);
    }
}

/// Install the systemd service (write unit file + daemon-reload).
pub fn install(config: &Config) -> Result<()> {
    let path = service_file_path();

    if path.exists() {
        ensure_stopped_before_changes()?;
    }

    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).context("Failed to create systemd user directory")?;
    }

    let content = generate_unit_file(config);
    std::fs::write(&path, content).context("Failed to write service file")?;
    daemon_reload()?;
    Ok(())
}

/// Replace any existing service definition with the current launcher-managed one.
///
/// If a service already exists, its enabled/running state is restored after the
/// new unit file is installed so migration remains seamless for existing users.
pub fn replace(config: &Config) -> Result<ServiceReplaceResult> {
    let path = service_file_path();
    let had_existing_service = path.exists();
    let was_enabled = had_existing_service && is_enabled();
    let was_running = had_existing_service && get_active_state() == ServiceState::Active;
    let replaced_legacy_service = std::fs::read_to_string(&path)
        .map(|contents| unit_file_looks_legacy(&contents))
        .unwrap_or(false);

    if had_existing_service {
        remove()?;
    }

    install(config)?;

    let mut result = ServiceReplaceResult {
        had_existing_service,
        replaced_legacy_service,
        restored_enabled: false,
        restored_running: false,
    };

    if was_enabled {
        enable()?;
        result.restored_enabled = true;
    }

    if was_running {
        start()?;
        result.restored_running = true;
    }

    Ok(result)
}

/// Remove the systemd service (stop + disable + delete + daemon-reload).
pub fn remove() -> Result<()> {
    ensure_stopped_before_changes()?;
    let _ = disable(); // best-effort disable

    let path = service_file_path();
    if path.exists() {
        std::fs::remove_file(&path).context("Failed to remove service file")?;
    }

    daemon_reload()?;
    Ok(())
}

/// Start the Peacock service.
pub fn start() -> Result<()> {
    run_systemctl(&["start", "peacock"])
}

/// Stop the Peacock service.
pub fn stop() -> Result<()> {
    run_systemctl(&["stop", "peacock"])
}

/// Enable the service to start on boot.
pub fn enable() -> Result<()> {
    run_systemctl(&["enable", "peacock"])
}

/// Disable the service from starting on boot.
pub fn disable() -> Result<()> {
    run_systemctl(&["disable", "peacock"])
}

/// Fetch recent journal lines for the Peacock service.
/// Returns the last `lines` entries from `journalctl --user -u peacock`.
pub fn journal(lines: usize) -> Vec<String> {
    let output = std::process::Command::new("journalctl")
        .args([
            "--user",
            "-u",
            "peacock",
            "--no-pager",
            "-n",
            &lines.to_string(),
        ])
        .output();

    match output {
        Ok(out) => {
            let text = String::from_utf8_lossy(&out.stdout);
            text.lines().map(|l| l.to_string()).collect()
        }
        Err(e) => vec![format!("Failed to read journal: {e}")],
    }
}

fn run_systemctl(args: &[&str]) -> Result<()> {
    let mut full_args = vec!["--user"];
    full_args.extend_from_slice(args);

    let output = std::process::Command::new("systemctl")
        .args(&full_args)
        .output()
        .with_context(|| format!("Failed to run systemctl {}", args.join(" ")))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("systemctl {} failed: {stderr}", args.join(" "));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn legacy_unit_detection_matches_old_setup() {
        let legacy_unit = r#"[Unit]
Description=Peacock

[Service]
WorkingDirectory=%h/linux-steam-setup
ExecStart=%h/linux-steam-setup/start.sh

[Install]
WantedBy=multi-user.target
"#;

        assert!(unit_file_looks_legacy(legacy_unit));
    }

    #[test]
    fn legacy_unit_detection_ignores_current_launcher_unit() {
        let config = Config {
            install_dir: PathBuf::from("/home/user/.local/share/peacock-linux"),
            port: 3000,
            ..Config::default()
        };

        assert!(!unit_file_looks_legacy(&generate_unit_file(&config)));
    }

    #[test]
    fn unit_file_generation() {
        let config = Config {
            install_dir: PathBuf::from("/home/user/.local/share/peacock-linux"),
            port: 3000,
            ..Config::default()
        };

        let unit = generate_unit_file(&config);
        assert!(unit.contains("WorkingDirectory=/home/user/.local/share/peacock-linux/Peacock"));
        assert!(unit.contains("Environment=PORT=3000"));
        assert!(unit.contains("ExecStart=/home/user/.local/share/peacock-linux/node/bin/node"));
        assert!(unit.contains("WantedBy=default.target"));
    }

    #[test]
    fn fully_stopped_states_exclude_transitional_states() {
        assert!(is_fully_stopped("inactive"));
        assert!(is_fully_stopped("failed"));
        assert!(is_fully_stopped("unknown"));
        assert!(!is_fully_stopped("active"));
        assert!(!is_fully_stopped("activating"));
        assert!(!is_fully_stopped("deactivating"));
    }
}
