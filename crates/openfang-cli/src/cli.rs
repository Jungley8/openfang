//! Clap CLI definitions for OpenFang.

use clap::{Parser, Subcommand};
use std::path::PathBuf;

pub const AFTER_HELP: &str = "\
\x1b[1mHint:\x1b[0m Commands suffixed with [*] have subcommands. Run `<command> --help` for details.

\x1b[1;36mExamples:\x1b[0m
  openfang init                 Initialize config and data directories
  openfang start                Start the kernel daemon
  openfang tui                  Launch the interactive terminal dashboard
  openfang chat                 Quick chat with the default agent
  openfang agent new coder      Spawn a new agent from a template
  openfang models list          Browse available LLM models
  openfang add github           Install the GitHub integration
  openfang doctor               Run diagnostic health checks
  openfang channel setup        Interactive channel setup wizard
  openfang cron list            List scheduled jobs
  openfang workspace clean      List orphan workspace dirs (use --force to remove)

\x1b[1;36mQuick Start:\x1b[0m
  1. openfang init              Set up config + API key
  2. openfang start             Launch the daemon
  3. openfang chat              Start chatting!

\x1b[1;36mMore:\x1b[0m
  Docs:       https://github.com/RightNow-AI/openfang
  Dashboard:  http://127.0.0.1:4200/ (when daemon is running)";

/// OpenFang — the open-source Agent Operating System.
#[derive(Parser)]
#[command(
    name = "openfang",
    version,
    about = "\u{1F40D} OpenFang \u{2014} Open-source Agent Operating System",
    long_about = "\u{1F40D} OpenFang \u{2014} Open-source Agent Operating System\n\n\
                  Deploy, manage, and orchestrate AI agents from your terminal.\n\
                  40 channels \u{00b7} 68 skills \u{00b7} 50+ models \u{00b7} infinite possibilities.",
    after_help = AFTER_HELP,
)]
pub struct Cli {
    /// Path to config file.
    #[arg(long, global = true)]
    pub config: Option<PathBuf>,

    #[command(subcommand)]
    pub command: Option<Commands>,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Initialize OpenFang (create ~/.openfang/ and default config).
    Init {
        /// Quick mode: no prompts, just write config + .env (for CI/scripts).
        #[arg(long)]
        quick: bool,
    },
    /// Start the OpenFang kernel daemon (API server + kernel).
    Start {
        /// Run in the background (daemon mode).
        #[arg(long, short = 'd')]
        daemon: bool,
    },
    /// Restart the running daemon.
    Restart {
        /// Run in the background (daemon mode).
        #[arg(long, short = 'd')]
        daemon: bool,
    },
    /// Stop the running daemon.
    Stop,
    /// Manage agents (new, list, chat, kill, spawn) [*].
    #[command(subcommand)]
    Agent(AgentCommands),
    /// Manage workflows (list, create, run) [*].
    #[command(subcommand)]
    Workflow(WorkflowCommands),
    /// Manage agent workspaces (clean orphan dirs) [*].
    #[command(subcommand)]
    Workspace(WorkspaceCommands),
    /// Manage event triggers (list, create, delete) [*].
    #[command(subcommand)]
    Trigger(TriggerCommands),
    /// Migrate from another agent framework to OpenFang.
    Migrate(MigrateArgs),
    /// Manage skills (install, list, search, create, remove) [*].
    #[command(subcommand)]
    Skill(SkillCommands),
    /// Manage channel integrations (setup, test, enable, disable) [*].
    #[command(subcommand)]
    Channel(ChannelCommands),
    /// Show or edit configuration (show, edit, get, set, keys) [*].
    #[command(subcommand)]
    Config(ConfigCommands),
    /// Quick chat with the default agent.
    Chat {
        /// Optional agent name or ID to chat with.
        agent: Option<String>,
    },
    /// Show kernel status.
    Status {
        /// Output as JSON for scripting.
        #[arg(long)]
        json: bool,
    },
    /// Run diagnostic health checks.
    Doctor {
        /// Output as JSON for scripting.
        #[arg(long)]
        json: bool,
        /// Attempt to auto-fix issues (create missing dirs/config).
        #[arg(long)]
        repair: bool,
    },
    /// Open the web dashboard in the default browser.
    Dashboard,
    /// Generate shell completion scripts.
    Completion {
        /// Shell to generate completions for.
        #[arg(value_enum)]
        shell: clap_complete::Shell,
    },
    /// Start MCP (Model Context Protocol) server over stdio.
    Mcp,
    /// Add an integration (one-click MCP server setup).
    Add {
        /// Integration name (e.g., "github", "slack", "notion").
        name: String,
        /// API key or token to store in the vault.
        #[arg(long)]
        key: Option<String>,
    },
    /// Remove an installed integration.
    Remove {
        /// Integration name.
        name: String,
    },
    /// List or search integrations.
    Integrations {
        /// Search query (optional — lists all if omitted).
        query: Option<String>,
    },
    /// Manage the credential vault (init, set, list, remove) [*].
    #[command(subcommand)]
    Vault(VaultCommands),
    /// Scaffold a new skill or integration template.
    New {
        /// What to scaffold.
        #[arg(value_enum)]
        kind: ScaffoldKind,
    },
    /// Launch the interactive terminal dashboard.
    Tui,
    /// Browse models, aliases, and providers [*].
    #[command(subcommand)]
    Models(ModelsCommands),
    /// Daemon control (start, stop, status) [*].
    #[command(subcommand)]
    Gateway(GatewayCommands),
    /// Manage execution approvals (list, approve, reject) [*].
    #[command(subcommand)]
    Approvals(ApprovalsCommands),
    /// Manage scheduled jobs (list, create, delete, enable, disable) [*].
    #[command(subcommand)]
    Cron(CronCommands),
    /// List conversation sessions.
    Sessions {
        /// Optional agent name or ID to filter by.
        agent: Option<String>,
        /// Output as JSON for scripting.
        #[arg(long)]
        json: bool,
    },
    /// Tail the OpenFang log file.
    Logs {
        /// Number of lines to show.
        #[arg(long, default_value = "50")]
        lines: usize,
        /// Follow log output in real time.
        #[arg(long, short)]
        follow: bool,
    },
    /// Quick daemon health check.
    Health {
        /// Output as JSON for scripting.
        #[arg(long)]
        json: bool,
    },
    /// Security tools and audit trail [*].
    #[command(subcommand)]
    Security(SecurityCommands),
    /// Search and manage agent memory (KV store) [*].
    #[command(subcommand)]
    Memory(MemoryCommands),
    /// Device pairing and token management [*].
    #[command(subcommand)]
    Devices(DevicesCommands),
    /// Generate device pairing QR code.
    Qr,
    /// Webhook helpers and trigger management [*].
    #[command(subcommand)]
    Webhooks(WebhooksCommands),
    /// Interactive onboarding wizard.
    Onboard {
        /// Quick non-interactive mode.
        #[arg(long)]
        quick: bool,
    },
    /// Quick non-interactive initialization.
    Setup {
        /// Quick mode (same as `init --quick`).
        #[arg(long)]
        quick: bool,
    },
    /// Interactive setup wizard for credentials and channels.
    Configure,
    /// Send a one-shot message to an agent.
    Message {
        /// Agent name or ID.
        agent: String,
        /// Message text.
        text: String,
        /// Output as JSON for scripting.
        #[arg(long)]
        json: bool,
    },
    /// System info and version [*].
    #[command(subcommand)]
    System(SystemCommands),
    /// Reset local config and state.
    Reset {
        /// Skip confirmation prompt.
        #[arg(long)]
        confirm: bool,
    },
    /// Manage PAI TELOS goal system [*].
    #[command(subcommand)]
    Telos(TelosCommands),
}

#[derive(Subcommand)]
pub enum TelosCommands {
    /// Initialize TELOS directory and templates.
    Init {
        /// Quick mode (only MISSION and GOALS).
        #[arg(long)]
        quick: bool,
    },
    /// Show current TELOS loading status.
    Status,
    /// Open a TELOS file in your editor.
    Edit {
        /// File name (mission, goals, projects, etc.).
        file: String,
    },
    /// Force reload TELOS context from disk.
    Reload,
    /// Preview how TELOS will be injected into a hand.
    Preview {
        /// Hand name (researcher, lead, etc.).
        hand: String,
    },
    /// Export TELOS context as JSON (backup/migration).
    Export {
        /// Write to file instead of stdout.
        #[arg(short, long)]
        output: Option<std::path::PathBuf>,
    },
    /// Goal alignment report: TELOS snapshots in the last N days.
    Report {
        #[arg(default_value = "30")]
        days: u32,
    },
}

#[derive(Subcommand)]
pub enum VaultCommands {
    /// Initialize the credential vault.
    Init,
    /// Store a credential in the vault.
    Set {
        /// Credential key (env var name).
        key: String,
    },
    /// List all keys in the vault (values are hidden).
    List,
    /// Remove a credential from the vault.
    Remove {
        /// Credential key.
        key: String,
    },
}

#[derive(Clone, clap::ValueEnum)]
pub enum ScaffoldKind {
    Skill,
    Integration,
}

#[derive(clap::Args)]
pub struct MigrateArgs {
    /// Source framework to migrate from.
    #[arg(long, value_enum)]
    pub from: MigrateSourceArg,
    /// Path to the source workspace (auto-detected if not set).
    #[arg(long)]
    pub source_dir: Option<PathBuf>,
    /// Dry run — show what would be imported without making changes.
    #[arg(long)]
    pub dry_run: bool,
}

#[derive(Clone, clap::ValueEnum)]
pub enum MigrateSourceArg {
    Openclaw,
    Langchain,
    Autogpt,
}

#[derive(Subcommand)]
pub enum SkillCommands {
    /// Install a skill from FangHub or a local directory.
    Install {
        /// Skill name, local path, or git URL.
        source: String,
    },
    /// List installed skills.
    List,
    /// Remove an installed skill.
    Remove {
        /// Skill name.
        name: String,
    },
    /// Search FangHub for skills.
    Search {
        /// Search query.
        query: String,
    },
    /// Create a new skill scaffold.
    Create,
}

#[derive(Subcommand)]
pub enum ChannelCommands {
    /// List configured channels and their status.
    List,
    /// Interactive setup wizard for a channel.
    Setup {
        /// Channel name (telegram, discord, slack, whatsapp, etc.). Shows picker if omitted.
        channel: Option<String>,
    },
    /// Test a channel by sending a test message.
    Test {
        /// Channel name.
        channel: String,
    },
    /// Enable a channel.
    Enable {
        /// Channel name.
        channel: String,
    },
    /// Disable a channel without removing its configuration.
    Disable {
        /// Channel name.
        channel: String,
    },
}

#[derive(Subcommand)]
pub enum ConfigCommands {
    /// Show the current configuration.
    Show,
    /// Open the configuration file in your editor.
    Edit,
    /// Get a config value by dotted key path (e.g. "default_model.provider").
    Get {
        /// Dotted key path (e.g. "default_model.provider", "api_listen").
        key: String,
    },
    /// Set a config value (warning: strips TOML comments).
    Set {
        /// Dotted key path.
        key: String,
        /// New value.
        value: String,
    },
    /// Remove a config key (warning: strips TOML comments).
    Unset {
        /// Dotted key path to remove (e.g. "api.cors_origin").
        key: String,
    },
    /// Save an API key to ~/.openfang/.env (prompts interactively).
    SetKey {
        /// Provider name (groq, anthropic, openai, gemini, deepseek, etc.).
        provider: String,
    },
    /// Remove an API key from ~/.openfang/.env.
    DeleteKey {
        /// Provider name.
        provider: String,
    },
    /// Test provider connectivity with the stored API key.
    TestKey {
        /// Provider name.
        provider: String,
    },
}

#[derive(Subcommand)]
pub enum AgentCommands {
    /// Spawn a new agent from a template (interactive or by name).
    New {
        /// Template name (e.g., "coder", "assistant"). Interactive picker if omitted.
        template: Option<String>,
    },
    /// Spawn a new agent from a manifest file.
    Spawn {
        /// Path to the agent manifest TOML file.
        manifest: PathBuf,
    },
    /// List all running agents.
    List {
        /// Output as JSON for scripting.
        #[arg(long)]
        json: bool,
    },
    /// Interactive chat with an agent.
    Chat {
        /// Agent ID (UUID).
        agent_id: String,
    },
    /// Kill an agent.
    Kill {
        /// Agent ID (UUID).
        agent_id: String,
    },
    /// Update mutable agent runtime settings.
    Set {
        /// Agent ID (UUID).
        agent_id: String,
        /// Field to update (currently only: model).
        field: String,
        /// New field value.
        value: String,
    },
}

#[derive(Subcommand)]
pub enum WorkflowCommands {
    /// List all registered workflows.
    List,
    /// Create a workflow from a JSON file.
    Create {
        /// Path to a JSON file describing the workflow.
        file: PathBuf,
    },
    /// Run a workflow by ID.
    Run {
        /// Workflow ID (UUID).
        workflow_id: String,
        /// Input text for the workflow.
        input: String,
    },
}

#[derive(Subcommand)]
pub enum TriggerCommands {
    /// List all triggers (optionally filtered by agent).
    List {
        /// Optional agent ID to filter by.
        #[arg(long)]
        agent_id: Option<String>,
    },
    /// Create a trigger for an agent.
    Create {
        /// Agent ID (UUID) that owns the trigger.
        agent_id: String,
        /// Trigger pattern as JSON (e.g. '{"lifecycle":{}}' or '{"agent_spawned":{"name_pattern":"*"}}').
        pattern_json: String,
        /// Prompt template (use {{event}} placeholder).
        #[arg(long, default_value = "Event: {{event}}")]
        prompt: String,
        /// Maximum number of times to fire (0 = unlimited).
        #[arg(long, default_value = "0")]
        max_fires: u64,
    },
    /// Delete a trigger by ID.
    Delete {
        /// Trigger ID (UUID).
        trigger_id: String,
    },
}

#[derive(Subcommand)]
pub enum WorkspaceCommands {
    /// Remove workspace directories that no longer belong to any registered agent.
    Clean {
        /// Actually delete orphan directories (default: dry run).
        #[arg(long)]
        force: bool,
    },
}

#[derive(Subcommand)]
pub enum ModelsCommands {
    /// List available models (optionally filter by provider).
    List {
        /// Filter by provider name.
        #[arg(long)]
        provider: Option<String>,
        /// Output as JSON for scripting.
        #[arg(long)]
        json: bool,
    },
    /// Show model aliases (shorthand names).
    Aliases {
        /// Output as JSON for scripting.
        #[arg(long)]
        json: bool,
    },
    /// List known LLM providers and their auth status.
    Providers {
        /// Output as JSON for scripting.
        #[arg(long)]
        json: bool,
    },
    /// Set the default model for the daemon.
    Set {
        /// Model ID or alias (e.g. "gpt-4o", "claude-sonnet"). Interactive picker if omitted.
        model: Option<String>,
    },
}

#[derive(Subcommand)]
pub enum GatewayCommands {
    /// Start the kernel daemon.
    Start,
    /// Stop the running daemon.
    Stop,
    /// Show daemon status.
    Status {
        /// Output as JSON for scripting.
        #[arg(long)]
        json: bool,
    },
}

#[derive(Subcommand)]
pub enum ApprovalsCommands {
    /// List pending approvals.
    List {
        /// Output as JSON for scripting.
        #[arg(long)]
        json: bool,
    },
    /// Approve a pending request.
    Approve {
        /// Approval ID.
        id: String,
    },
    /// Reject a pending request.
    Reject {
        /// Approval ID.
        id: String,
    },
}

#[derive(Subcommand)]
pub enum CronCommands {
    /// List scheduled jobs.
    List {
        /// Output as JSON for scripting.
        #[arg(long)]
        json: bool,
    },
    /// Create a new scheduled job.
    Create {
        /// Agent name or ID to run.
        agent: String,
        /// Cron expression (e.g. "0 */6 * * *").
        spec: String,
        /// Prompt to send when the job fires.
        prompt: String,
    },
    /// Delete a scheduled job.
    Delete {
        /// Job ID.
        id: String,
    },
    /// Enable a disabled job.
    Enable {
        /// Job ID.
        id: String,
    },
    /// Disable a job without deleting it.
    Disable {
        /// Job ID.
        id: String,
    },
}

#[derive(Subcommand)]
pub enum SecurityCommands {
    /// Show security status summary.
    Status {
        /// Output as JSON for scripting.
        #[arg(long)]
        json: bool,
    },
    /// Show recent audit trail entries.
    Audit {
        /// Maximum number of entries to show.
        #[arg(long, default_value = "20")]
        limit: usize,
        /// Output as JSON for scripting.
        #[arg(long)]
        json: bool,
    },
    /// Verify audit trail integrity (Merkle chain).
    Verify,
}

#[derive(Subcommand)]
pub enum MemoryCommands {
    /// List KV pairs for an agent.
    List {
        /// Agent name or ID.
        agent: String,
        /// Output as JSON for scripting.
        #[arg(long)]
        json: bool,
    },
    /// Get a specific KV value.
    Get {
        /// Agent name or ID.
        agent: String,
        /// Key name.
        key: String,
        /// Output as JSON for scripting.
        #[arg(long)]
        json: bool,
    },
    /// Set a KV value.
    Set {
        /// Agent name or ID.
        agent: String,
        /// Key name.
        key: String,
        /// Value to store.
        value: String,
    },
    /// Delete a KV pair.
    Delete {
        /// Agent name or ID.
        agent: String,
        /// Key name.
        key: String,
    },
}

#[derive(Subcommand)]
pub enum DevicesCommands {
    /// List paired devices.
    List {
        /// Output as JSON for scripting.
        #[arg(long)]
        json: bool,
    },
    /// Start a new device pairing flow.
    Pair,
    /// Remove a paired device.
    Remove {
        /// Device ID.
        id: String,
    },
}

#[derive(Subcommand)]
pub enum WebhooksCommands {
    /// List configured webhooks.
    List {
        /// Output as JSON for scripting.
        #[arg(long)]
        json: bool,
    },
    /// Create a new webhook trigger.
    Create {
        /// Agent name or ID.
        agent: String,
        /// Webhook callback URL.
        url: String,
    },
    /// Delete a webhook.
    Delete {
        /// Webhook ID.
        id: String,
    },
    /// Send a test payload to a webhook.
    Test {
        /// Webhook ID.
        id: String,
    },
}

#[derive(Subcommand)]
pub enum SystemCommands {
    /// Show detailed system info.
    Info {
        /// Output as JSON for scripting.
        #[arg(long)]
        json: bool,
    },
    /// Show version information.
    Version {
        /// Output as JSON for scripting.
        #[arg(long)]
        json: bool,
    },
}
