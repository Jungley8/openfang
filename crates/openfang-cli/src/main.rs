//! OpenFang CLI — command-line interface for the OpenFang Agent OS.
//!
//! When a daemon is running (`openfang start`), the CLI talks to it over HTTP.
//! Otherwise, commands boot an in-process kernel (single-shot mode).

mod bundled_agents;
mod cli;
mod cmd;
mod daemon;
mod dotenv;
mod launcher;
mod mcp;
pub mod progress;
pub mod table;
mod templates;
mod tracing_init;
mod tui;
mod ui;

pub(crate) use crate::daemon::{
    boot_kernel, daemon_client, daemon_json, find_daemon, restrict_dir_permissions,
    restrict_file_permissions,
};
pub(crate) use cmd::config::backup_existing_config;
pub(crate) use cmd::config::test_api_key;

use crate::cli::*;
use clap::Parser;
use openfang_api::server::read_daemon_info;
use openfang_types::agent::{AgentId, AgentManifest};
use std::io::{self, BufRead, Write};
use std::path::PathBuf;
use std::sync::atomic::AtomicBool;
#[cfg(windows)]
use std::sync::atomic::Ordering;

/// Global flag set by the Ctrl+C handler.
static CTRLC_PRESSED: AtomicBool = AtomicBool::new(false);

/// Install a Ctrl+C handler that force-exits the process.
/// On Windows/MINGW, the default handler doesn't reliably interrupt blocking
/// `read_line` calls, so we explicitly call `process::exit`.
fn install_ctrlc_handler() {
    #[cfg(windows)]
    {
        extern "system" {
            fn SetConsoleCtrlHandler(
                handler: Option<unsafe extern "system" fn(u32) -> i32>,
                add: i32,
            ) -> i32;
        }
        unsafe extern "system" fn handler(_ctrl_type: u32) -> i32 {
            if CTRLC_PRESSED.swap(true, Ordering::SeqCst) {
                // Second press: hard exit
                std::process::exit(130);
            }
            // First press: print message and exit cleanly
            let _ = std::io::Write::write_all(&mut std::io::stderr(), b"\nInterrupted.\n");
            std::process::exit(0);
        }
        unsafe { SetConsoleCtrlHandler(Some(handler), 1) };
    }

    #[cfg(not(windows))]
    {
        // On Unix, the default SIGINT handler already interrupts read_line
        // and terminates the process.
        let _ = &CTRLC_PRESSED;
    }
}

fn init_tracing_stderr() {
    tracing_init::init_tracing_stderr();
}

/// Get the OpenFang home directory, respecting OPENFANG_HOME env var.
fn cli_openfang_home() -> std::path::PathBuf {
    if let Ok(home) = std::env::var("OPENFANG_HOME") {
        return std::path::PathBuf::from(home);
    }
    dirs::home_dir()
        .unwrap_or_else(std::env::temp_dir)
        .join(".openfang")
}

/// Redirect tracing to a log file so it doesn't corrupt the ratatui TUI.
fn init_tracing_file() {
    tracing_init::init_tracing_file();
}

fn main() {
    // Load ~/.openfang/.env into process environment (system env takes priority).
    dotenv::load_dotenv();

    let cli = Cli::parse();

    // Determine if this invocation launches a ratatui TUI.
    // TUI modes must NOT install the Ctrl+C handler (it calls process::exit
    // which bypasses ratatui::restore and leaves the terminal in raw mode).
    // TUI modes also need file-based tracing (stderr output corrupts the TUI).
    let is_launcher = cli.command.is_none() && std::io::IsTerminal::is_terminal(&std::io::stdout());
    let is_tui_mode = is_launcher
        || matches!(cli.command, Some(Commands::Tui))
        || matches!(cli.command, Some(Commands::Chat { .. }))
        || matches!(
            cli.command,
            Some(Commands::Agent(AgentCommands::Chat { .. }))
        );

    if is_tui_mode {
        init_tracing_file();
    } else {
        // CLI subcommands: install Ctrl+C handler for clean interrupt of
        // blocking read_line calls, and trace to stderr.
        install_ctrlc_handler();
        init_tracing_stderr();
    }

    match cli.command {
        None => {
            if !std::io::IsTerminal::is_terminal(&std::io::stdout()) {
                // Piped: fall back to text help
                use clap::CommandFactory;
                Cli::command().print_help().unwrap();
                println!();
                return;
            }
            match launcher::run(cli.config.clone()) {
                launcher::LauncherChoice::GetStarted => cmd::init::cmd_init(false),
                launcher::LauncherChoice::Chat => cmd::agent::cmd_quick_chat(cli.config, None),
                launcher::LauncherChoice::Dashboard => cmd::system::cmd_dashboard(),
                launcher::LauncherChoice::DesktopApp => launcher::launch_desktop_app(),
                launcher::LauncherChoice::TerminalUI => tui::run(cli.config),
                launcher::LauncherChoice::ShowHelp => {
                    use clap::CommandFactory;
                    Cli::command().print_help().unwrap();
                    println!();
                }
                launcher::LauncherChoice::Quit => {}
            }
        }
        Some(Commands::Tui) => tui::run(cli.config),
        Some(Commands::Init { quick }) => cmd::init::cmd_init(quick),
        Some(Commands::Start { daemon }) => cmd::init::cmd_start(cli.config, daemon),
        Some(Commands::Restart { daemon }) => cmd::init::cmd_restart(cli.config, daemon),
        Some(Commands::Stop) => cmd::init::cmd_stop(),
        Some(Commands::Agent(sub)) => match sub {
            AgentCommands::New { template } => cmd::agent::cmd_agent_new(cli.config, template),
            AgentCommands::Spawn { manifest } => cmd::agent::cmd_agent_spawn(cli.config, manifest),
            AgentCommands::List { json } => cmd::agent::cmd_agent_list(cli.config, json),
            AgentCommands::Chat { agent_id } => cmd::agent::cmd_agent_chat(cli.config, &agent_id),
            AgentCommands::Kill { agent_id } => cmd::agent::cmd_agent_kill(cli.config, &agent_id),
            AgentCommands::Set {
                agent_id,
                field,
                value,
            } => cmd::agent::cmd_agent_set(cli.config, &agent_id, &field, &value),
        },
        Some(Commands::Workflow(sub)) => match sub {
            WorkflowCommands::List => cmd::workflow::cmd_workflow_list(),
            WorkflowCommands::Create { file } => cmd::workflow::cmd_workflow_create(file),
            WorkflowCommands::Run { workflow_id, input } => {
                cmd::workflow::cmd_workflow_run(&workflow_id, &input)
            }
        },
        Some(Commands::Workspace(sub)) => match sub {
            WorkspaceCommands::Clean { force } => {
                cmd::workflow::cmd_workspace_clean(cli.config, force)
            }
        },
        Some(Commands::Trigger(sub)) => match sub {
            TriggerCommands::List { agent_id } => {
                cmd::workflow::cmd_trigger_list(agent_id.as_deref())
            }
            TriggerCommands::Create {
                agent_id,
                pattern_json,
                prompt,
                max_fires,
            } => cmd::workflow::cmd_trigger_create(&agent_id, &pattern_json, &prompt, max_fires),
            TriggerCommands::Delete { trigger_id } => {
                cmd::workflow::cmd_trigger_delete(&trigger_id)
            }
        },
        Some(Commands::Migrate(args)) => cmd::workflow::cmd_migrate(args),
        Some(Commands::Skill(sub)) => match sub {
            SkillCommands::Install { source } => cmd::integration::cmd_skill_install(&source),
            SkillCommands::List => cmd::integration::cmd_skill_list(),
            SkillCommands::Remove { name } => cmd::integration::cmd_skill_remove(&name),
            SkillCommands::Search { query } => cmd::integration::cmd_skill_search(&query),
            SkillCommands::Create => cmd::integration::cmd_skill_create(),
        },
        Some(Commands::Channel(sub)) => match sub {
            ChannelCommands::List => cmd::integration::cmd_channel_list(),
            ChannelCommands::Setup { channel } => {
                cmd::integration::cmd_channel_setup(channel.as_deref())
            }
            ChannelCommands::Test { channel } => cmd::integration::cmd_channel_test(&channel),
            ChannelCommands::Enable { channel } => {
                cmd::integration::cmd_channel_toggle(&channel, true)
            }
            ChannelCommands::Disable { channel } => {
                cmd::integration::cmd_channel_toggle(&channel, false)
            }
        },
        Some(Commands::Hand(sub)) => match sub {
            HandCommands::List => cmd::hand::cmd_hand_list(),
            HandCommands::Active => cmd::hand::cmd_hand_active(),
            HandCommands::Install { path } => cmd::hand::cmd_hand_install(&path),
            HandCommands::Activate { id } => cmd::hand::cmd_hand_activate(&id),
            HandCommands::Deactivate { id } => cmd::hand::cmd_hand_deactivate(&id),
            HandCommands::Info { id } => cmd::hand::cmd_hand_info(&id),
            HandCommands::CheckDeps { id } => cmd::hand::cmd_hand_check_deps(&id),
            HandCommands::InstallDeps { id } => cmd::hand::cmd_hand_install_deps(&id),
            HandCommands::Pause { id } => cmd::hand::cmd_hand_pause(&id),
            HandCommands::Resume { id } => cmd::hand::cmd_hand_resume(&id),
        },
        Some(Commands::Config(sub)) => match sub {
            ConfigCommands::Show => cmd::config::cmd_config_show(),
            ConfigCommands::Edit => cmd::config::cmd_config_edit(),
            ConfigCommands::Get { key } => cmd::config::cmd_config_get(&key),
            ConfigCommands::Set { key, value } => cmd::config::cmd_config_set(&key, &value),
            ConfigCommands::Unset { key } => cmd::config::cmd_config_unset(&key),
            ConfigCommands::SetKey { provider } => cmd::config::cmd_config_set_key(&provider),
            ConfigCommands::DeleteKey { provider } => cmd::config::cmd_config_delete_key(&provider),
            ConfigCommands::TestKey { provider } => cmd::config::cmd_config_test_key(&provider),
        },
        Some(Commands::Chat { agent }) => cmd::agent::cmd_quick_chat(cli.config, agent),
        Some(Commands::Status { json }) => cmd::system::cmd_status(cli.config, json),
        Some(Commands::Doctor { json, repair }) => cmd::system::cmd_doctor(json, repair),
        Some(Commands::Dashboard) => cmd::system::cmd_dashboard(),
        Some(Commands::Completion { shell }) => cmd::system::cmd_completion(shell),
        Some(Commands::Mcp) => mcp::run_mcp_server(cli.config),
        Some(Commands::Add { name, key }) => {
            cmd::integration::cmd_integration_add(&name, key.as_deref())
        }
        Some(Commands::Remove { name }) => cmd::integration::cmd_integration_remove(&name),
        Some(Commands::Integrations { query }) => {
            cmd::integration::cmd_integrations_list(query.as_deref())
        }
        Some(Commands::Vault(sub)) => match sub {
            VaultCommands::Init => cmd::config::cmd_vault_init(),
            VaultCommands::Set { key } => cmd::config::cmd_vault_set(&key),
            VaultCommands::List => cmd::config::cmd_vault_list(),
            VaultCommands::Remove { key } => cmd::config::cmd_vault_remove(&key),
        },
        Some(Commands::New { kind }) => cmd::integration::cmd_scaffold(kind),
        // ── New commands ────────────────────────────────────────────────
        Some(Commands::Models(sub)) => match sub {
            ModelsCommands::List { provider, json } => {
                cmd::model::cmd_models_list(provider.as_deref(), json)
            }
            ModelsCommands::Aliases { json } => cmd::model::cmd_models_aliases(json),
            ModelsCommands::Providers { json } => cmd::model::cmd_models_providers(json),
            ModelsCommands::Set { model } => cmd::model::cmd_models_set(model),
        },
        Some(Commands::Gateway(sub)) => match sub {
            GatewayCommands::Start => cmd::init::cmd_start(cli.config, false),
            GatewayCommands::Stop => cmd::init::cmd_stop(),
            GatewayCommands::Status { json } => cmd::system::cmd_status(cli.config, json),
        },
        Some(Commands::Approvals(sub)) => match sub {
            ApprovalsCommands::List { json } => cmd::integration::cmd_approvals_list(json),
            ApprovalsCommands::Approve { id } => cmd::integration::cmd_approvals_respond(&id, true),
            ApprovalsCommands::Reject { id } => cmd::integration::cmd_approvals_respond(&id, false),
        },
        Some(Commands::Cron(sub)) => match sub {
            CronCommands::List { json } => cmd::integration::cmd_cron_list(json),
            CronCommands::Create {
                agent,
                spec,
                prompt,
                name,
            } => cmd::integration::cmd_cron_create(&agent, &spec, &prompt, name.as_deref()),
            CronCommands::Delete { id } => cmd::integration::cmd_cron_delete(&id),
            CronCommands::Enable { id } => cmd::integration::cmd_cron_toggle(&id, true),
            CronCommands::Disable { id } => cmd::integration::cmd_cron_toggle(&id, false),
        },
        Some(Commands::Sessions { agent, json }) => {
            cmd::system::cmd_sessions(agent.as_deref(), json)
        }
        Some(Commands::Logs { lines, follow }) => cmd::system::cmd_logs(lines, follow),
        Some(Commands::Health { json }) => cmd::system::cmd_health(json),
        Some(Commands::Security(sub)) => match sub {
            SecurityCommands::Status { json } => cmd::integration::cmd_security_status(json),
            SecurityCommands::Audit { limit, json } => {
                cmd::integration::cmd_security_audit(limit, json)
            }
            SecurityCommands::Verify => cmd::integration::cmd_security_verify(),
        },
        Some(Commands::Memory(sub)) => match sub {
            MemoryCommands::List { agent, json } => cmd::integration::cmd_memory_list(&agent, json),
            MemoryCommands::Get { agent, key, json } => {
                cmd::integration::cmd_memory_get(&agent, &key, json)
            }
            MemoryCommands::Set { agent, key, value } => {
                cmd::integration::cmd_memory_set(&agent, &key, &value)
            }
            MemoryCommands::Delete { agent, key } => {
                cmd::integration::cmd_memory_delete(&agent, &key)
            }
        },
        Some(Commands::Devices(sub)) => match sub {
            DevicesCommands::List { json } => cmd::integration::cmd_devices_list(json),
            DevicesCommands::Pair => cmd::integration::cmd_devices_pair(),
            DevicesCommands::Remove { id } => cmd::integration::cmd_devices_remove(&id),
        },
        Some(Commands::Qr) => cmd::integration::cmd_devices_pair(),
        Some(Commands::Webhooks(sub)) => match sub {
            WebhooksCommands::List { json } => cmd::integration::cmd_webhooks_list(json),
            WebhooksCommands::Create { agent, url } => {
                cmd::integration::cmd_webhooks_create(&agent, &url)
            }
            WebhooksCommands::Delete { id } => cmd::integration::cmd_webhooks_delete(&id),
            WebhooksCommands::Test { id } => cmd::integration::cmd_webhooks_test(&id),
        },
        Some(Commands::Onboard { quick }) | Some(Commands::Setup { quick }) => {
            cmd::init::cmd_init(quick)
        }
        Some(Commands::Configure) => cmd::init::cmd_init(false),
        Some(Commands::Message { agent, text, json }) => {
            cmd::agent::cmd_message(&agent, &text, json)
        }
        Some(Commands::System(sub)) => match sub {
            SystemCommands::Info { json } => cmd::system::cmd_system_info(json),
            SystemCommands::Version { json } => cmd::system::cmd_system_version(json),
        },
        Some(Commands::Reset { confirm }) => cmd::config::cmd_reset(confirm),
        Some(Commands::Uninstall {
            confirm,
            keep_config,
        }) => cmd::config::cmd_uninstall(confirm, keep_config),
        Some(Commands::Telos(sub)) => match sub {
            TelosCommands::Init { quick } => cmd::telos::cmd_telos_init(quick),
            TelosCommands::Status => cmd::telos::cmd_telos_status(),
            TelosCommands::Edit { file } => cmd::telos::cmd_telos_edit(&file),
            TelosCommands::Reload => cmd::telos::cmd_telos_reload(),
            TelosCommands::Preview { hand } => cmd::telos::cmd_telos_preview(&hand),
            TelosCommands::Export { output } => cmd::telos::cmd_telos_export(output.as_deref()),
            TelosCommands::Report { days } => cmd::telos::cmd_telos_report(days),
        },
    }
}

// ---------------------------------------------------------------------------
// Commands
// ---------------------------------------------------------------------------

fn cmd_init(quick: bool) {
    let home = match dirs::home_dir() {
        Some(h) => h,
        None => {
            ui::error("Could not determine home directory");
            std::process::exit(1);
        }
    };

    let openfang_dir = cli_openfang_home();

    // --- Ensure directories exist ---
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

    // Install bundled agent templates (skips existing ones to preserve user edits)
    bundled_agents::install_bundled_agents(&openfang_dir.join("agents"));

    if quick {
        cmd_init_quick(&openfang_dir);
    } else {
        cmd_init_interactive(&openfang_dir);
    }
}

/// Quick init: no prompts, auto-detect, write config + .env, print next steps.
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

/// Interactive 5-step onboarding wizard (ratatui TUI).
fn cmd_init_interactive(openfang_dir: &std::path::Path) {
    use tui::screens::init_wizard::{self, InitResult, LaunchChoice};

    match init_wizard::run() {
        InitResult::Completed {
            provider,
            model,
            daemon_started,
            launch,
        } => {
            // Print summary after TUI restores terminal
            ui::blank();
            ui::success("OpenFang initialized!");
            ui::kv("Provider", &provider);
            ui::kv("Model", &model);

            if daemon_started {
                ui::kv_ok("Daemon", "running");
            }
            ui::blank();

            // Execute the user's chosen launch action.
            match launch {
                LaunchChoice::Desktop => {
                    launch_desktop_app(openfang_dir);
                }
                LaunchChoice::Dashboard => {
                    if let Some(base) = find_daemon() {
                        let url = format!("{base}/");
                        ui::success(&format!("Opening dashboard at {url}"));
                        if !open_in_browser(&url) {
                            ui::hint(&format!("Could not open browser. Visit: {url}"));
                        }
                    } else {
                        ui::error("Daemon is not running. Start it with: openfang start");
                    }
                }
                LaunchChoice::Chat => {
                    ui::hint("Starting chat session...");
                    ui::blank();
                    // Note: tracing was initialized for stderr (init is a CLI
                    // subcommand).  The chat TUI takes over the terminal with
                    // raw mode so stderr output is suppressed.  We can't
                    // reinitialize tracing (global subscriber is set once).
                    cmd::agent::cmd_quick_chat(None, None);
                }
            }
        }
        InitResult::Cancelled => {
            println!("  Setup cancelled.");
        }
    }
}

/// Launch the openfang-desktop Tauri app, connecting to the running daemon.
fn launch_desktop_app(_openfang_dir: &std::path::Path) {
    // Look for the desktop binary next to our own executable.
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
                if !open_in_browser(&url) {
                    ui::hint(&format!("Visit: {url}"));
                }
            }
        }
    }
}

/// Auto-detect the best available provider.
fn detect_best_provider() -> (&'static str, &'static str, &'static str) {
    let providers = provider_list();

    for (p, env_var, m, display) in &providers {
        if std::env::var(env_var).is_ok() {
            ui::success(&format!("Detected {display} ({env_var})"));
            return (p, env_var, m);
        }
    }
    // Also check GOOGLE_API_KEY
    if std::env::var("GOOGLE_API_KEY").is_ok() {
        ui::success("Detected Gemini (GOOGLE_API_KEY)");
        return ("gemini", "GOOGLE_API_KEY", "gemini-2.5-flash");
    }
    ui::hint("No LLM provider API keys found");
    ui::hint("Groq offers a free tier: https://console.groq.com");
    ("groq", "GROQ_API_KEY", "llama-3.3-70b-versatile")
}

/// Static list of supported providers: (id, env_var, default_model, display_name).
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

/// Write config.toml if it doesn't already exist.
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

# For Docker, change to "0.0.0.0:4200" or set OPENFANG_LISTEN env var.
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

fn cmd_start(config: Option<PathBuf>) {
    if let Some(base) = find_daemon() {
        ui::error_with_fix(
            &format!("Daemon already running at {base}"),
            "Use `openfang status` to check it, or stop it first",
        );
        std::process::exit(1);
    }

    ui::banner();
    ui::blank();
    println!("  Starting daemon...");
    ui::blank();

    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        let kernel = boot_kernel(config);

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

/// Read the api_key from ~/.openfang/config.toml (if any).
fn read_api_key() -> Option<String> {
    let config_path = cli_openfang_home().join("config.toml");
    let text = std::fs::read_to_string(config_path).ok()?;
    let table: toml::Value = text.parse().ok()?;
    let key = table.get("api_key")?.as_str()?;
    if key.is_empty() {
        None
    } else {
        Some(key.to_string())
    }
}

fn cmd_stop() {
    match find_daemon() {
        Some(base) => {
            let client = daemon_client();
            let mut req = client.post(format!("{base}/api/shutdown"));
            if let Some(key) = read_api_key() {
                req = req.bearer_auth(key);
            }
            match req.send() {
                Ok(r) if r.status().is_success() => {
                    // Wait for daemon to actually stop (up to 5 seconds)
                    for _ in 0..10 {
                        std::thread::sleep(std::time::Duration::from_millis(500));
                        if find_daemon().is_none() {
                            ui::success("Daemon stopped");
                            return;
                        }
                    }
                    // Still alive — force kill via PID
                    {
                        let of_dir = cli_openfang_home();
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

fn force_kill_pid(pid: u32) {
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

/// Show context-aware error for kernel boot failures.
fn boot_kernel_error(e: &openfang_kernel::error::KernelError) {
    let msg = e.to_string();
    if msg.contains("parse") || msg.contains("toml") || msg.contains("config") {
        ui::error_with_fix(
            "Failed to parse configuration",
            "Check your config.toml syntax: openfang config show",
        );
    } else if msg.contains("database") || msg.contains("locked") || msg.contains("sqlite") {
        ui::error_with_fix(
            "Database error (file may be locked)",
            "Check if another OpenFang process is running: openfang status",
        );
    } else if msg.contains("key") || msg.contains("API") || msg.contains("auth") {
        ui::error_with_fix(
            "LLM provider authentication failed",
            "Run `openfang doctor` to check your API key configuration",
        );
    } else {
        ui::error_with_fix(
            &format!("Failed to boot kernel: {msg}"),
            "Run `openfang doctor` to diagnose the issue",
        );
    }
}

fn cmd_agent_spawn(config: Option<PathBuf>, manifest_path: PathBuf) {
    if !manifest_path.exists() {
        ui::error_with_fix(
            &format!("Manifest file not found: {}", manifest_path.display()),
            "Use `openfang agent new` to spawn from a template instead",
        );
        std::process::exit(1);
    }

    let contents = std::fs::read_to_string(&manifest_path).unwrap_or_else(|e| {
        eprintln!("Error reading manifest: {e}");
        std::process::exit(1);
    });

    if let Some(base) = find_daemon() {
        let client = daemon_client();
        let body = daemon_json(
            client
                .post(format!("{base}/api/agents"))
                .json(&serde_json::json!({"manifest_toml": contents}))
                .send(),
        );
        if body.get("agent_id").is_some() {
            println!("Agent spawned successfully!");
            println!("  ID:   {}", body["agent_id"].as_str().unwrap_or("?"));
            println!("  Name: {}", body["name"].as_str().unwrap_or("?"));
        } else {
            eprintln!(
                "Failed to spawn agent: {}",
                body["error"].as_str().unwrap_or("Unknown error")
            );
            std::process::exit(1);
        }
    } else {
        let manifest: AgentManifest = toml::from_str(&contents).unwrap_or_else(|e| {
            eprintln!("Error parsing manifest: {e}");
            std::process::exit(1);
        });
        let kernel = boot_kernel(config);
        match kernel.spawn_agent(manifest) {
            Ok(id) => {
                println!("Agent spawned (in-process mode).");
                println!("  ID: {id}");
                println!("\n  Note: Agent will be lost when this process exits.");
                println!("  For persistent agents, use `openfang start` first.");
            }
            Err(e) => {
                eprintln!("Failed to spawn agent: {e}");
                std::process::exit(1);
            }
        }
    }
}

fn cmd_agent_list(config: Option<PathBuf>, json: bool) {
    if let Some(base) = find_daemon() {
        let client = daemon_client();
        let body = daemon_json(client.get(format!("{base}/api/agents")).send());

        if json {
            println!(
                "{}",
                serde_json::to_string_pretty(&body).unwrap_or_default()
            );
            return;
        }

        let agents = body.as_array();

        match agents {
            Some(agents) if agents.is_empty() => println!("No agents running."),
            Some(agents) => {
                println!(
                    "{:<38} {:<16} {:<10} {:<12} MODEL",
                    "ID", "NAME", "STATE", "PROVIDER"
                );
                println!("{}", "-".repeat(95));
                for a in agents {
                    println!(
                        "{:<38} {:<16} {:<10} {:<12} {}",
                        a["id"].as_str().unwrap_or("?"),
                        a["name"].as_str().unwrap_or("?"),
                        a["state"].as_str().unwrap_or("?"),
                        a["model_provider"].as_str().unwrap_or("?"),
                        a["model_name"].as_str().unwrap_or("?"),
                    );
                }
            }
            None => println!("No agents running."),
        }
    } else {
        let kernel = boot_kernel(config);
        let agents = kernel.registry.list();

        if json {
            let list: Vec<serde_json::Value> = agents
                .iter()
                .map(|e| {
                    serde_json::json!({
                        "id": e.id.to_string(),
                        "name": e.name,
                        "state": format!("{:?}", e.state),
                        "created_at": e.created_at.to_rfc3339(),
                    })
                })
                .collect();
            println!(
                "{}",
                serde_json::to_string_pretty(&list).unwrap_or_default()
            );
            return;
        }

        if agents.is_empty() {
            println!("No agents running.");
            return;
        }

        println!("{:<38} {:<20} {:<12} CREATED", "ID", "NAME", "STATE");
        println!("{}", "-".repeat(85));
        for entry in agents {
            println!(
                "{:<38} {:<20} {:<12} {}",
                entry.id,
                entry.name,
                format!("{:?}", entry.state),
                entry.created_at.format("%Y-%m-%d %H:%M")
            );
        }
    }
}

fn cmd_agent_chat(config: Option<PathBuf>, agent_id_str: &str) {
    tui::chat_runner::run_chat_tui(config, Some(agent_id_str.to_string()));
}

fn cmd_agent_kill(config: Option<PathBuf>, agent_id_str: &str) {
    if let Some(base) = find_daemon() {
        let client = daemon_client();
        let body = daemon_json(
            client
                .delete(format!("{base}/api/agents/{agent_id_str}"))
                .send(),
        );
        if body.get("status").is_some() {
            println!("Agent {agent_id_str} killed.");
        } else {
            eprintln!(
                "Failed to kill agent: {}",
                body["error"].as_str().unwrap_or("Unknown error")
            );
            std::process::exit(1);
        }
    } else {
        let agent_id: AgentId = agent_id_str.parse().unwrap_or_else(|_| {
            eprintln!("Invalid agent ID: {agent_id_str}");
            std::process::exit(1);
        });
        let kernel = boot_kernel(config);
        match kernel.kill_agent(agent_id) {
            Ok(()) => println!("Agent {agent_id} killed."),
            Err(e) => {
                eprintln!("Failed to kill agent: {e}");
                std::process::exit(1);
            }
        }
    }
}

fn cmd_agent_set(agent_id_str: &str, field: &str, value: &str) {
    match field {
        "model" => {
            if let Some(base) = find_daemon() {
                let client = daemon_client();
                let body = daemon_json(
                    client
                        .put(format!("{base}/api/agents/{agent_id_str}/model"))
                        .json(&serde_json::json!({"model": value}))
                        .send(),
                );
                if body.get("status").is_some() {
                    println!("Agent {agent_id_str} model set to {value}.");
                } else {
                    eprintln!(
                        "Failed to set model: {}",
                        body["error"].as_str().unwrap_or("Unknown error")
                    );
                    std::process::exit(1);
                }
            } else {
                eprintln!("No running daemon found. Start one with: openfang start");
                std::process::exit(1);
            }
        }
        _ => {
            eprintln!("Unknown field: {field}. Supported fields: model");
            std::process::exit(1);
        }
    }
}

fn cmd_agent_new(config: Option<PathBuf>, template_name: Option<String>) {
    let all_templates = templates::load_all_templates();
    if all_templates.is_empty() {
        ui::error_with_fix(
            "No agent templates found",
            "Run `openfang init` to set up the agents directory",
        );
        std::process::exit(1);
    }

    // Resolve template: by name or interactive picker
    let chosen = match template_name {
        Some(ref name) => match all_templates.iter().find(|t| t.name == *name) {
            Some(t) => t,
            None => {
                ui::error_with_fix(
                    &format!("Template '{name}' not found"),
                    "Run `openfang agent new` to see available templates",
                );
                std::process::exit(1);
            }
        },
        None => {
            ui::section("Available Agent Templates");
            ui::blank();
            for (i, t) in all_templates.iter().enumerate() {
                let desc = if t.description.is_empty() {
                    String::new()
                } else {
                    format!("  {}", t.description)
                };
                println!(
                    "    {:>2}. {:<22}{}",
                    i + 1,
                    t.name,
                    colored::Colorize::dimmed(desc.as_str())
                );
            }
            ui::blank();
            let choice = prompt_input("  Choose template [1]: ");
            let idx = if choice.is_empty() {
                0
            } else {
                choice
                    .parse::<usize>()
                    .unwrap_or(1)
                    .saturating_sub(1)
                    .min(all_templates.len() - 1)
            };
            &all_templates[idx]
        }
    };

    // Spawn the agent
    spawn_template_agent(config, chosen);
}

/// Spawn an agent from a template, via daemon or in-process.
fn spawn_template_agent(config: Option<PathBuf>, template: &templates::AgentTemplate) {
    if let Some(base) = find_daemon() {
        let client = daemon_client();
        let body = daemon_json(
            client
                .post(format!("{base}/api/agents"))
                .json(&serde_json::json!({"manifest_toml": template.content}))
                .send(),
        );
        if let Some(id) = body["agent_id"].as_str() {
            ui::blank();
            ui::success(&format!("Agent '{}' spawned", template.name));
            ui::kv("ID", id);
            if let Some(model) = body["model_name"].as_str() {
                let provider = body["model_provider"].as_str().unwrap_or("?");
                ui::kv("Model", &format!("{provider}/{model}"));
            }
            ui::blank();
            ui::hint(&format!("Chat: openfang chat {}", template.name));
        } else {
            ui::error(&format!(
                "Failed to spawn: {}",
                body["error"].as_str().unwrap_or("Unknown error")
            ));
            std::process::exit(1);
        }
    } else {
        let manifest: AgentManifest = toml::from_str(&template.content).unwrap_or_else(|e| {
            ui::error_with_fix(
                &format!("Failed to parse template '{}': {e}", template.name),
                "The template manifest may be corrupted",
            );
            std::process::exit(1);
        });
        let kernel = boot_kernel(config);
        match kernel.spawn_agent(manifest) {
            Ok(id) => {
                ui::blank();
                ui::success(&format!("Agent '{}' spawned (in-process)", template.name));
                ui::kv("ID", &id.to_string());
                ui::blank();
                ui::hint(&format!("Chat: openfang chat {}", template.name));
                ui::hint("Note: Agent will be lost when this process exits");
                ui::hint("For persistent agents, use `openfang start` first");
            }
            Err(e) => {
                ui::error(&format!("Failed to spawn agent: {e}"));
                std::process::exit(1);
            }
        }
    }
}

fn cmd_status(config: Option<PathBuf>, json: bool) {
    if let Some(base) = find_daemon() {
        let client = daemon_client();
        let body = daemon_json(client.get(format!("{base}/api/status")).send());

        if json {
            println!(
                "{}",
                serde_json::to_string_pretty(&body).unwrap_or_default()
            );
            return;
        }

        ui::section("OpenFang Daemon Status");
        ui::blank();
        ui::kv_ok("Status", body["status"].as_str().unwrap_or("?"));
        ui::kv(
            "Agents",
            &body["agent_count"].as_u64().unwrap_or(0).to_string(),
        );
        ui::kv("Provider", body["default_provider"].as_str().unwrap_or("?"));
        ui::kv("Model", body["default_model"].as_str().unwrap_or("?"));
        ui::kv("API", &base);
        ui::kv("Dashboard", &format!("{base}/"));
        ui::kv("Data dir", body["data_dir"].as_str().unwrap_or("?"));
        ui::kv(
            "Uptime",
            &format!("{}s", body["uptime_seconds"].as_u64().unwrap_or(0)),
        );

        if let Some(agents) = body["agents"].as_array() {
            if !agents.is_empty() {
                ui::blank();
                ui::section("Active Agents");
                for a in agents {
                    println!(
                        "    {} ({}) -- {} [{}:{}]",
                        a["name"].as_str().unwrap_or("?"),
                        a["id"].as_str().unwrap_or("?"),
                        a["state"].as_str().unwrap_or("?"),
                        a["model_provider"].as_str().unwrap_or("?"),
                        a["model_name"].as_str().unwrap_or("?"),
                    );
                }
            }
        }
    } else {
        let kernel = boot_kernel(config);
        let agent_count = kernel.registry.count();

        if json {
            println!(
                "{}",
                serde_json::to_string_pretty(&serde_json::json!({
                    "status": "in-process",
                    "agent_count": agent_count,
                    "data_dir": kernel.config.data_dir.display().to_string(),
                    "default_provider": kernel.config.default_model.provider,
                    "default_model": kernel.config.default_model.model,
                    "daemon": false,
                }))
                .unwrap_or_default()
            );
            return;
        }

        ui::section("OpenFang Status (in-process)");
        ui::blank();
        ui::kv("Agents", &agent_count.to_string());
        ui::kv("Provider", &kernel.config.default_model.provider);
        ui::kv("Model", &kernel.config.default_model.model);
        ui::kv("Data dir", &kernel.config.data_dir.display().to_string());
        ui::kv_warn("Daemon", "NOT RUNNING");
        ui::blank();
        ui::hint("Run `openfang start` to launch the daemon");

        if agent_count > 0 {
            ui::blank();
            ui::section("Persisted Agents");
            for entry in kernel.registry.list() {
                println!("    {} ({}) -- {:?}", entry.name, entry.id, entry.state);
            }
        }
    }
}

fn cmd_doctor(json: bool, repair: bool) {
    let mut checks: Vec<serde_json::Value> = Vec::new();
    let mut all_ok = true;
    let mut repaired = false;

    if !json {
        ui::step("OpenFang Doctor");
        println!();
    }

    let home = dirs::home_dir();
    if let Some(_h) = &home {
        let openfang_dir = cli_openfang_home();

        // --- Check 1: OpenFang directory ---
        if openfang_dir.exists() {
            if !json {
                ui::check_ok(&format!("OpenFang directory: {}", openfang_dir.display()));
            }
            checks.push(serde_json::json!({"check": "openfang_dir", "status": "ok", "path": openfang_dir.display().to_string()}));
        } else if repair {
            if !json {
                ui::check_fail("OpenFang directory not found.");
            }
            let answer = prompt_input("    Create it now? [Y/n] ");
            if answer.is_empty() || answer.starts_with('y') || answer.starts_with('Y') {
                if std::fs::create_dir_all(&openfang_dir).is_ok() {
                    restrict_dir_permissions(&openfang_dir);
                    for sub in ["data", "agents"] {
                        let _ = std::fs::create_dir_all(openfang_dir.join(sub));
                    }
                    if !json {
                        ui::check_ok("Created OpenFang directory");
                    }
                    repaired = true;
                } else {
                    if !json {
                        ui::check_fail("Failed to create directory");
                    }
                    all_ok = false;
                }
            } else {
                all_ok = false;
            }
            checks.push(serde_json::json!({"check": "openfang_dir", "status": if repaired { "repaired" } else { "fail" }}));
        } else {
            if !json {
                ui::check_fail("OpenFang directory not found. Run `openfang init` first.");
            }
            checks.push(serde_json::json!({"check": "openfang_dir", "status": "fail"}));
            all_ok = false;
        }

        // --- Check 2: .env file exists + permissions ---
        let env_path = openfang_dir.join(".env");
        if env_path.exists() {
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                if let Ok(meta) = std::fs::metadata(&env_path) {
                    let mode = meta.permissions().mode() & 0o777;
                    if mode == 0o600 {
                        if !json {
                            ui::check_ok(".env file (permissions OK)");
                        }
                    } else if repair {
                        let _ = std::fs::set_permissions(
                            &env_path,
                            std::fs::Permissions::from_mode(0o600),
                        );
                        if !json {
                            ui::check_ok(".env file (permissions fixed to 0600)");
                        }
                        repaired = true;
                    } else {
                        if !json {
                            ui::check_warn(&format!(
                                ".env file has loose permissions ({:o}), should be 0600",
                                mode
                            ));
                        }
                    }
                } else {
                    if !json {
                        ui::check_ok(".env file");
                    }
                }
            }
            #[cfg(not(unix))]
            {
                if !json {
                    ui::check_ok(".env file");
                }
            }
            checks.push(serde_json::json!({"check": "env_file", "status": "ok"}));
        } else {
            if !json {
                ui::check_warn(
                    ".env file not found (create with: openfang config set-key <provider>)",
                );
            }
            checks.push(serde_json::json!({"check": "env_file", "status": "warn"}));
        }

        // --- Check 3: Config TOML syntax validation ---
        let config_path = openfang_dir.join("config.toml");
        if config_path.exists() {
            let config_content = std::fs::read_to_string(&config_path).unwrap_or_default();
            match toml::from_str::<toml::Value>(&config_content) {
                Ok(_) => {
                    if !json {
                        ui::check_ok(&format!("Config file: {}", config_path.display()));
                    }
                    checks.push(serde_json::json!({"check": "config_file", "status": "ok"}));
                }
                Err(e) => {
                    if !json {
                        ui::check_fail(&format!("Config file has syntax errors: {e}"));
                        ui::hint("Fix with: openfang config edit");
                    }
                    checks.push(serde_json::json!({"check": "config_syntax", "status": "fail", "error": e.to_string()}));
                    all_ok = false;
                }
            }
        } else if repair {
            if !json {
                ui::check_fail("Config file not found.");
            }
            let answer = prompt_input("    Create default config? [Y/n] ");
            if answer.is_empty() || answer.starts_with('y') || answer.starts_with('Y') {
                let default_config = r#"# OpenFang Agent OS configuration
# See https://github.com/RightNow-AI/openfang for documentation

# For Docker, change to "0.0.0.0:4200" or set OPENFANG_LISTEN env var.
api_listen = "127.0.0.1:4200"

[default_model]
provider = "groq"
model = "llama-3.3-70b-versatile"
api_key_env = "GROQ_API_KEY"

[memory]
decay_rate = 0.05
"#;
                let _ = std::fs::create_dir_all(&openfang_dir);
                if std::fs::write(&config_path, default_config).is_ok() {
                    restrict_file_permissions(&config_path);
                    if !json {
                        ui::check_ok("Created default config.toml");
                    }
                    repaired = true;
                } else {
                    if !json {
                        ui::check_fail("Failed to create config.toml");
                    }
                    all_ok = false;
                }
            } else {
                all_ok = false;
            }
            checks.push(serde_json::json!({"check": "config_file", "status": if repaired { "repaired" } else { "fail" }}));
        } else {
            if !json {
                ui::check_fail("Config file not found.");
            }
            checks.push(serde_json::json!({"check": "config_file", "status": "fail"}));
            all_ok = false;
        }

        // --- Check 4: Port availability ---
        // Read api_listen from config (default: 127.0.0.1:4200)
        let api_listen = {
            let cfg_path = openfang_dir.join("config.toml");
            if cfg_path.exists() {
                std::fs::read_to_string(&cfg_path)
                    .ok()
                    .and_then(|s| toml::from_str::<openfang_types::config::KernelConfig>(&s).ok())
                    .map(|c| c.api_listen)
                    .unwrap_or_else(|| "127.0.0.1:4200".to_string())
            } else {
                "127.0.0.1:4200".to_string()
            }
        };
        if !json {
            println!();
        }
        let daemon_running = find_daemon();
        if let Some(ref base) = daemon_running {
            if !json {
                ui::check_ok(&format!("Daemon running at {base}"));
            }
            checks.push(serde_json::json!({"check": "daemon", "status": "ok", "url": base}));
        } else {
            if !json {
                ui::check_warn("Daemon not running (start with `openfang start`)");
            }
            checks.push(serde_json::json!({"check": "daemon", "status": "warn"}));

            // Check if the configured port is available
            let bind_addr = if api_listen.starts_with("0.0.0.0") {
                api_listen.replacen("0.0.0.0", "127.0.0.1", 1)
            } else {
                api_listen.clone()
            };
            match std::net::TcpListener::bind(&bind_addr) {
                Ok(_) => {
                    if !json {
                        ui::check_ok(&format!("Port {api_listen} is available"));
                    }
                    checks.push(
                        serde_json::json!({"check": "port", "status": "ok", "address": api_listen}),
                    );
                }
                Err(_) => {
                    if !json {
                        ui::check_warn(&format!("Port {api_listen} is in use by another process"));
                    }
                    checks.push(serde_json::json!({"check": "port", "status": "warn", "address": api_listen}));
                }
            }
        }

        // --- Check 5: Stale daemon.json ---
        let daemon_json_path = openfang_dir.join("daemon.json");
        if daemon_json_path.exists() && daemon_running.is_none() {
            if repair {
                let _ = std::fs::remove_file(&daemon_json_path);
                if !json {
                    ui::check_ok("Removed stale daemon.json");
                }
                repaired = true;
            } else if !json {
                ui::check_warn(
                    "Stale daemon.json found (daemon not running). Run with --repair to clean up.",
                );
            }
            checks.push(serde_json::json!({"check": "stale_daemon_json", "status": if repair { "repaired" } else { "warn" }}));
        }

        // --- Check 6: Database file ---
        let db_path = openfang_dir.join("data").join("openfang.db");
        if db_path.exists() {
            // Quick SQLite magic bytes check
            if let Ok(bytes) = std::fs::read(&db_path) {
                if bytes.len() >= 16 && bytes.starts_with(b"SQLite format 3") {
                    if !json {
                        ui::check_ok("Database file (valid SQLite)");
                    }
                    checks.push(serde_json::json!({"check": "database", "status": "ok"}));
                } else {
                    if !json {
                        ui::check_fail("Database file exists but is not valid SQLite");
                    }
                    checks.push(serde_json::json!({"check": "database", "status": "fail"}));
                    all_ok = false;
                }
            }
        } else {
            if !json {
                ui::check_warn("No database file (will be created on first run)");
            }
            checks.push(serde_json::json!({"check": "database", "status": "warn"}));
        }

        // --- Check 7: Disk space ---
        #[cfg(unix)]
        {
            if let Ok(output) = std::process::Command::new("df")
                .args(["-m", &openfang_dir.display().to_string()])
                .output()
            {
                let stdout = String::from_utf8_lossy(&output.stdout);
                // Parse the available MB from df output (4th column of 2nd line)
                if let Some(line) = stdout.lines().nth(1) {
                    let cols: Vec<&str> = line.split_whitespace().collect();
                    if cols.len() >= 4 {
                        if let Ok(available_mb) = cols[3].parse::<u64>() {
                            if available_mb < 100 {
                                if !json {
                                    ui::check_warn(&format!(
                                        "Low disk space: {available_mb}MB available"
                                    ));
                                }
                                checks.push(serde_json::json!({"check": "disk_space", "status": "warn", "available_mb": available_mb}));
                            } else {
                                if !json {
                                    ui::check_ok(&format!(
                                        "Disk space: {available_mb}MB available"
                                    ));
                                }
                                checks.push(serde_json::json!({"check": "disk_space", "status": "ok", "available_mb": available_mb}));
                            }
                        }
                    }
                }
            }
        }

        // --- Check 8: Agent manifests parse correctly ---
        let agents_dir = openfang_dir.join("agents");
        if agents_dir.exists() {
            let mut agent_errors = Vec::new();
            if let Ok(entries) = std::fs::read_dir(&agents_dir) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.extension().and_then(|e| e.to_str()) == Some("toml") {
                        if let Ok(content) = std::fs::read_to_string(&path) {
                            if let Err(e) = toml::from_str::<AgentManifest>(&content) {
                                agent_errors.push((
                                    path.file_name()
                                        .unwrap_or_default()
                                        .to_string_lossy()
                                        .to_string(),
                                    e.to_string(),
                                ));
                            }
                        }
                    }
                }
            }
            if agent_errors.is_empty() {
                if !json {
                    ui::check_ok("Agent manifests are valid");
                }
                checks.push(serde_json::json!({"check": "agent_manifests", "status": "ok"}));
            } else {
                for (file, err) in &agent_errors {
                    if !json {
                        ui::check_fail(&format!("Invalid manifest {file}: {err}"));
                    }
                }
                checks.push(serde_json::json!({"check": "agent_manifests", "status": "fail", "errors": agent_errors.len()}));
                all_ok = false;
            }
        }
    } else {
        if !json {
            ui::check_fail("Could not determine home directory");
        }
        checks.push(serde_json::json!({"check": "home_dir", "status": "fail"}));
        all_ok = false;
    }

    // --- LLM providers ---
    if !json {
        println!("\n  LLM Providers:");
    }
    let provider_keys = [
        ("GROQ_API_KEY", "Groq", "groq"),
        ("OPENROUTER_API_KEY", "OpenRouter", "openrouter"),
        ("ANTHROPIC_API_KEY", "Anthropic", "anthropic"),
        ("OPENAI_API_KEY", "OpenAI", "openai"),
        ("DEEPSEEK_API_KEY", "DeepSeek", "deepseek"),
        ("GEMINI_API_KEY", "Gemini", "gemini"),
        ("GOOGLE_API_KEY", "Google", "google"),
        ("TOGETHER_API_KEY", "Together", "together"),
        ("MISTRAL_API_KEY", "Mistral", "mistral"),
        ("FIREWORKS_API_KEY", "Fireworks", "fireworks"),
    ];

    let mut any_key_set = false;
    for (env_var, name, provider_id) in &provider_keys {
        let set = std::env::var(env_var).is_ok();
        if set {
            // --- Check 9: Live key validation ---
            let valid = test_api_key(provider_id, env_var);
            if valid {
                if !json {
                    ui::provider_status(name, env_var, true);
                }
            } else if !json {
                ui::check_warn(&format!("{name} ({env_var}) - key rejected (401/403)"));
            }
            any_key_set = true;
            checks.push(serde_json::json!({"check": "provider", "name": name, "env_var": env_var, "status": if valid { "ok" } else { "warn" }, "live_test": !valid}));
        } else {
            if !json {
                ui::provider_status(name, env_var, false);
            }
            checks.push(serde_json::json!({"check": "provider", "name": name, "env_var": env_var, "status": "warn"}));
        }
    }

    if !any_key_set {
        if !json {
            println!();
            ui::check_fail("No LLM provider API keys found!");
            ui::blank();
            ui::section("Getting an API key (free tiers)");
            ui::suggest_cmd("Groq:", "https://console.groq.com       (free, fast)");
            ui::suggest_cmd("Gemini:", "https://aistudio.google.com    (free tier)");
            ui::suggest_cmd("DeepSeek:", "https://platform.deepseek.com  (low cost)");
            ui::blank();
            ui::hint("Or run: openfang config set-key groq");
        }
        all_ok = false;
    }

    // --- Check 10: Channel token format validation ---
    if !json {
        println!("\n  Channel Integrations:");
    }
    let channel_keys = [
        ("TELEGRAM_BOT_TOKEN", "Telegram"),
        ("DISCORD_BOT_TOKEN", "Discord"),
        ("SLACK_APP_TOKEN", "Slack App"),
        ("SLACK_BOT_TOKEN", "Slack Bot"),
    ];
    for (env_var, name) in &channel_keys {
        let set = std::env::var(env_var).is_ok();
        if set {
            // Format validation
            let val = std::env::var(env_var).unwrap_or_default();
            let format_ok = match *env_var {
                "TELEGRAM_BOT_TOKEN" => val.contains(':'), // Telegram tokens have format "123456:ABC-DEF..."
                "DISCORD_BOT_TOKEN" => val.len() > 50,     // Discord tokens are typically 59+ chars
                "SLACK_APP_TOKEN" => val.starts_with("xapp-"),
                "SLACK_BOT_TOKEN" => val.starts_with("xoxb-"),
                _ => true,
            };
            if format_ok {
                if !json {
                    ui::provider_status(name, env_var, true);
                }
            } else if !json {
                ui::check_warn(&format!("{name} ({env_var}) - unexpected token format"));
            }
            checks.push(serde_json::json!({"check": "channel", "name": name, "env_var": env_var, "status": if format_ok { "ok" } else { "warn" }}));
        } else {
            if !json {
                ui::provider_status(name, env_var, false);
            }
            checks.push(serde_json::json!({"check": "channel", "name": name, "env_var": env_var, "status": "warn"}));
        }
    }

    // --- Check 11: .env keys vs config api_key_env consistency ---
    {
        let openfang_dir = cli_openfang_home();
        let config_path = openfang_dir.join("config.toml");
        if config_path.exists() {
            let config_str = std::fs::read_to_string(&config_path).unwrap_or_default();
            // Look for api_key_env references in config
            for line in config_str.lines() {
                let trimmed = line.trim();
                if let Some(rest) = trimmed.strip_prefix("api_key_env") {
                    if let Some(val_part) = rest.strip_prefix('=') {
                        let val = val_part.trim().trim_matches('"');
                        if !val.is_empty() && std::env::var(val).is_err() {
                            if !json {
                                ui::check_warn(&format!(
                                    "Config references {val} but it is not set in env or .env"
                                ));
                            }
                            checks.push(serde_json::json!({"check": "env_consistency", "status": "warn", "missing_var": val}));
                        }
                    }
                }
            }
        }
    }

    // --- Check 12: Config deserialization into KernelConfig ---
    {
        let openfang_dir = cli_openfang_home();
        let config_path = openfang_dir.join("config.toml");
        if config_path.exists() {
            if !json {
                println!("\n  Config Validation:");
            }
            let config_content = std::fs::read_to_string(&config_path).unwrap_or_default();
            match toml::from_str::<openfang_types::config::KernelConfig>(&config_content) {
                Ok(cfg) => {
                    if !json {
                        ui::check_ok("Config deserializes into KernelConfig");
                    }
                    checks.push(serde_json::json!({"check": "config_deser", "status": "ok"}));

                    // Check exec policy
                    let mode = format!("{:?}", cfg.exec_policy.mode);
                    let safe_bins_count = cfg.exec_policy.safe_bins.len();
                    if !json {
                        ui::check_ok(&format!(
                            "Exec policy: mode={mode}, safe_bins={safe_bins_count}"
                        ));
                    }
                    checks.push(serde_json::json!({"check": "exec_policy", "status": "ok", "mode": mode, "safe_bins": safe_bins_count}));

                    // Check includes
                    if !cfg.include.is_empty() {
                        let mut include_ok = true;
                        for inc in &cfg.include {
                            let inc_path = openfang_dir.join(inc);
                            if inc_path.exists() {
                                if !json {
                                    ui::check_ok(&format!("Include file: {inc}"));
                                }
                            } else if repair {
                                if !json {
                                    ui::check_warn(&format!("Include file missing: {inc}"));
                                }
                                include_ok = false;
                            } else {
                                if !json {
                                    ui::check_fail(&format!("Include file not found: {inc}"));
                                }
                                include_ok = false;
                                all_ok = false;
                            }
                        }
                        checks.push(serde_json::json!({"check": "config_includes", "status": if include_ok { "ok" } else { "fail" }, "count": cfg.include.len()}));
                    }

                    // Check MCP server configs
                    if !cfg.mcp_servers.is_empty() {
                        let mcp_count = cfg.mcp_servers.len();
                        if !json {
                            ui::check_ok(&format!("MCP servers configured: {mcp_count}"));
                        }
                        for server in &cfg.mcp_servers {
                            // Validate transport config
                            match &server.transport {
                                openfang_types::config::McpTransportEntry::Stdio {
                                    command,
                                    ..
                                } => {
                                    if command.is_empty() {
                                        if !json {
                                            ui::check_warn(&format!(
                                                "MCP server '{}' has empty command",
                                                server.name
                                            ));
                                        }
                                        checks.push(serde_json::json!({"check": "mcp_server_config", "status": "warn", "name": server.name}));
                                    }
                                }
                                openfang_types::config::McpTransportEntry::Sse { url } => {
                                    if url.is_empty() {
                                        if !json {
                                            ui::check_warn(&format!(
                                                "MCP server '{}' has empty URL",
                                                server.name
                                            ));
                                        }
                                        checks.push(serde_json::json!({"check": "mcp_server_config", "status": "warn", "name": server.name}));
                                    }
                                }
                            }
                        }
                        checks.push(serde_json::json!({"check": "mcp_servers", "status": "ok", "count": mcp_count}));
                    }
                }
                Err(e) => {
                    if !json {
                        ui::check_fail(&format!("Config fails KernelConfig deserialization: {e}"));
                    }
                    checks.push(serde_json::json!({"check": "config_deser", "status": "fail", "error": e.to_string()}));
                    all_ok = false;
                }
            }
        }
    }

    // --- Check 13: Skill registry health ---
    {
        if !json {
            println!("\n  Skills:");
        }
        let skills_dir = cli_openfang_home().join("skills");
        let mut skill_reg = openfang_skills::registry::SkillRegistry::new(skills_dir.clone());
        skill_reg.load_bundled();
        let bundled_count = skill_reg.count();
        if !json {
            ui::check_ok(&format!("Bundled skills loaded: {bundled_count}"));
        }
        checks.push(
            serde_json::json!({"check": "bundled_skills", "status": "ok", "count": bundled_count}),
        );

        // Check workspace skills if home dir available
        if skills_dir.exists() {
            match skill_reg.load_workspace_skills(&skills_dir) {
                Ok(_) => {
                    let total = skill_reg.count();
                    let ws_count = total.saturating_sub(bundled_count);
                    if ws_count > 0 {
                        if !json {
                            ui::check_ok(&format!("Workspace skills loaded: {ws_count}"));
                        }
                        checks.push(serde_json::json!({"check": "workspace_skills", "status": "ok", "count": ws_count}));
                    }
                }
                Err(e) => {
                    if !json {
                        ui::check_warn(&format!("Failed to load workspace skills: {e}"));
                    }
                    checks.push(serde_json::json!({"check": "workspace_skills", "status": "warn", "error": e.to_string()}));
                }
            }
        }

        // Check for prompt injection issues in skill definitions
        let skills = skill_reg.list();
        let mut injection_warnings = 0;
        for skill in &skills {
            if let Some(ref prompt) = skill.manifest.prompt_context {
                let warnings = openfang_skills::verify::SkillVerifier::scan_prompt_content(prompt);
                if !warnings.is_empty() {
                    injection_warnings += 1;
                    if !json {
                        ui::check_warn(&format!(
                            "Prompt injection warning in skill: {}",
                            skill.manifest.skill.name
                        ));
                    }
                }
            }
        }
        if injection_warnings > 0 {
            checks.push(serde_json::json!({"check": "skill_injection_scan", "status": "warn", "warnings": injection_warnings}));
        } else {
            if !json {
                ui::check_ok("All skills pass prompt injection scan");
            }
            checks.push(serde_json::json!({"check": "skill_injection_scan", "status": "ok"}));
        }
    }

    // --- Check 14: Extension registry health ---
    {
        if !json {
            println!("\n  Extensions:");
        }
        let openfang_dir = cli_openfang_home();
        let mut ext_registry =
            openfang_extensions::registry::IntegrationRegistry::new(&openfang_dir);
        ext_registry.load_bundled();
        let _ = ext_registry.load_installed();
        let template_count = ext_registry.template_count();
        let installed_count = ext_registry.installed_count();
        if !json {
            ui::check_ok(&format!(
                "Available integration templates: {template_count}"
            ));
            ui::check_ok(&format!("Installed integrations: {installed_count}"));
        }
        checks.push(serde_json::json!({"check": "extensions_available", "status": "ok", "count": template_count}));
        checks.push(serde_json::json!({"check": "extensions_installed", "status": "ok", "count": installed_count}));
    }

    // --- Check 15: Daemon health detail (if running) ---
    if let Some(ref base) = find_daemon() {
        if !json {
            println!("\n  Daemon Health:");
        }
        let client = daemon_client();
        match client.get(format!("{base}/api/health/detail")).send() {
            Ok(resp) if resp.status().is_success() => {
                if let Ok(body) = resp.json::<serde_json::Value>() {
                    if let Some(agents) = body.get("agent_count").and_then(|v| v.as_u64()) {
                        if !json {
                            ui::check_ok(&format!("Running agents: {agents}"));
                        }
                        checks.push(serde_json::json!({"check": "daemon_agents", "status": "ok", "count": agents}));
                    }
                    if let Some(uptime) = body.get("uptime_secs").and_then(|v| v.as_u64()) {
                        let hours = uptime / 3600;
                        let mins = (uptime % 3600) / 60;
                        if !json {
                            ui::check_ok(&format!("Daemon uptime: {hours}h {mins}m"));
                        }
                        checks.push(serde_json::json!({"check": "daemon_uptime", "status": "ok", "secs": uptime}));
                    }
                    if let Some(db_status) = body.get("database").and_then(|v| v.as_str()) {
                        if db_status == "ok" {
                            if !json {
                                ui::check_ok("Database connectivity: OK");
                            }
                        } else {
                            if !json {
                                ui::check_fail(&format!("Database status: {db_status}"));
                            }
                            all_ok = false;
                        }
                        checks.push(serde_json::json!({"check": "daemon_db", "status": db_status}));
                    }
                }
            }
            Ok(resp) => {
                if !json {
                    ui::check_warn(&format!("Health detail returned {}", resp.status()));
                }
                checks.push(serde_json::json!({"check": "daemon_health", "status": "warn"}));
            }
            Err(e) => {
                if !json {
                    ui::check_warn(&format!("Failed to query daemon health: {e}"));
                }
                checks.push(serde_json::json!({"check": "daemon_health", "status": "warn", "error": e.to_string()}));
            }
        }

        // Check skills endpoint
        match client.get(format!("{base}/api/skills")).send() {
            Ok(resp) if resp.status().is_success() => {
                if let Ok(body) = resp.json::<serde_json::Value>() {
                    if let Some(arr) = body.as_array() {
                        if !json {
                            ui::check_ok(&format!("Skills loaded in daemon: {}", arr.len()));
                        }
                        checks.push(serde_json::json!({"check": "daemon_skills", "status": "ok", "count": arr.len()}));
                    }
                }
            }
            _ => {}
        }

        // Check MCP servers endpoint
        match client.get(format!("{base}/api/mcp/servers")).send() {
            Ok(resp) if resp.status().is_success() => {
                if let Ok(body) = resp.json::<serde_json::Value>() {
                    if let Some(arr) = body.as_array() {
                        let connected = arr
                            .iter()
                            .filter(|s| {
                                s.get("connected")
                                    .and_then(|v| v.as_bool())
                                    .unwrap_or(false)
                            })
                            .count();
                        if !json {
                            ui::check_ok(&format!(
                                "MCP servers: {} configured, {} connected",
                                arr.len(),
                                connected
                            ));
                        }
                        checks.push(serde_json::json!({"check": "daemon_mcp", "status": "ok", "configured": arr.len(), "connected": connected}));
                    }
                }
            }
            _ => {}
        }

        // Check extensions health endpoint
        match client.get(format!("{base}/api/integrations/health")).send() {
            Ok(resp) if resp.status().is_success() => {
                if let Ok(body) = resp.json::<serde_json::Value>() {
                    if let Some(obj) = body.as_object() {
                        let healthy = obj
                            .values()
                            .filter(|v| v.get("healthy").and_then(|h| h.as_bool()).unwrap_or(false))
                            .count();
                        let total = obj.len();
                        if healthy == total {
                            if !json {
                                ui::check_ok(&format!(
                                    "Integration health: {healthy}/{total} healthy"
                                ));
                            }
                        } else if !json {
                            ui::check_warn(&format!(
                                "Integration health: {healthy}/{total} healthy"
                            ));
                        }
                        checks.push(serde_json::json!({"check": "integration_health", "status": if healthy == total { "ok" } else { "warn" }, "healthy": healthy, "total": total}));
                    }
                }
            }
            _ => {}
        }
    }

    if !json {
        println!();
    }
    match std::process::Command::new("rustc")
        .arg("--version")
        .output()
    {
        Ok(output) => {
            let version = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if !json {
                ui::check_ok(&format!("Rust: {version}"));
            }
            checks.push(serde_json::json!({"check": "rust", "status": "ok", "version": version}));
        }
        Err(_) => {
            if !json {
                ui::check_fail("Rust toolchain not found");
            }
            checks.push(serde_json::json!({"check": "rust", "status": "fail"}));
            all_ok = false;
        }
    }

    // Python runtime check
    match std::process::Command::new("python3")
        .arg("--version")
        .output()
    {
        Ok(output) if output.status.success() => {
            let version = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if !json {
                ui::check_ok(&format!("Python: {version}"));
            }
            checks.push(serde_json::json!({"check": "python", "status": "ok", "version": version}));
        }
        _ => {
            // Try `python` instead
            match std::process::Command::new("python")
                .arg("--version")
                .output()
            {
                Ok(output) if output.status.success() => {
                    let version = String::from_utf8_lossy(&output.stdout).trim().to_string();
                    if !json {
                        ui::check_ok(&format!("Python: {version}"));
                    }
                    checks.push(
                        serde_json::json!({"check": "python", "status": "ok", "version": version}),
                    );
                }
                _ => {
                    if !json {
                        ui::check_warn("Python not found (needed for Python skill runtime)");
                    }
                    checks.push(serde_json::json!({"check": "python", "status": "warn"}));
                }
            }
        }
    }

    // Node.js runtime check
    match std::process::Command::new("node").arg("--version").output() {
        Ok(output) if output.status.success() => {
            let version = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if !json {
                ui::check_ok(&format!("Node.js: {version}"));
            }
            checks.push(serde_json::json!({"check": "node", "status": "ok", "version": version}));
        }
        _ => {
            if !json {
                ui::check_warn("Node.js not found (needed for Node skill runtime)");
            }
            checks.push(serde_json::json!({"check": "node", "status": "warn"}));
        }
    }

    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(&serde_json::json!({
                "all_ok": all_ok,
                "checks": checks,
            }))
            .unwrap_or_default()
        );
    } else {
        println!();
        if all_ok {
            ui::success("All checks passed! OpenFang is ready.");
            ui::hint("Start the daemon: openfang start");
        } else if repaired {
            ui::success("Repairs applied. Re-run `openfang doctor` to verify.");
        } else {
            ui::error("Some checks failed.");
            if !repair {
                ui::hint("Run `openfang doctor --repair` to attempt auto-fix");
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Dashboard command
// ---------------------------------------------------------------------------

fn cmd_dashboard() {
    let base = if let Some(url) = find_daemon() {
        url
    } else {
        // Auto-start the daemon
        ui::hint("No daemon running — starting one now...");
        match start_daemon_background() {
            Ok(url) => {
                ui::success("Daemon started");
                url
            }
            Err(e) => {
                ui::error_with_fix(
                    &format!("Could not start daemon: {e}"),
                    "Start it manually: openfang start",
                );
                std::process::exit(1);
            }
        }
    };

    let url = format!("{base}/");
    ui::success(&format!("Opening dashboard at {url}"));
    if copy_to_clipboard(&url) {
        ui::hint("URL copied to clipboard");
    }
    if !open_in_browser(&url) {
        ui::hint(&format!("Could not open browser. Visit: {url}"));
    }
}

/// Copy text to the system clipboard. Returns true on success.
pub(crate) fn copy_to_clipboard(text: &str) -> bool {
    #[cfg(target_os = "windows")]
    {
        // Use PowerShell to set clipboard (handles special characters better than cmd)
        std::process::Command::new("powershell")
            .args([
                "-NoProfile",
                "-Command",
                &format!("Set-Clipboard '{}'", text.replace('\'', "''")),
            ])
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()
            .map(|s| s.success())
            .unwrap_or(false)
    }
    #[cfg(target_os = "macos")]
    {
        use std::io::Write as IoWrite;
        std::process::Command::new("pbcopy")
            .stdin(std::process::Stdio::piped())
            .spawn()
            .and_then(|mut child| {
                if let Some(ref mut stdin) = child.stdin {
                    let _ = stdin.write_all(text.as_bytes());
                }
                child.wait()
            })
            .map(|s| s.success())
            .unwrap_or(false)
    }
    #[cfg(target_os = "linux")]
    {
        use std::io::Write as IoWrite;
        // Try xclip first, then xsel
        let result = std::process::Command::new("xclip")
            .args(["-selection", "clipboard"])
            .stdin(std::process::Stdio::piped())
            .spawn()
            .and_then(|mut child| {
                if let Some(ref mut stdin) = child.stdin {
                    let _ = stdin.write_all(text.as_bytes());
                }
                child.wait()
            })
            .map(|s| s.success())
            .unwrap_or(false);
        if result {
            return true;
        }
        std::process::Command::new("xsel")
            .args(["--clipboard", "--input"])
            .stdin(std::process::Stdio::piped())
            .spawn()
            .and_then(|mut child| {
                if let Some(ref mut stdin) = child.stdin {
                    let _ = stdin.write_all(text.as_bytes());
                }
                child.wait()
            })
            .map(|s| s.success())
            .unwrap_or(false)
    }
    #[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
    {
        let _ = text;
        false
    }
}

/// Try to open a URL in the default browser. Returns true on success.
pub(crate) fn open_in_browser(url: &str) -> bool {
    #[cfg(target_os = "windows")]
    {
        std::process::Command::new("cmd")
            .args(["/C", "start", "", url])
            .spawn()
            .is_ok()
    }
    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open").arg(url).spawn().is_ok()
    }
    #[cfg(target_os = "linux")]
    {
        std::process::Command::new("xdg-open")
            .arg(url)
            .spawn()
            .is_ok()
    }
    #[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
    {
        let _ = url;
        false
    }
}
// ---------------------------------------------------------------------------
// Background daemon start
// ---------------------------------------------------------------------------

/// Spawn `openfang start` as a detached background process.
///
/// Polls for daemon health for up to 10 seconds. Returns the daemon URL on success.
pub(crate) fn start_daemon_background() -> Result<String, String> {
    let exe = std::env::current_exe().map_err(|e| format!("Cannot find executable: {e}"))?;

    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const DETACHED_PROCESS: u32 = 0x00000008;
        const CREATE_NEW_PROCESS_GROUP: u32 = 0x00000200;
        std::process::Command::new(&exe)
            .arg("start")
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .creation_flags(DETACHED_PROCESS | CREATE_NEW_PROCESS_GROUP)
            .spawn()
            .map_err(|e| format!("Failed to spawn daemon: {e}"))?;
    }

    #[cfg(not(windows))]
    {
        std::process::Command::new(&exe)
            .arg("start")
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn()
            .map_err(|e| format!("Failed to spawn daemon: {e}"))?;
    }

    // Poll for daemon readiness
    for _ in 0..20 {
        std::thread::sleep(std::time::Duration::from_millis(500));
        if let Some(url) = crate::daemon::find_daemon() {
            return Ok(url);
        }
    }

    Err("Daemon did not become ready within 10 seconds".to_string())
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

pub(crate) fn openfang_home() -> PathBuf {
    if let Ok(home) = std::env::var("OPENFANG_HOME") {
        return PathBuf::from(home);
    }
    dirs::home_dir()
        .unwrap_or_else(|| {
            eprintln!("Error: Could not determine home directory");
            std::process::exit(1);
        })
        .join(".openfang")
}

pub(crate) fn prompt_input(prompt: &str) -> String {
    print!("{prompt}");
    io::stdout().flush().unwrap();
    let mut line = String::new();
    io::stdin().lock().read_line(&mut line).unwrap_or(0);
    line.trim().to_string()
}

pub(crate) fn copy_dir_recursive(src: &PathBuf, dst: &PathBuf) {
    std::fs::create_dir_all(dst).unwrap();
    if let Ok(entries) = std::fs::read_dir(src) {
        for entry in entries.flatten() {
            let path = entry.path();
            let dest_path = dst.join(entry.file_name());
            if path.is_dir() {
                copy_dir_recursive(&path, &dest_path);
            } else {
                let _ = std::fs::copy(&path, &dest_path);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Cli;
    use crate::daemon::parse_api_key_from_config_toml;
    use clap::Parser;

    // --- Doctor command unit tests ---

    #[test]
    fn test_agent_set_model_cli_parse() {
        let cli = Cli::try_parse_from([
            "openfang",
            "agent",
            "set",
            "123e4567-e89b-12d3-a456-426614174000",
            "model",
            "gpt-4o",
        ])
        .expect("agent set model syntax should parse");
        assert!(matches!(
            cli.command,
            Some(super::Commands::Agent(super::AgentCommands::Set {
                ref agent_id,
                ref field,
                ref value,
            })) if agent_id == "123e4567-e89b-12d3-a456-426614174000" && field == "model" && value == "gpt-4o"
        ));
    }

    #[test]
    fn test_doctor_skill_registry_loads_bundled() {
        let skills_dir = std::env::temp_dir().join("openfang-doctor-test-skills");
        let mut skill_reg = openfang_skills::registry::SkillRegistry::new(skills_dir);
        let count = skill_reg.load_bundled();
        assert!(count > 0, "Should load bundled skills");
        assert_eq!(skill_reg.count(), count);
    }

    #[test]
    fn test_doctor_extension_registry_loads_bundled() {
        let tmp = std::env::temp_dir().join("openfang-doctor-test-ext");
        let _ = std::fs::create_dir_all(&tmp);
        let mut ext_reg = openfang_extensions::registry::IntegrationRegistry::new(&tmp);
        let count = ext_reg.load_bundled();
        assert!(count > 0, "Should load bundled integration templates");
        assert_eq!(ext_reg.template_count(), count);
    }

    #[test]
    fn test_doctor_config_deser_default() {
        // Default KernelConfig should serialize/deserialize round-trip
        let config = openfang_types::config::KernelConfig::default();
        let toml_str = toml::to_string_pretty(&config).unwrap();
        let parsed: openfang_types::config::KernelConfig = toml::from_str(&toml_str).unwrap();
        assert_eq!(parsed.api_listen, config.api_listen);
    }

    #[test]
    fn test_doctor_config_include_field() {
        let config_toml = r#"
api_listen = "127.0.0.1:4200"
include = ["providers.toml", "agents.toml"]

[default_model]
provider = "groq"
model = "llama-3.3-70b-versatile"
api_key_env = "GROQ_API_KEY"
"#;
        let config: openfang_types::config::KernelConfig = toml::from_str(config_toml).unwrap();
        assert_eq!(config.include.len(), 2);
        assert_eq!(config.include[0], "providers.toml");
        assert_eq!(config.include[1], "agents.toml");
    }

    #[test]
    fn test_doctor_exec_policy_field() {
        let config_toml = r#"
api_listen = "127.0.0.1:4200"

[exec_policy]
mode = "allowlist"
safe_bins = ["ls", "cat", "echo"]
timeout_secs = 30

[default_model]
provider = "groq"
model = "llama-3.3-70b-versatile"
api_key_env = "GROQ_API_KEY"
"#;
        let config: openfang_types::config::KernelConfig = toml::from_str(config_toml).unwrap();
        assert_eq!(
            config.exec_policy.mode,
            openfang_types::config::ExecSecurityMode::Allowlist
        );
        assert_eq!(config.exec_policy.safe_bins.len(), 3);
        assert_eq!(config.exec_policy.timeout_secs, 30);
    }

    #[test]
    fn test_doctor_mcp_transport_validation() {
        let config_toml = r#"
api_listen = "127.0.0.1:4200"

[default_model]
provider = "groq"
model = "llama-3.3-70b-versatile"
api_key_env = "GROQ_API_KEY"

[[mcp_servers]]
name = "github"
timeout_secs = 30

[mcp_servers.transport]
type = "stdio"
command = "npx"
args = ["-y", "@modelcontextprotocol/server-github"]
"#;
        let config: openfang_types::config::KernelConfig = toml::from_str(config_toml).unwrap();
        assert_eq!(config.mcp_servers.len(), 1);
        assert_eq!(config.mcp_servers[0].name, "github");
        match &config.mcp_servers[0].transport {
            openfang_types::config::McpTransportEntry::Stdio { command, args } => {
                assert_eq!(command, "npx");
                assert_eq!(args.len(), 2);
            }
            _ => panic!("Expected Stdio transport"),
        }
    }

    #[test]
    fn test_doctor_skill_injection_scan_clean() {
        let clean_content = "This is a normal skill prompt with helpful instructions.";
        let warnings = openfang_skills::verify::SkillVerifier::scan_prompt_content(clean_content);
        assert!(warnings.is_empty(), "Clean content should have no warnings");
    }

    #[test]
    fn test_doctor_hook_event_variants() {
        // Verify all 4 hook event types are constructable
        use openfang_types::agent::HookEvent;
        let events = [
            HookEvent::BeforeToolCall,
            HookEvent::AfterToolCall,
            HookEvent::BeforePromptBuild,
            HookEvent::AgentLoopEnd,
        ];
        assert_eq!(events.len(), 4);
    }

    #[test]
    fn test_parse_api_key_from_config_toml_present() {
        let config = r#"
api_listen = "127.0.0.1:4200"
api_key = "test-secret"

[default_model]
provider = "groq"
model = "llama-3.3-70b-versatile"
api_key_env = "GROQ_API_KEY"
"#;
        let parsed = parse_api_key_from_config_toml(config);
        assert_eq!(parsed.as_deref(), Some("test-secret"));
    }

    #[test]
    fn test_parse_api_key_from_config_toml_empty_or_missing() {
        let with_empty = r#"
api_listen = "127.0.0.1:4200"
api_key = ""

[default_model]
provider = "groq"
model = "llama-3.3-70b-versatile"
api_key_env = "GROQ_API_KEY"
"#;
        let missing = r#"
api_listen = "127.0.0.1:4200"

[default_model]
provider = "groq"
model = "llama-3.3-70b-versatile"
api_key_env = "GROQ_API_KEY"
"#;
        assert_eq!(parse_api_key_from_config_toml(with_empty), None);
        assert_eq!(parse_api_key_from_config_toml(missing), None);
    }

    #[test]
    fn test_parse_api_key_from_config_toml_trims_value() {
        let config = r#"
api_listen = "127.0.0.1:4200"
api_key = "  test-secret  "
"#;
        assert_eq!(
            parse_api_key_from_config_toml(config).as_deref(),
            Some("test-secret")
        );
    }

    #[test]
    fn test_parse_api_key_from_config_toml_invalid_toml() {
        let invalid = r#"
api_listen = "127.0.0.1:4200"
api_key = "test-secret
"#;
        assert_eq!(parse_api_key_from_config_toml(invalid), None);
    }

    #[test]
    fn test_uninstall_path_line_filter() {
        use crate::cmd::config::is_openfang_path_line;
        let dir = "/home/user/.openfang/bin";
        assert!(is_openfang_path_line(
            r#"export PATH="$HOME/.openfang/bin:$PATH""#,
            dir
        ));
        assert!(is_openfang_path_line(
            r#"export PATH="/home/user/.openfang/bin:$PATH""#,
            dir
        ));
        assert!(is_openfang_path_line(
            "set -gx PATH $HOME/.openfang/bin $PATH",
            dir
        ));
        assert!(is_openfang_path_line(
            "fish_add_path $HOME/.openfang/bin",
            dir
        ));
        assert!(!is_openfang_path_line(
            r#"export PATH="$HOME/.cargo/bin:$PATH""#,
            dir
        ));
        assert!(!is_openfang_path_line(
            r#"export PATH="/usr/local/bin:$PATH""#,
            dir
        ));
        assert!(!is_openfang_path_line("# openfang config", dir));
        assert!(!is_openfang_path_line("alias of=openfang", dir));
    }
}
