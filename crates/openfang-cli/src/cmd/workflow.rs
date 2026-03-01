//! Workflow, workspace, trigger, and migrate commands.

use crate::cli::{MigrateArgs, MigrateSourceArg};
use crate::daemon::{boot_kernel, daemon_client, daemon_json, require_daemon};
use crate::ui;
use std::path::PathBuf;

pub fn cmd_workflow_list() {
    let base = require_daemon("workflow list");
    let client = daemon_client();
    let body = daemon_json(client.get(format!("{base}/api/workflows")).send());

    match body.as_array() {
        Some(workflows) if workflows.is_empty() => println!("No workflows registered."),
        Some(workflows) => {
            println!("{:<38} {:<20} {:<6} CREATED", "ID", "NAME", "STEPS");
            println!("{}", "-".repeat(80));
            for w in workflows {
                println!(
                    "{:<38} {:<20} {:<6} {}",
                    w["id"].as_str().unwrap_or("?"),
                    w["name"].as_str().unwrap_or("?"),
                    w["steps"].as_u64().unwrap_or(0),
                    w["created_at"].as_str().unwrap_or("?"),
                );
            }
        }
        None => println!("No workflows registered."),
    }
}

pub fn cmd_workflow_create(file: PathBuf) {
    let base = require_daemon("workflow create");
    if !file.exists() {
        eprintln!("Workflow file not found: {}", file.display());
        std::process::exit(1);
    }
    let contents = std::fs::read_to_string(&file).unwrap_or_else(|e| {
        eprintln!("Error reading workflow file: {e}");
        std::process::exit(1);
    });
    let json_body: serde_json::Value = serde_json::from_str(&contents).unwrap_or_else(|e| {
        eprintln!("Invalid JSON: {e}");
        std::process::exit(1);
    });

    let client = daemon_client();
    let body = daemon_json(
        client
            .post(format!("{base}/api/workflows"))
            .json(&json_body)
            .send(),
    );

    if let Some(id) = body["workflow_id"].as_str() {
        println!("Workflow created successfully!");
        println!("  ID: {id}");
    } else {
        eprintln!(
            "Failed to create workflow: {}",
            body["error"].as_str().unwrap_or("Unknown error")
        );
        std::process::exit(1);
    }
}

pub fn cmd_workflow_run(workflow_id: &str, input: &str) {
    let base = require_daemon("workflow run");
    let client = daemon_client();
    let body = daemon_json(
        client
            .post(format!("{base}/api/workflows/{workflow_id}/run"))
            .json(&serde_json::json!({"input": input}))
            .send(),
    );

    if let Some(output) = body["output"].as_str() {
        println!("Workflow completed!");
        println!("  Run ID: {}", body["run_id"].as_str().unwrap_or("?"));
        println!("  Output:\n{output}");
    } else {
        eprintln!(
            "Workflow failed: {}",
            body["error"].as_str().unwrap_or("Unknown error")
        );
        std::process::exit(1);
    }
}

pub fn cmd_workspace_clean(config: Option<PathBuf>, force: bool) {
    let kernel = boot_kernel(config);
    let workspaces_dir = kernel.config.effective_workspaces_dir();
    let active_short_ids: std::collections::HashSet<String> =
        kernel.list_active_short_ids().into_iter().collect();

    if !workspaces_dir.exists() {
        println!(
            "Workspaces directory does not exist: {}",
            workspaces_dir.display()
        );
        return;
    }

    let entries = match std::fs::read_dir(&workspaces_dir) {
        Ok(e) => e,
        Err(e) => {
            eprintln!("Failed to read workspaces dir: {e}");
            std::process::exit(1);
        }
    };

    let mut keep = Vec::new();
    let mut remove = Vec::new();
    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        let name = match path.file_name().and_then(|n| n.to_str()) {
            Some(n) => n,
            None => continue,
        };
        if name.starts_with('.') {
            continue;
        }
        let suffix = name.rsplit('-').next().unwrap_or("");
        if suffix.len() != 8 || !suffix.chars().all(|c| c.is_ascii_hexdigit()) {
            continue;
        }
        if active_short_ids.contains(suffix) {
            keep.push(name.to_string());
        } else {
            remove.push(path);
        }
    }

    let active_count = active_short_ids.len();
    let orphan_count = remove.len();
    println!("Active agents: {active_count}");
    println!("Orphan workspace directories: {orphan_count}");
    ui::blank();
    for k in &keep {
        println!("  KEEP:  {k}");
    }
    for r in &remove {
        let name = r
            .file_name()
            .map(|n| n.to_string_lossy())
            .unwrap_or_default();
        println!("  REMOVE: {name}");
    }

    if remove.is_empty() {
        println!("Nothing to clean.");
        return;
    }

    if !force {
        ui::hint("Run with --force to delete these directories.");
        return;
    }

    for path in &remove {
        if let Err(e) = std::fs::remove_dir_all(path) {
            eprintln!("Failed to remove {}: {e}", path.display());
        }
    }
    ui::success(&format!("Removed {} orphan directory(ies).", remove.len()));
}

pub fn cmd_trigger_list(agent_id: Option<&str>) {
    let base = require_daemon("trigger list");
    let client = daemon_client();

    let url = match agent_id {
        Some(id) => format!("{base}/api/triggers?agent_id={id}"),
        None => format!("{base}/api/triggers"),
    };
    let body = daemon_json(client.get(&url).send());

    match body.as_array() {
        Some(triggers) if triggers.is_empty() => println!("No triggers registered."),
        Some(triggers) => {
            println!(
                "{:<38} {:<38} {:<8} {:<6} PATTERN",
                "TRIGGER ID", "AGENT ID", "ENABLED", "FIRES"
            );
            println!("{}", "-".repeat(110));
            for t in triggers {
                println!(
                    "{:<38} {:<38} {:<8} {:<6} {}",
                    t["id"].as_str().unwrap_or("?"),
                    t["agent_id"].as_str().unwrap_or("?"),
                    t["enabled"].as_bool().unwrap_or(false),
                    t["fire_count"].as_u64().unwrap_or(0),
                    t["pattern"],
                );
            }
        }
        None => println!("No triggers registered."),
    }
}

pub fn cmd_trigger_create(agent_id: &str, pattern_json: &str, prompt: &str, max_fires: u64) {
    let base = require_daemon("trigger create");
    let pattern: serde_json::Value = serde_json::from_str(pattern_json).unwrap_or_else(|e| {
        eprintln!("Invalid pattern JSON: {e}");
        eprintln!("Examples:");
        eprintln!("  '{{\"lifecycle\":{{}}}}'");
        eprintln!("  '{{\"agent_spawned\":{{\"name_pattern\":\"*\"}}}}'");
        eprintln!("  '{{\"agent_terminated\":{{}}}}'");
        eprintln!("  '{{\"all\":{{}}}}'");
        std::process::exit(1);
    });

    let client = daemon_client();
    let body = daemon_json(
        client
            .post(format!("{base}/api/triggers"))
            .json(&serde_json::json!({
                "agent_id": agent_id,
                "pattern": pattern,
                "prompt_template": prompt,
                "max_fires": max_fires,
            }))
            .send(),
    );

    if let Some(id) = body["trigger_id"].as_str() {
        println!("Trigger created successfully!");
        println!("  Trigger ID: {id}");
        println!("  Agent ID:   {agent_id}");
    } else {
        eprintln!(
            "Failed to create trigger: {}",
            body["error"].as_str().unwrap_or("Unknown error")
        );
        std::process::exit(1);
    }
}

pub fn cmd_trigger_delete(trigger_id: &str) {
    let base = require_daemon("trigger delete");
    let client = daemon_client();
    let body = daemon_json(
        client
            .delete(format!("{base}/api/triggers/{trigger_id}"))
            .send(),
    );

    if body.get("status").is_some() {
        println!("Trigger {trigger_id} deleted.");
    } else {
        eprintln!(
            "Failed to delete trigger: {}",
            body["error"].as_str().unwrap_or("Unknown error")
        );
        std::process::exit(1);
    }
}

pub fn cmd_migrate(args: MigrateArgs) {
    let source = match args.from {
        MigrateSourceArg::Openclaw => openfang_migrate::MigrateSource::OpenClaw,
        MigrateSourceArg::Langchain => openfang_migrate::MigrateSource::LangChain,
        MigrateSourceArg::Autogpt => openfang_migrate::MigrateSource::AutoGpt,
    };

    let source_dir = args.source_dir.unwrap_or_else(|| {
        let home = dirs::home_dir().unwrap_or_else(|| {
            eprintln!("Error: Could not determine home directory");
            std::process::exit(1);
        });
        match source {
            openfang_migrate::MigrateSource::OpenClaw => home.join(".openclaw"),
            openfang_migrate::MigrateSource::LangChain => home.join(".langchain"),
            openfang_migrate::MigrateSource::AutoGpt => home.join("Auto-GPT"),
        }
    });

    let target_dir = dirs::home_dir()
        .unwrap_or_else(|| {
            eprintln!("Error: Could not determine home directory");
            std::process::exit(1);
        })
        .join(".openfang");

    println!("Migrating from {} ({})...", source, source_dir.display());
    if args.dry_run {
        println!("  (dry run — no changes will be made)\n");
    }

    let options = openfang_migrate::MigrateOptions {
        source,
        source_dir,
        target_dir,
        dry_run: args.dry_run,
    };

    match openfang_migrate::run_migration(&options) {
        Ok(report) => {
            report.print_summary();

            if !args.dry_run {
                let report_path = options.target_dir.join("migration_report.md");
                if let Err(e) = std::fs::write(&report_path, report.to_markdown()) {
                    eprintln!("Warning: Could not save migration report: {e}");
                } else {
                    println!("\n  Report saved to: {}", report_path.display());
                }
            }
        }
        Err(e) => {
            eprintln!("Migration failed: {e}");
            std::process::exit(1);
        }
    }
}
