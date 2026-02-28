pub mod injector;
pub mod templates;

pub use injector::{InjectionMode, InjectionParams, InjectionPosition};

use anyhow::{Context, Result};
use notify::{Config, Event, RecursiveMode, Watcher};
use serde::{Deserialize, Serialize};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, error, info};

pub const TELOS_FILES: &[(&str, &str)] = &[
    ("mission", "MISSION.md"),
    ("goals", "GOALS.md"),
    ("projects", "PROJECTS.md"),
    ("beliefs", "BELIEFS.md"),
    ("models", "MODELS.md"),
    ("strategies", "STRATEGIES.md"),
    ("narratives", "NARRATIVES.md"),
    ("learned", "LEARNED.md"),
    ("challenges", "CHALLENGES.md"),
    ("ideas", "IDEAS.md"),
];

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TelosContext {
    pub mission: Option<String>,
    pub goals: Option<String>,
    pub projects: Option<String>,
    pub beliefs: Option<String>,
    pub models: Option<String>,
    pub strategies: Option<String>,
    pub narratives: Option<String>,
    pub learned: Option<String>,
    pub challenges: Option<String>,
    pub ideas: Option<String>,
    pub last_updated: chrono::DateTime<chrono::Utc>,
}

impl TelosContext {
    pub fn field(&self, key: &str) -> Option<&str> {
        match key {
            "mission" => self.mission.as_deref(),
            "goals" => self.goals.as_deref(),
            "projects" => self.projects.as_deref(),
            "beliefs" => self.beliefs.as_deref(),
            "models" => self.models.as_deref(),
            "strategies" => self.strategies.as_deref(),
            "narratives" => self.narratives.as_deref(),
            "learned" => self.learned.as_deref(),
            "challenges" => self.challenges.as_deref(),
            "ideas" => self.ideas.as_deref(),
            _ => None,
        }
    }

    fn field_mut(&mut self, key: &str) -> &mut Option<String> {
        match key {
            "mission" => &mut self.mission,
            "goals" => &mut self.goals,
            "projects" => &mut self.projects,
            "beliefs" => &mut self.beliefs,
            "models" => &mut self.models,
            "strategies" => &mut self.strategies,
            "narratives" => &mut self.narratives,
            "learned" => &mut self.learned,
            "challenges" => &mut self.challenges,
            "ideas" => &mut self.ideas,
            other => panic!("unknown TELOS key: {other}"),
        }
    }
}

pub struct TelosEngine {
    dir_path: PathBuf,
    context: Arc<RwLock<TelosContext>>,
}

impl TelosEngine {
    pub fn new<P: AsRef<Path>>(dir: P) -> Self {
        Self {
            dir_path: dir.as_ref().to_path_buf(),
            context: Arc::new(RwLock::new(TelosContext::default())),
        }
    }

    pub fn get_default_dir() -> PathBuf {
        std::env::var("OCTARQ_TELOS_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(|_| {
                dirs::home_dir()
                    .unwrap_or_else(|| PathBuf::from("."))
                    .join(".octarq")
                    .join("telos")
            })
    }

    pub fn load_context_sync(dir: &Path) -> TelosContext {
        let mut ctx = TelosContext::default();
        for &(key, filename) in TELOS_FILES {
            *ctx.field_mut(key) = std::fs::read_to_string(dir.join(filename)).ok();
        }
        ctx.last_updated = chrono::Utc::now();
        ctx
    }

    pub async fn init(&self, quick: bool) -> Result<()> {
        if !self.dir_path.exists() {
            std::fs::create_dir_all(&self.dir_path)
                .with_context(|| format!("Failed to create directory {:?}", self.dir_path))?;
        }

        let files = if quick {
            vec![
                ("MISSION.md", templates::MISSION_TEMPLATE),
                ("GOALS.md", templates::GOALS_TEMPLATE),
            ]
        } else {
            vec![
                ("MISSION.md", templates::MISSION_TEMPLATE),
                ("GOALS.md", templates::GOALS_TEMPLATE),
                ("PROJECTS.md", templates::PROJECTS_TEMPLATE),
                ("BELIEFS.md", templates::BELIEFS_TEMPLATE),
                ("MODELS.md", templates::MODELS_TEMPLATE),
                ("STRATEGIES.md", templates::STRATEGIES_TEMPLATE),
                ("NARRATIVES.md", templates::NARRATIVES_TEMPLATE),
                ("LEARNED.md", templates::LEARNED_TEMPLATE),
                ("CHALLENGES.md", templates::CHALLENGES_TEMPLATE),
                ("IDEAS.md", templates::IDEAS_TEMPLATE),
            ]
        };

        for (filename, template) in files {
            let path = self.dir_path.join(filename);
            if !path.exists() {
                let mut file = std::fs::File::create(&path)?;
                file.write_all(template.as_bytes())?;
                info!("Created template: {:?}", path);
            }
        }

        Ok(())
    }

    pub async fn load_all(&self) -> Result<()> {
        let mut ctx = self.context.write().await;
        for &(key, filename) in TELOS_FILES {
            *ctx.field_mut(key) = self.read_file(filename).await;
        }
        ctx.last_updated = chrono::Utc::now();
        info!("TELOS context loaded from {:?}", self.dir_path);
        Ok(())
    }

    async fn read_file(&self, filename: &str) -> Option<String> {
        tokio::fs::read_to_string(self.dir_path.join(filename))
            .await
            .ok()
    }

    pub async fn get_context(&self) -> TelosContext {
        self.context.read().await.clone()
    }

    pub fn start_watching(engine: Arc<Self>) -> Result<notify::RecommendedWatcher> {
        let dir = engine.dir_path.clone();
        let handle = tokio::runtime::Handle::current();

        let mut watcher = notify::RecommendedWatcher::new(
            move |res: notify::Result<Event>| match res {
                Ok(event) => {
                    if event.kind.is_modify() || event.kind.is_create() {
                        debug!("TELOS file changed: {:?}", event.paths);
                        let engine_clone = engine.clone();
                        handle.spawn(async move {
                            if let Err(e) = engine_clone.load_all().await {
                                error!("Failed to reload TELOS: {:?}", e);
                            }
                        });
                    }
                }
                Err(e) => error!("watch error: {:?}", e),
            },
            Config::default(),
        )?;

        watcher.watch(&dir, RecursiveMode::NonRecursive)?;
        info!("Watching TELOS directory: {:?}", dir);
        Ok(watcher)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn load_context_sync_empty_dir() {
        let dir = tempfile::tempdir().unwrap();
        let ctx = TelosEngine::load_context_sync(dir.path());
        assert!(ctx.mission.is_none());
        assert!(ctx.goals.is_none());
        assert!(ctx.projects.is_none());
        assert!(ctx.challenges.is_none());
    }

    #[test]
    fn load_context_sync_partial_files() {
        let dir = tempfile::tempdir().unwrap();
        let mission = "My mission is to ship.";
        std::fs::write(dir.path().join("MISSION.md"), mission).unwrap();
        std::fs::write(dir.path().join("GOALS.md"), "- [ ] Q1 goal").unwrap();
        let ctx = TelosEngine::load_context_sync(dir.path());
        assert_eq!(ctx.mission.as_deref(), Some(mission));
        assert_eq!(ctx.goals.as_deref(), Some("- [ ] Q1 goal"));
        assert!(ctx.projects.is_none());
    }

    #[test]
    fn load_context_sync_nonexistent_dir() {
        let dir = std::path::Path::new("/nonexistent_telos_dir_xyz");
        let ctx = TelosEngine::load_context_sync(dir);
        assert!(ctx.mission.is_none());
        assert!(ctx.goals.is_none());
    }

    #[test]
    fn field_accessor_round_trip() {
        let mut ctx = TelosContext::default();
        *ctx.field_mut("mission") = Some("test".into());
        assert_eq!(ctx.field("mission"), Some("test"));
        assert_eq!(ctx.field("goals"), None);
        assert_eq!(ctx.field("nonexistent"), None);
    }
}
