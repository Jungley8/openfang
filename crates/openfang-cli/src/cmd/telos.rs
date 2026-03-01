//! TELOS goal system commands.

use crate::daemon::{daemon_client, find_daemon};
use crate::ui;
use colored::Colorize;
use openfang_telos::TelosEngine;

pub fn cmd_telos_init(quick: bool) {
    let dir = TelosEngine::get_default_dir();
    let engine = TelosEngine::new(&dir);

    ui::banner();
    ui::blank();
    println!("  Initializing PAI TELOS system...");
    ui::kv("Directory", &dir.display().to_string());
    ui::blank();

    let rt = tokio::runtime::Runtime::new().unwrap();
    if let Err(e) = rt.block_on(engine.init(quick)) {
        ui::error(&format!("Failed to initialize TELOS: {e}"));
        std::process::exit(1);
    }

    ui::success("TELOS initialized successfully!");
    ui::blank();
    ui::next_steps(&[
        "Edit your mission:  openfang telos edit mission",
        "Edit your goals:    openfang telos edit goals",
        "Check status:       openfang telos status",
    ]);
}

pub fn cmd_telos_status() {
    let dir = TelosEngine::get_default_dir();
    let engine = TelosEngine::new(&dir);

    let rt = tokio::runtime::Runtime::new().unwrap();
    if let Err(e) = rt.block_on(engine.load_all()) {
        ui::error(&format!("Failed to load TELOS: {e}"));
        ui::hint("Run `openfang telos init` to set up templates.");
        return;
    }

    let ctx = rt.block_on(engine.get_context());

    ui::section("PAI TELOS Status");
    ui::blank();
    ui::kv("Directory", &dir.display().to_string());
    ui::kv(
        "Last Loaded",
        &ctx.last_updated.format("%Y-%m-%d %H:%M:%S").to_string(),
    );
    ui::blank();

    ui::section("Files Loaded");
    let files = [
        ("MISSION.md", ctx.mission.is_some()),
        ("GOALS.md", ctx.goals.is_some()),
        ("PROJECTS.md", ctx.projects.is_some()),
        ("CHALLENGES.md", ctx.challenges.is_some()),
        ("BELIEFS.md", ctx.beliefs.is_some()),
        ("MODELS.md", ctx.models.is_some()),
        ("STRATEGIES.md", ctx.strategies.is_some()),
        ("NARRATIVES.md", ctx.narratives.is_some()),
        ("LEARNED.md", ctx.learned.is_some()),
        ("IDEAS.md", ctx.ideas.is_some()),
    ];

    for (name, exists) in files {
        if exists {
            ui::check_ok(name);
        } else {
            ui::check_fail(&format!("{name} (missing)"));
        }
    }
    ui::blank();
}

pub fn cmd_telos_edit(file: &str) {
    let dir = TelosEngine::get_default_dir();
    let filename = match file.to_lowercase().as_str() {
        "mission" => "MISSION.md",
        "goals" => "GOALS.md",
        "projects" => "PROJECTS.md",
        "beliefs" => "BELIEFS.md",
        "models" => "MODELS.md",
        "strategies" => "STRATEGIES.md",
        "narratives" => "NARRATIVES.md",
        "learned" => "LEARNED.md",
        "challenges" => "CHALLENGES.md",
        "ideas" => "IDEAS.md",
        _ => {
            ui::error(&format!("Unknown TELOS file: {file}"));
            return;
        }
    };

    let path = dir.join(filename);
    if !path.exists() {
        ui::error(&format!("File does not exist: {}", path.display()));
        ui::hint("Run `openfang telos init` to create it.");
        return;
    }

    let editor = std::env::var("EDITOR").unwrap_or_else(|_| {
        #[cfg(windows)]
        {
            "notepad".to_string()
        }
        #[cfg(not(windows))]
        {
            "vi".to_string()
        }
    });

    println!("Opening {} in {}...", filename, editor);

    let status = std::process::Command::new(editor).arg(&path).status();

    match status {
        Ok(s) if s.success() => ui::success(&format!("Finished editing {filename}")),
        Ok(s) => ui::error(&format!("Editor exited with status: {s}")),
        Err(e) => ui::error(&format!("Failed to launch editor: {e}")),
    }
}

pub fn cmd_telos_reload() {
    if let Some(base) = find_daemon() {
        let client = daemon_client();
        let resp = client.post(format!("{base}/api/telos/reload")).send();
        match resp {
            Ok(r) => {
                let status = r.status();
                if status.is_success() {
                    ui::success("Daemon TELOS context reloaded.");
                } else {
                    ui::error(&format!("Failed to reload daemon TELOS: {status}"));
                }
            }
            Err(e) => ui::error(&format!("Failed to contact daemon: {e}")),
        }
    } else {
        ui::success("TELOS reloaded (local cache is always fresh on new CLI commands).");
    }
}

pub fn cmd_telos_preview(hand: &str) {
    let dir = TelosEngine::get_default_dir();
    let engine = TelosEngine::new(&dir);

    let rt = tokio::runtime::Runtime::new().unwrap();
    if let Err(e) = rt.block_on(engine.load_all()) {
        ui::error(&format!("Failed to load TELOS: {e}"));
        return;
    }

    let ctx = rt.block_on(engine.get_context());

    let bundled = openfang_hands::bundled::bundled_hands();
    let found = bundled.into_iter().find(|(id, _, _)| *id == hand);

    let (def, telos_config) = if let Some((id, toml_str, skill_str)) = found {
        match openfang_hands::bundled::parse_bundled(id, toml_str, skill_str) {
            Ok(d) => {
                let tc = d.telos.clone();
                (Some(d), tc)
            }
            Err(e) => {
                ui::error(&format!("Failed to parse hand {hand}: {e}"));
                return;
            }
        }
    } else {
        ui::error(&format!("Hand '{}' not found in bundled registry. Using default TELOS config (Focused mode) to show a generic preview.", hand));
        (
            None,
            openfang_hands::HandTelosConfig {
                mode: openfang_telos::InjectionMode::Focused,
                position: openfang_telos::InjectionPosition::BeforePrompt,
                max_chars: 4000,
                files: vec![],
                directive: None,
            },
        )
    };

    ui::banner();
    println!("  TELOS Injection Preview for Hand: {}", hand.cyan().bold());
    println!(
        "  Mode: {:?} | Position: {:?}",
        telos_config.mode, telos_config.position
    );
    ui::blank();

    let dummy_prompt = if let Some(d) = def {
        d.agent.system_prompt
    } else {
        "[Original Hand System Prompt]".to_string()
    };

    let params = openfang_telos::InjectionParams {
        mode: telos_config.mode,
        position: telos_config.position,
        max_chars: telos_config.max_chars,
        custom_files: &telos_config.files,
        directive: telos_config.directive.as_deref(),
        trusted_provider: false,
    };
    let injected = openfang_telos::injector::HandInjector::inject(&ctx, &dummy_prompt, &params);

    println!("{}", injected);
    ui::blank();
    ui::hint("This is exactly what the LLM will see when this hand is activated.");
}

pub fn cmd_telos_export(output: Option<&std::path::Path>) {
    let json_str = if let Some(base) = find_daemon() {
        let client = daemon_client();
        match client.get(format!("{base}/api/telos/status")).send() {
            Ok(r) if r.status().is_success() => match r.text() {
                Ok(s) => s,
                Err(e) => {
                    ui::error(&format!("Failed to read daemon TELOS response: {e}"));
                    std::process::exit(1);
                }
            },
            Ok(r) => {
                ui::error(&format!("Daemon returned {}", r.status()));
                std::process::exit(1);
            }
            Err(e) => {
                ui::error(&format!("Failed to contact daemon: {e}"));
                std::process::exit(1);
            }
        }
    } else {
        let dir = TelosEngine::get_default_dir();
        let engine = TelosEngine::new(&dir);
        let rt = tokio::runtime::Runtime::new().unwrap();
        if let Err(e) = rt.block_on(engine.load_all()) {
            ui::error(&format!("Failed to load TELOS: {e}"));
            ui::hint("Run `openfang telos init` to set up templates.");
            std::process::exit(1);
        }
        let ctx = rt.block_on(engine.get_context());
        match serde_json::to_string_pretty(&ctx) {
            Ok(s) => s,
            Err(e) => {
                ui::error(&format!("Failed to serialize TELOS: {e}"));
                std::process::exit(1);
            }
        }
    };

    let out = match serde_json::from_str::<serde_json::Value>(&json_str)
        .and_then(|v| serde_json::to_string_pretty(&v))
    {
        Ok(pretty) => pretty,
        Err(_) => json_str,
    };

    if let Some(path) = output {
        match std::fs::write(path, &out) {
            Ok(_) => ui::success(&format!("Exported TELOS to {}", path.display())),
            Err(e) => {
                ui::error(&format!("Failed to write file: {e}"));
                std::process::exit(1);
            }
        }
    } else {
        println!("{out}");
    }
}

pub fn cmd_telos_report(days: u32) {
    if let Some(base) = find_daemon() {
        let client = daemon_client();
        let url = format!("{base}/api/telos/report?days={days}");
        match client.get(&url).send() {
            Ok(r) if r.status().is_success() => {
                let json: serde_json::Value = match r.json() {
                    Ok(v) => v,
                    Err(e) => {
                        ui::error(&format!("Failed to parse report: {e}"));
                        std::process::exit(1);
                    }
                };
                let empty: Vec<serde_json::Value> = vec![];
                let snapshots = json
                    .get("snapshots")
                    .and_then(|s| s.as_array())
                    .unwrap_or(&empty);
                ui::section("TELOS Report");
                ui::kv("Period", &format!("Last {days} days"));
                ui::kv("Snapshots", &snapshots.len().to_string());
                ui::blank();
                for s in snapshots.iter().take(20) {
                    let created = s.get("created_at").and_then(|c| c.as_str()).unwrap_or("—");
                    let mid = s
                        .get("mission_hash")
                        .and_then(|h| h.as_str())
                        .unwrap_or("—");
                    println!("  {}  mission_hash: {}", created, mid);
                }
                if snapshots.len() > 20 {
                    println!("  ... and {} more", snapshots.len() - 20);
                }
            }
            Ok(r) => {
                ui::error(&format!("Daemon returned {}", r.status()));
                std::process::exit(1);
            }
            Err(e) => {
                ui::error(&format!("Failed to contact daemon: {e}"));
                ui::hint("Start the daemon with `openfang start` to use telos report.");
                std::process::exit(1);
            }
        }
    } else {
        ui::error("No daemon running.");
        ui::hint("Start the daemon with `openfang start` to use telos report.");
        std::process::exit(1);
    }
}
