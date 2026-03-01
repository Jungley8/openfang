use crate::TelosContext;
use serde::{Deserialize, Serialize};

/// Priority-ordered list mapping TELOS keys to display titles.
/// MISSION is first and never truncated; lower entries are truncated first.
const SECTION_PRIORITY: &[(&str, &str)] = &[
    ("mission", "使命"),
    ("goals", "当前目标"),
    ("projects", "进行中的项目"),
    ("challenges", "当前挑战"),
    ("strategies", "行动策略"),
    ("beliefs", "信仰与价值观"),
    ("learned", "已学到的教训"),
    ("models", "思维模型"),
    ("narratives", "个人叙事"),
    ("ideas", "想法"),
];

const PRIVATE_START: &str = "<!-- PRIVATE START -->";
const PRIVATE_END: &str = "<!-- PRIVATE END -->";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum InjectionMode {
    #[default]
    Disabled,
    Full,
    Focused,
    Minimal,
    Custom,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum InjectionPosition {
    #[default]
    BeforePrompt,
    AfterPrompt,
    /// Replace `{{TELOS}}` marker in system prompt; falls back to BeforePrompt.
    Placeholder,
}

/// All parameters needed to perform a TELOS injection.
pub struct InjectionParams<'a> {
    pub mode: InjectionMode,
    pub position: InjectionPosition,
    pub max_chars: usize,
    pub custom_files: &'a [String],
    pub directive: Option<&'a str>,
    /// When false, `<!-- PRIVATE START -->…<!-- PRIVATE END -->` blocks are stripped.
    /// Set to true only for local/trusted LLM providers.
    pub trusted_provider: bool,
}

pub struct HandInjector;

impl HandInjector {
    /// Injects the TELOS context into the provided system prompt.
    pub fn inject(context: &TelosContext, system_prompt: &str, params: &InjectionParams) -> String {
        if params.mode == InjectionMode::Disabled {
            return system_prompt.to_string();
        }

        let wants_file = |name: &str| -> bool {
            match params.mode {
                InjectionMode::Disabled => false,
                InjectionMode::Full => true,
                InjectionMode::Focused => {
                    matches!(name, "mission" | "goals" | "projects" | "challenges")
                }
                InjectionMode::Minimal => matches!(name, "mission" | "goals"),
                InjectionMode::Custom => params
                    .custom_files
                    .iter()
                    .any(|f| f.eq_ignore_ascii_case(name)),
            }
        };

        let mut loaded_count = 0usize;
        let mut total_expected = 0usize;
        let mut ordered_sections: Vec<(&str, &str, String)> = Vec::new();

        for &(key, title) in SECTION_PRIORITY {
            if !wants_file(key) {
                continue;
            }
            total_expected += 1;
            if let Some(content) = context.field(key) {
                loaded_count += 1;
                ordered_sections.push((
                    key,
                    title,
                    sanitize_content(content, params.trusted_provider),
                ));
            }
        }

        if total_expected == 0 || loaded_count == 0 {
            return system_prompt.to_string();
        }

        let header = "─────────────────────────────────────────────\n\
                      # 用户上下文 (TELOS — 个人目标系统)\n\n\
                      > 以下是你服务的用户的完整上下文。\n\
                      > 你的所有行动、发现、输出都应与这些目标对齐。\n";

        let footer = format!(
            "\n---\n\
             *TELOS 版本: {} | 加载 {}/{} 文件*\n\
             ─────────────────────────────────────────────\n",
            context.last_updated.format("%Y-%m-%d"),
            loaded_count,
            total_expected
        );

        let directive_text = params
            .directive
            .filter(|d| !d.is_empty())
            .map(|d| format!("\n**关键指令:**\n{d}\n"))
            .unwrap_or_default();

        let fixed_chars = header.len() + footer.len() + directive_text.len();
        let mut current_chars = fixed_chars;
        let mut final_sections = Vec::new();

        for (key, title, content) in &ordered_sections {
            let section_text = format!("\n## {title}\n{content}\n");
            if *key == "mission" || current_chars + section_text.len() <= params.max_chars {
                current_chars += section_text.len();
                final_sections.push(section_text);
            } else {
                let remaining = params.max_chars.saturating_sub(current_chars);
                if remaining > 50 {
                    let budget = remaining.saturating_sub(title.len() + 30);
                    let truncated: String = content.chars().take(budget).collect();
                    final_sections.push(format!("\n## {title}\n{truncated}\n... (已截断) ...\n"));
                }
                break;
            }
        }

        let mut telos_block = String::with_capacity(current_chars);
        telos_block.push_str(header);
        telos_block.push_str(&directive_text);
        for section in &final_sections {
            telos_block.push_str(section);
        }
        telos_block.push_str(&footer);

        match params.position {
            InjectionPosition::BeforePrompt => format!("{telos_block}\n{system_prompt}"),
            InjectionPosition::AfterPrompt => format!("{system_prompt}\n\n{telos_block}"),
            InjectionPosition::Placeholder => {
                if system_prompt.contains("{{TELOS}}") {
                    system_prompt.replace("{{TELOS}}", &telos_block)
                } else {
                    format!("{telos_block}\n{system_prompt}")
                }
            }
        }
    }
}

/// Two-phase content sanitization:
/// 1. Handle PRIVATE blocks (strip or keep based on trust level)
/// 2. Strip all remaining HTML comments (template hints etc.)
fn sanitize_content(input: &str, trusted: bool) -> String {
    let after_private = if trusted {
        input.replace(PRIVATE_START, "").replace(PRIVATE_END, "")
    } else {
        strip_delimited(input, PRIVATE_START, PRIVATE_END)
    };
    strip_html_comments(&after_private)
}

/// Strip regions delimited by `open`…`close` markers (inclusive).
fn strip_delimited(input: &str, open: &str, close: &str) -> String {
    let mut result = String::with_capacity(input.len());
    let mut rest = input;
    while let Some(start) = rest.find(open) {
        result.push_str(&rest[..start]);
        match rest[start..].find(close) {
            Some(end) => rest = &rest[start + end + close.len()..],
            None => {
                rest = "";
                break;
            }
        }
    }
    result.push_str(rest);
    result
}

fn strip_html_comments(input: &str) -> String {
    let result = strip_delimited(input, "<!--", "-->");
    result.trim().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ctx_with(mission: Option<&str>, goals: Option<&str>) -> TelosContext {
        TelosContext {
            mission: mission.map(String::from),
            goals: goals.map(String::from),
            ..TelosContext::default()
        }
    }

    fn default_params() -> InjectionParams<'static> {
        InjectionParams {
            mode: InjectionMode::Focused,
            position: InjectionPosition::BeforePrompt,
            max_chars: 4000,
            custom_files: &[],
            directive: None,
            trusted_provider: false,
        }
    }

    #[test]
    fn inject_disabled_returns_prompt_unchanged() {
        let ctx = ctx_with(Some("m"), Some("g"));
        let params = InjectionParams {
            mode: InjectionMode::Disabled,
            ..default_params()
        };
        assert_eq!(
            HandInjector::inject(&ctx, "You are a helpful agent.", &params),
            "You are a helpful agent."
        );
    }

    #[test]
    fn inject_full_empty_context_returns_prompt_unchanged() {
        let ctx = TelosContext::default();
        let params = InjectionParams {
            mode: InjectionMode::Full,
            ..default_params()
        };
        assert_eq!(
            HandInjector::inject(&ctx, "You are a helpful agent.", &params),
            "You are a helpful agent."
        );
    }

    #[test]
    fn inject_focused_before_prompt() {
        let ctx = ctx_with(Some("Build tools."), Some("- [ ] Ship MVP"));
        let out = HandInjector::inject(&ctx, "You are Researcher Hand.", &default_params());
        assert!(out.starts_with("─────────────────────────────────────────────"));
        assert!(out.contains("## 使命"));
        assert!(out.contains("Build tools."));
        assert!(out.contains("## 当前目标"));
        assert!(out.contains("Ship MVP"));
        assert!(out.ends_with("You are Researcher Hand."));
    }

    #[test]
    fn inject_strips_html_comments() {
        let ctx = ctx_with(Some("Visible <!-- secret --> end"), None);
        let params = InjectionParams {
            mode: InjectionMode::Minimal,
            ..default_params()
        };
        let out = HandInjector::inject(&ctx, "Prompt.", &params);
        assert!(out.contains("Visible"));
        assert!(out.contains("end"));
        assert!(!out.contains("secret"));
    }

    #[test]
    fn inject_placeholder_replaces_marker() {
        let ctx = ctx_with(Some("Build tools."), Some("Ship MVP"));
        let params = InjectionParams {
            mode: InjectionMode::Minimal,
            position: InjectionPosition::Placeholder,
            ..default_params()
        };
        let out =
            HandInjector::inject(&ctx, "You are an agent.\n{{TELOS}}\nDo your best.", &params);
        assert!(!out.contains("{{TELOS}}"));
        assert!(out.contains("Build tools."));
        assert!(out.contains("You are an agent."));
        assert!(out.contains("Do your best."));
    }

    #[test]
    fn inject_placeholder_falls_back_without_marker() {
        let ctx = ctx_with(Some("Build tools."), None);
        let params = InjectionParams {
            mode: InjectionMode::Minimal,
            position: InjectionPosition::Placeholder,
            ..default_params()
        };
        let out = HandInjector::inject(&ctx, "You are an agent.", &params);
        assert!(out.contains("Build tools."));
        assert!(out.ends_with("You are an agent."));
    }

    // -- PRIVATE block tests --

    #[test]
    fn private_blocks_stripped_when_untrusted() {
        let ctx = ctx_with(
            Some("Public mission\n<!-- PRIVATE START -->secret salary<!-- PRIVATE END -->\nMore public"),
            None,
        );
        let params = InjectionParams {
            mode: InjectionMode::Minimal,
            ..default_params()
        };
        let out = HandInjector::inject(&ctx, "P.", &params);
        assert!(out.contains("Public mission"));
        assert!(out.contains("More public"));
        assert!(!out.contains("secret salary"));
        assert!(!out.contains("PRIVATE"));
    }

    #[test]
    fn private_blocks_kept_when_trusted() {
        let ctx = ctx_with(
            Some("Public\n<!-- PRIVATE START -->secret<!-- PRIVATE END -->\nEnd"),
            None,
        );
        let params = InjectionParams {
            mode: InjectionMode::Minimal,
            trusted_provider: true,
            ..default_params()
        };
        let out = HandInjector::inject(&ctx, "P.", &params);
        assert!(out.contains("Public"));
        assert!(out.contains("secret"));
        assert!(out.contains("End"));
        assert!(!out.contains("PRIVATE START"));
    }

    // -- strip helpers --

    #[test]
    fn strip_html_comments_basic() {
        assert_eq!(
            strip_html_comments("hello <!-- removed --> world"),
            "hello  world"
        );
    }

    #[test]
    fn strip_html_comments_unclosed() {
        assert_eq!(strip_html_comments("before <!-- unclosed"), "before");
    }

    #[test]
    fn strip_html_comments_multiple() {
        assert_eq!(
            strip_html_comments("a <!-- x --> b <!-- y --> c"),
            "a  b  c"
        );
    }
}
