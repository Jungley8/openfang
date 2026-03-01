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
mod tui;
mod ui;

pub(crate) use crate::daemon::{
    daemon_client, daemon_json, find_daemon, restrict_dir_permissions, restrict_file_permissions,
};
pub(crate) use cmd::config::backup_existing_config;
pub(crate) use cmd::config::test_api_key;

use crate::cli::*;
use clap::Parser;
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
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();
}

/// Redirect tracing to a log file so it doesn't corrupt the ratatui TUI.
fn init_tracing_file() {
    let log_dir = dirs::home_dir()
        .map(|h| h.join(".openfang"))
        .unwrap_or_else(|| std::path::PathBuf::from("."));
    let _ = std::fs::create_dir_all(&log_dir);
    let log_path = log_dir.join("tui.log");

    match std::fs::File::create(&log_path) {
        Ok(file) => {
            tracing_subscriber::fmt()
                .with_env_filter(
                    tracing_subscriber::EnvFilter::try_from_default_env()
                        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
                )
                .with_writer(std::sync::Mutex::new(file))
                .with_ansi(false)
                .init();
        }
        Err(_) => {
            // Fallback: suppress all output rather than corrupt the TUI
            tracing_subscriber::fmt()
                .with_max_level(tracing::Level::ERROR)
                .with_writer(std::io::sink)
                .init();
        }
    }
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
            } => cmd::integration::cmd_cron_create(&agent, &spec, &prompt),
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
}
