//! Daemon detection, HTTP client, and in-process kernel boot.

use openfang_api::server::read_daemon_info;
use openfang_kernel::OpenFangKernel;
use std::path::PathBuf;

/// Try to find a running daemon. Returns its base URL if found.
/// SECURITY: Restrict file permissions to owner-only (0600) on Unix.
#[cfg(unix)]
pub(crate) fn restrict_file_permissions(path: &std::path::Path) {
    use std::os::unix::fs::PermissionsExt;
    let _ = std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600));
}

#[cfg(not(unix))]
pub(crate) fn restrict_file_permissions(_path: &std::path::Path) {}

/// SECURITY: Restrict directory permissions to owner-only (0700) on Unix.
#[cfg(unix)]
pub(crate) fn restrict_dir_permissions(path: &std::path::Path) {
    use std::os::unix::fs::PermissionsExt;
    let _ = std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o700));
}

#[cfg(not(unix))]
pub(crate) fn restrict_dir_permissions(_path: &std::path::Path) {}

pub(crate) fn find_daemon() -> Option<String> {
    let home_dir = dirs::home_dir()?.join(".openfang");
    let info = read_daemon_info(&home_dir)?;

    // Normalize listen address: replace 0.0.0.0 with 127.0.0.1 to avoid
    // DNS/connectivity issues on macOS where 0.0.0.0 can hang.
    let addr = info.listen_addr.replace("0.0.0.0", "127.0.0.1");
    let url = format!("http://{addr}/api/health");

    let client = reqwest::blocking::Client::builder()
        .connect_timeout(std::time::Duration::from_secs(1))
        .timeout(std::time::Duration::from_secs(2))
        .build()
        .ok()?;
    let resp = client.get(&url).send().ok()?;
    if resp.status().is_success() {
        Some(format!("http://{addr}"))
    } else {
        None
    }
}

/// Build an HTTP client for daemon calls.
pub(crate) fn daemon_client() -> reqwest::blocking::Client {
    let mut builder =
        reqwest::blocking::Client::builder().timeout(std::time::Duration::from_secs(120));

    if let Some(api_key) = daemon_api_key() {
        let mut headers = reqwest::header::HeaderMap::new();
        let auth_value = format!("Bearer {api_key}");
        if let Ok(mut hv) = reqwest::header::HeaderValue::from_str(&auth_value) {
            hv.set_sensitive(true);
            headers.insert(reqwest::header::AUTHORIZATION, hv);
            builder = builder.default_headers(headers);
        }
    }

    builder.build().expect("Failed to build HTTP client")
}

fn daemon_api_key() -> Option<String> {
    let config_path = dirs::home_dir()?.join(".openfang").join("config.toml");
    let content = std::fs::read_to_string(config_path).ok()?;
    parse_api_key_from_config_toml(&content)
}

pub(crate) fn parse_api_key_from_config_toml(content: &str) -> Option<String> {
    let table: toml::Value = toml::from_str(content).ok()?;
    let api_key = table.get("api_key")?.as_str()?.trim();
    if api_key.is_empty() {
        None
    } else {
        Some(api_key.to_string())
    }
}

/// Helper: send a request to the daemon and parse the JSON body.
/// Exits with error on connection failure.
pub(crate) fn daemon_json(
    resp: Result<reqwest::blocking::Response, reqwest::Error>,
) -> serde_json::Value {
    match resp {
        Ok(r) => {
            let status = r.status();
            let body = r.json::<serde_json::Value>().unwrap_or_default();
            if status.is_server_error() {
                crate::ui::error_with_fix(
                    &format!("Daemon returned error ({})", status),
                    "Check daemon logs: ~/.openfang/tui.log",
                );
            }
            body
        }
        Err(e) => {
            let msg = e.to_string();
            if msg.contains("timed out") || msg.contains("Timeout") {
                crate::ui::error_with_fix(
                    "Request timed out",
                    "The agent may be processing a complex request. Try again, or check `openfang status`",
                );
            } else if msg.contains("Connection refused") || msg.contains("connect") {
                crate::ui::error_with_fix(
                    "Cannot connect to daemon",
                    "Is the daemon running? Start it with: openfang start",
                );
            } else {
                crate::ui::error_with_fix(
                    &format!("Daemon communication error: {msg}"),
                    "Check `openfang status` or restart: openfang start",
                );
            }
            std::process::exit(1);
        }
    }
}

/// Require a running daemon — exit with helpful message if not found.
pub(crate) fn require_daemon(command: &str) -> String {
    find_daemon().unwrap_or_else(|| {
        crate::ui::error_with_fix(
            &format!("`openfang {command}` requires a running daemon"),
            "Start the daemon: openfang start",
        );
        crate::ui::hint("Or try `openfang chat` which works without a daemon");
        std::process::exit(1);
    })
}

/// Show context-aware error for kernel boot failures.
pub(crate) fn boot_kernel_error(e: &openfang_kernel::error::KernelError) {
    let msg = e.to_string();
    if msg.contains("parse") || msg.contains("toml") || msg.contains("config") {
        crate::ui::error_with_fix(
            "Failed to parse configuration",
            "Check your config.toml syntax: openfang config show",
        );
    } else if msg.contains("database") || msg.contains("locked") || msg.contains("sqlite") {
        crate::ui::error_with_fix(
            "Database error (file may be locked)",
            "Check if another OpenFang process is running: openfang status",
        );
    } else if msg.contains("key") || msg.contains("API") || msg.contains("auth") {
        crate::ui::error_with_fix(
            "LLM provider authentication failed",
            "Run `openfang doctor` to check your API key configuration",
        );
    } else {
        crate::ui::error_with_fix(
            &format!("Failed to boot kernel: {msg}"),
            "Run `openfang doctor` to diagnose the issue",
        );
    }
}

pub(crate) fn boot_kernel(config: Option<PathBuf>) -> OpenFangKernel {
    match OpenFangKernel::boot(config.as_deref()) {
        Ok(k) => k,
        Err(e) => {
            boot_kernel_error(&e);
            std::process::exit(1);
        }
    }
}

pub(crate) fn force_kill_pid(pid: u32) {
    #[cfg(unix)]
    {
        let _ = std::process::Command::new("kill")
            .args(["-9", &pid.to_string()])
            .output();
    }
    #[cfg(windows)]
    {
        let _ = std::process::Command::new("taskkill")
            .args(["/PID", &pid.to_string(), "/F"])
            .output();
    }
}
