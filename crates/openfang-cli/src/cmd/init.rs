//! Init, onboarding, and daemon start/restart/stop.

use crate::daemon::{
    boot_kernel_error, daemon_client, find_daemon, force_kill_pid, restrict_dir_permissions,
    restrict_file_permissions,
};
use crate::ui;
use openfang_api::server::read_daemon_info;
use openfang_kernel::OpenFangKernel;
use std::path::PathBuf;

pub fn cmd_init(quick: bool) {
    let home = match dirs::home_dir() {
        Some(h) => h,
        None => {
            ui::error("Could not determine home directory");
            std::process::exit(1);
        }
    };

    let openfang_dir = home.join(".openfang");

    if !openfang_dir.exists() {
        std::fs::create_dir_all(&openfang_dir).unwrap_or_else(|e| {
            ui::error_with_fix(
                &format!("Failed to create {}", openfang_dir.display()),
                &format!("Check permissions on {}", home.display()),
            );
            eprintln!("  {e}");
            std::process::exit(1);
        });
        restrict_dir_permissions(&openfang_dir);
    }

    for sub in ["data", "agents"] {
        let dir = openfang_dir.join(sub);
        if !dir.exists() {
            std::fs::create_dir_all(&dir).unwrap_or_else(|e| {
                eprintln!("Error creating {sub} dir: {e}");
                std::process::exit(1);
            });
        }
    }

    crate::bundled_agents::install_bundled_agents(&openfang_dir.join("agents"));

    if quick {
        cmd_init_quick(&openfang_dir);
    } else {
        cmd_init_interactive(&openfang_dir);
    }
}

fn cmd_init_quick(openfang_dir: &std::path::Path) {
    ui::banner();
    ui::blank();

    let (provider, api_key_env, model) = detect_best_provider();
    write_config_if_missing(openfang_dir, provider, model, api_key_env);

    ui::blank();
    ui::success("OpenFang initialized (quick mode)");
    ui::kv("Provider", provider);
    ui::kv("Model", model);
    ui::blank();
    ui::next_steps(&[
        "Start the daemon:  openfang start",
        "Chat:              openfang chat",
    ]);
}

fn cmd_init_interactive(openfang_dir: &std::path::Path) {
    use crate::tui::screens::init_wizard::{self, InitResult, LaunchChoice};

    match init_wizard::run() {
        InitResult::Completed {
            provider,
            model,
            daemon_started,
            launch,
        } => {
            ui::blank();
            ui::success("OpenFang initialized!");
            ui::kv("Provider", &provider);
            ui::kv("Model", &model);

            if daemon_started {
                ui::kv_ok("Daemon", "running");
            }
            ui::blank();

            match launch {
                LaunchChoice::Desktop => {
                    launch_desktop_app(openfang_dir);
                }
                LaunchChoice::Dashboard => {
                    if let Some(base) = find_daemon() {
                        let url = format!("{base}/");
                        ui::success(&format!("Opening dashboard at {url}"));
                        if !crate::open_in_browser(&url) {
                            ui::hint(&format!("Could not open browser. Visit: {url}"));
                        }
                    } else {
                        ui::error("Daemon is not running. Start it with: openfang start");
                    }
                }
                LaunchChoice::Chat => {
                    ui::hint("Starting chat session...");
                    ui::blank();
                    crate::cmd::agent::cmd_quick_chat(None, None);
                }
            }
        }
        InitResult::Cancelled => {
            println!("  Setup cancelled.");
        }
    }
}

pub fn launch_desktop_app(_openfang_dir: &std::path::Path) {
    let desktop_bin = {
        let exe = std::env::current_exe().ok();
        let dir = exe.as_ref().and_then(|e| e.parent());

        #[cfg(windows)]
        let name = "openfang-desktop.exe";
        #[cfg(not(windows))]
        let name = "openfang-desktop";

        dir.map(|d| d.join(name))
    };

    match desktop_bin {
        Some(ref path) if path.exists() => {
            ui::success("Launching OpenFang Desktop...");
            match std::process::Command::new(path)
                .stdin(std::process::Stdio::null())
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null())
                .spawn()
            {
                Ok(_) => {
                    ui::success("Desktop app started.");
                }
                Err(e) => {
                    ui::error(&format!("Failed to launch desktop app: {e}"));
                    ui::hint("Try: openfang dashboard");
                }
            }
        }
        _ => {
            ui::error("Desktop app not found.");
            ui::hint("Install it with: cargo install openfang-desktop");
            ui::hint("Falling back to web dashboard...");
            ui::blank();
            if let Some(base) = find_daemon() {
                let url = format!("{base}/");
                if !crate::open_in_browser(&url) {
                    ui::hint(&format!("Visit: {url}"));
                }
            }
        }
    }
}

fn detect_best_provider() -> (&'static str, &'static str, &'static str) {
    let providers = provider_list();
    for (p, env_var, m, display) in &providers {
        if std::env::var(env_var).is_ok() {
            ui::success(&format!("Detected {display} ({env_var})"));
            return (p, env_var, m);
        }
    }
    if std::env::var("GOOGLE_API_KEY").is_ok() {
        ui::success("Detected Gemini (GOOGLE_API_KEY)");
        return ("gemini", "GOOGLE_API_KEY", "gemini-2.5-flash");
    }
    ui::hint("No LLM provider API keys found");
    ui::hint("Groq offers a free tier: https://console.groq.com");
    ("groq", "GROQ_API_KEY", "llama-3.3-70b-versatile")
}

fn provider_list() -> Vec<(&'static str, &'static str, &'static str, &'static str)> {
    vec![
        ("groq", "GROQ_API_KEY", "llama-3.3-70b-versatile", "Groq"),
        ("gemini", "GEMINI_API_KEY", "gemini-2.5-flash", "Gemini"),
        ("deepseek", "DEEPSEEK_API_KEY", "deepseek-chat", "DeepSeek"),
        (
            "anthropic",
            "ANTHROPIC_API_KEY",
            "claude-sonnet-4-20250514",
            "Anthropic",
        ),
        ("openai", "OPENAI_API_KEY", "gpt-4o", "OpenAI"),
        (
            "openrouter",
            "OPENROUTER_API_KEY",
            "openrouter/auto",
            "OpenRouter",
        ),
    ]
}

fn write_config_if_missing(
    openfang_dir: &std::path::Path,
    provider: &str,
    model: &str,
    api_key_env: &str,
) {
    let config_path = openfang_dir.join("config.toml");
    if config_path.exists() {
        ui::check_ok(&format!("Config already exists: {}", config_path.display()));
    } else {
        let default_config = format!(
            r#"# OpenFang Agent OS configuration
# See https://github.com/RightNow-AI/openfang for documentation

api_listen = "127.0.0.1:4200"

[default_model]
provider = "{provider}"
model = "{model}"
api_key_env = "{api_key_env}"

[memory]
decay_rate = 0.05
"#
        );
        std::fs::write(&config_path, &default_config).unwrap_or_else(|e| {
            ui::error_with_fix("Failed to write config", &e.to_string());
            std::process::exit(1);
        });
        restrict_file_permissions(&config_path);
        ui::success(&format!("Created: {}", config_path.display()));
    }
}

pub fn cmd_start(config: Option<PathBuf>, daemon: bool) {
    if let Some(base) = find_daemon() {
        ui::error_with_fix(
            &format!("Daemon already running at {base}"),
            "Use `openfang status` to check it, or stop it first",
        );
        std::process::exit(1);
    }

    if daemon {
        spawn_daemon(config);
        return;
    }

    ui::banner();
    ui::blank();
    println!("  Starting daemon...");
    ui::blank();

    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        let kernel = match OpenFangKernel::boot(config.as_deref()) {
            Ok(k) => k,
            Err(e) => {
                boot_kernel_error(&e);
                std::process::exit(1);
            }
        };

        let listen_addr = kernel.config.api_listen.clone();
        let daemon_info_path = kernel.config.home_dir.join("daemon.json");
        let provider = kernel.config.default_model.provider.clone();
        let model = kernel.config.default_model.model.clone();
        let agent_count = kernel.registry.count();
        let model_count = kernel
            .model_catalog
            .read()
            .map(|c| c.list_models().len())
            .unwrap_or(0);

        ui::success(&format!("Kernel booted ({provider}/{model})"));
        if model_count > 0 {
            ui::success(&format!("{model_count} models available"));
        }
        if agent_count > 0 {
            ui::success(&format!("{agent_count} agent(s) loaded"));
        }
        ui::blank();
        ui::kv("API", &format!("http://{listen_addr}"));
        ui::kv("Dashboard", &format!("http://{listen_addr}/"));
        ui::kv("Provider", &provider);
        ui::kv("Model", &model);
        ui::blank();
        ui::hint("Open the dashboard in your browser, or run `openfang chat`");
        ui::hint("Press Ctrl+C to stop the daemon");
        ui::blank();

        if let Err(e) =
            openfang_api::server::run_daemon(kernel, &listen_addr, Some(&daemon_info_path)).await
        {
            ui::error(&format!("Daemon error: {e}"));
            std::process::exit(1);
        }

        ui::blank();
        println!("  OpenFang daemon stopped.");
    });
}

pub fn cmd_restart(config: Option<PathBuf>, daemon: bool) {
    println!("  Validating configuration...");
    let cfg = openfang_kernel::config::load_config(config.as_deref());
    let warnings = cfg.validate();
    if warnings
        .iter()
        .any(|w| w.contains("invalid") || w.contains("error") || w.contains("failed"))
    {
        for w in &warnings {
            ui::error(&format!("Config validation error: {w}"));
        }
        ui::error("Restart aborted: configuration is invalid.");
        std::process::exit(1);
    }
    ui::success("Configuration is valid.");

    if find_daemon().is_some() {
        println!("  Stopping existing daemon...");
        cmd_stop();

        let mut retries = 0;
        while find_daemon().is_some() && retries < 10 {
            std::thread::sleep(std::time::Duration::from_millis(500));
            retries += 1;
        }

        if find_daemon().is_some() {
            ui::error("Daemon failed to stop. Please kill it manually.");
            std::process::exit(1);
        }
        ui::success("Daemon stopped.");
        ui::blank();
    } else {
        ui::check_warn("No running daemon found.");
    }

    cmd_start(config, daemon);
}

fn spawn_daemon(config: Option<PathBuf>) {
    use std::process::{Command, Stdio};

    let exe = std::env::current_exe().expect("Failed to get current executable path");
    let mut cmd = Command::new(exe);
    cmd.arg("start");

    if let Some(ref cfg_path) = config {
        cmd.arg("--config").arg(cfg_path);
    }

    println!("  Spawning OpenFang daemon in the background...");

    match cmd
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .stdin(Stdio::null())
        .spawn()
    {
        Ok(child) => {
            let pid = child.id();
            ui::success(&format!("Daemon spawned with PID {pid}"));
            ui::hint("Use `openfang status` or check the Dashboard to monitor it.");
        }
        Err(e) => {
            ui::error(&format!("Failed to spawn daemon: {e}"));
            std::process::exit(1);
        }
    }
}

pub fn cmd_stop() {
    match find_daemon() {
        Some(base) => {
            let client = daemon_client();
            match client.post(format!("{base}/api/shutdown")).send() {
                Ok(r) if r.status().is_success() => {
                    for _ in 0..10 {
                        std::thread::sleep(std::time::Duration::from_millis(500));
                        if find_daemon().is_none() {
                            ui::success("Daemon stopped");
                            return;
                        }
                    }
                    if let Some(home) = dirs::home_dir() {
                        let of_dir = home.join(".openfang");
                        if let Some(info) = read_daemon_info(&of_dir) {
                            force_kill_pid(info.pid);
                            let _ = std::fs::remove_file(of_dir.join("daemon.json"));
                        }
                    }
                    ui::success("Daemon stopped (forced)");
                }
                Ok(r) => {
                    ui::error(&format!("Shutdown request failed ({})", r.status()));
                }
                Err(e) => {
                    ui::error(&format!("Could not reach daemon: {e}"));
                }
            }
        }
        None => {
            ui::warn_with_fix(
                "No running daemon found",
                "Is it running? Check with: openfang status",
            );
        }
    }
}
