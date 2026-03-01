# PAI × Octarq 集成方案设计 (v1.1)

> 将 PAI (Personal AI Infrastructure) 的核心思想与 TELOS 目标系统集成到 Octarq Agent OS

---

## 一、设计哲学：为什么要集成

### 当前格局的缺失

所有现有的 Agent OS（Octarq/OpenFang、OpenClaw、ZeroClaw、LangGraph、CrewAI）都回答了同一个问题：

> **"怎么执行任务？"**

没有一个回答了：

> **"为谁执行？他们真正想要什么？这个任务对他们的人生目标有什么意义？"**

PAI 的核心洞察是：**没有目标感知的 Agent，只是一个更快的搜索引擎**。

Octarq 拥有最完善的执行基础设施（Rust 性能、16 层安全、40 个渠道、自主调度），但缺少"灵魂"——对用户是谁、想要什么、正在面对什么的深度理解。PAI 就是这个灵魂。

### 集成后的用户体验差异

**集成前（现在的 Octarq）：**
```
用户: octarq hand activate researcher
Researcher Hand: 我是一个研究代理。请给我一个研究主题。
用户: 帮我研究竞争对手
Researcher Hand: 请告诉我你的竞争对手是谁，你的行业是什么...
```

**集成后（PAI × Octarq）：**
```
用户: octarq hand activate researcher
Researcher Hand: 我已加载你的 TELOS 上下文。
  → 使命: 构建让创业者更高效的 B2B SaaS 工具
  → 当前项目: ProjectFlow MVP (Q2 发布目标)
  → 挑战: 融资路演准备，需要市场差异化数据

基于你的上下文，我将主动开始:
  1. 监控 ProjectFlow 竞品的产品更新 (Notion, Linear, Monday.com)
  2. 收集 B2B SaaS 融资叙事中的差异化数据点
  3. 每周五 9:00 发送竞品动态报告到你的 Telegram

是否启动？[Y/n]
```

这就是"有灵魂的 Agent OS"。

---

## 二、PAI 核心思想提炼

集成前必须理解 PAI 的本质，避免照搬形式而丢失精髓。

### 2.1 TELOS 系统：10 个文件，1 套人格

PAI 的 TELOS 不是简单的"用户偏好设置"，而是一套**结构化的人格档案**：

| 文件 | 核心问题 | Octarq 场景价值 |
|------|----------|----------------|
| `MISSION.md` | 你为什么存在？ | 过滤与使命无关的任务建议 |
| `GOALS.md` | 你现在追求什么？ | 为所有 Hand 提供优先级排序基准 |
| `PROJECTS.md` | 你在做什么？ | Lead/Researcher Hand 的研究靶向 |
| `BELIEFS.md` | 你相信什么？ | 避免 Hand 输出与用户价值观冲突的内容 |
| `MODELS.md` | 你如何思考？ | Predictor Hand 使用用户偏好的分析框架 |
| `STRATEGIES.md` | 你如何行动？ | Twitter/Lead Hand 遵循用户的沟通风格 |
| `NARRATIVES.md` | 你的故事是什么？ | 提升 Researcher/Collector 的背景意识 |
| `LEARNED.md` | 你从经历中学到了什么？ | 防止 Agent 重复犯同类错误 |
| `CHALLENGES.md` | 你现在面对什么障碍？ | 自动将阻碍纳入研究范围 |
| `IDEAS.md` | 你有什么想法想探索？ | Researcher/Predictor 的主动探索清单 |

### 2.2 PAI 核心原则的 Octarq 映射

PAI 的 16 条原则中，以下 6 条对 Octarq 集成最关键：

**原则 1：用户中心性 (User Centricity)**
> "基础设施服务于人，而不是反过来"

→ Octarq 的 HAND.toml 需要新增 `[telos]` 块，不是可选字段，而是 Hand 的一等公民配置。

**原则 4：脚手架 > 模型 (Scaffolding > Model)**
> "系统架构比你用哪个模型更重要"

→ TELOS 注入是架构层的改进，不依赖换更好的模型。同样的 Claude 3.5 Sonnet，有了用户目标上下文后质量天壤之别。

**原则 7：Spec/Test/Evals First**
> "先写规格和测试，再构建"

→ 每个 Hand 应声明自己使用哪些 TELOS 字段，形成可测试的 capability contract。

**原则 8：UNIX 哲学**
> "做好一件事，保持可组合"

→ TELOS 引擎是独立 crate（`openfang-telos`），不污染其他 crate，任何 Hand 可选择性接入。

**原则 12：Skill 管理**
> "模块化能力，基于上下文智能路由"

→ PAI 的 18 个 Skills（research、OSINT、redteam 等）通过 SKILL.md 格式直接兼容 Octarq 的 60 个 bundled skills。

**原则 13：Memory 系统**
> "所有值得知道的事情都要被捕获"

→ Octarq 已有 SQLite + 向量内存，需要加入 TELOS 版本历史，让 Agent 感知用户目标随时间的演变。

---

## 三、集成架构全景

```
┌─────────────────────────────────────────────────────────────────────┐
│                         用户空间                                      │
│  ~/.openfang/telos/ (支持环境变量 OCTARQ_TELOS_DIR 覆盖)                 │
│  ├── MISSION.md        ← 你是谁，为什么做这些                          │
│  ├── GOALS.md          ← 当前目标，优先级排序                           │
│  ├── PROJECTS.md       ← 进行中的项目                                  │
│  ├── BELIEFS.md        ← 价值观与信仰                                  │
│  ├── MODELS.md         ← 思维框架                                      │
│  ├── STRATEGIES.md     ← 行动策略偏好                                  │
│  ├── NARRATIVES.md     ← 人生叙事与经历                                │
│  ├── LEARNED.md        ← 已学到的教训                                  │
│  ├── CHALLENGES.md     ← 当前面临的挑战                                │
│  └── IDEAS.md          ← 想探索的想法                                  │
└──────────────────────┬──────────────────────────────────────────────┘
                       │ 读取 + 解析 (支持加密挂载)
                       ▼
┌─────────────────────────────────────────────────────────────────────┐
│                    openfang-telos crate (新增)                        │
│                                                                      │
│  TelosEngine          ← 文件监听 + 缓存 + 热更新                        │
│  TelosContext         ← 结构化内存表示 (10个文件 → Rust struct)          │
│  HandInjector         ← 上下文注入引擎                                 │
└──────────────┬──────────────────────────────────────────────────────┘
               │ inject(hand_name, ctx, system_prompt)
               ▼
┌─────────────────────────────────────────────────────────────────────┐
│                     openfang-hands crate (修改)                       │
│                                                                      │
│  HAND.toml 新增:                                                     │
│  [telos]                                                             │
│  mode = "focused"          ← Researcher: Full; Lead: Minimal         │
│  files = ["goals", "projects", "challenges"]  ← Custom 模式          │
│  max_chars = 3000          ← 防止 prompt 过长                          │
│  directive = "始终根据用户目标对发现排序优先级"                            │
└──────────────┬──────────────────────────────────────────────────────┘
               │ enriched system prompt
               ▼
┌─────────────────────────────────────────────────────────────────────┐
│                        LLM 执行层                                      │
│  [TELOS 上下文块]                                                     │
│  + [原始 Hand 系统提示 (500+ 字专家流程)]                                │
│  + [SKILL.md 技能文档]                                                │
│  = 知道用户是谁的专业 Agent                                             │
└─────────────────────────────────────────────────────────────────────┘
```

---
## 四、TELOS 注入机制详细设计

### 4.1 注入模式 (InjectionMode)

不同 Hand 需要不同深度的用户上下文：

```toml
# HAND.toml 中的 [telos] 块

# Full 模式：Researcher、Collector、Predictor
# 注入全部 10 个文件
[telos]
mode = "full"
max_chars = 6000

# Focused 模式（默认）：大多数 Hand
# 注入 Mission + Goals + Projects + Challenges
[telos]
mode = "focused"
max_chars = 3000

# Minimal 模式：Lead、Twitter、Clip
# 只注入 Mission + 活跃目标列表
[telos]
mode = "minimal"
max_chars = 800

# Custom 模式：精确控制
[telos]
mode = "custom"
files = ["goals", "projects", "strategies"]
max_chars = 2000
directive = "在生成 Twitter 内容时，严格遵循用户的内容策略风格"

# 关闭（某些 Hand 不需要个人上下文，如 Clip 视频剪辑）
[telos]
mode = "none"
```

### 4.2 注入位置 (InjectionPosition)

```
before_prompt (默认):
┌─────────────────────────┐
│ [TELOS 上下文块]          │ ← 先建立用户认知
│ [Hand 原始系统提示]        │
└─────────────────────────┘

after_prompt:
┌─────────────────────────┐
│ [Hand 原始系统提示]        │
│ [TELOS 上下文块]          │ ← 在操作指令后加入个人约束
└─────────────────────────┘

placeholder:
Hand 系统提示中包含 {{TELOS}} 标记，精确控制插入位置
```

### 4.3 注入块的渲染格式

```markdown
─────────────────────────────────────────────
# 用户上下文 (TELOS — 个人目标系统)

> 以下是你服务的用户的完整上下文。
> 你的所有行动、发现、输出都应与这些目标对齐。

## 使命
[MISSION.md 内容]

## 当前目标
[GOALS.md 的 Active Goals 章节]

## 进行中的项目
[PROJECTS.md 摘要]

## 当前挑战
[CHALLENGES.md 内容]

---
*TELOS 版本: 2025-03-01 | 加载 8/10 文件*
─────────────────────────────────────────────

[Hand 原始系统提示在这里继续...]
```

### 4.4 上下文优先级算法

当 TELOS 上下文长度超过 `max_chars` 时，按以下优先级截断：

```
Priority 1: MISSION.md       (不可截断，保留全文)
Priority 2: GOALS.md         (Active Goals 部分优先)
Priority 3: PROJECTS.md      (摘要模式，仅项目名称+状态)
Priority 4: CHALLENGES.md    (关键阻碍)
Priority 5: STRATEGIES.md    (仅策略标题)
Priority 6: BELIEFS.md       (简化版)
Priority 7: LEARNED.md       (最近 5 条)
Priority 8: MODELS.md        (标题列表)
Priority 9: NARRATIVES.md    (仅摘要)
Priority 10: IDEAS.md        (标题列表)
```

---

### 4.5 安全性与隐私保护 (新增)

TELOS 包含用户极度私密的个人目标与价值观。

1.  **端侧解析**：所有的 TELOS 解析、过滤和注入逻辑都在本地 Rust 运行时完成，不经过任何中间服务器。
2.  **隐私分级**：支持在文件内通过 HTML 注释标记敏感字段，例如 `<!-- PRIVATE START --> 敏感内容 <!-- PRIVATE END -->`，这些内容仅在使用本地 LLM 或受信任的 Provider 时才会注入。
3.  **可选加密**：Phase 3 将支持对 `~/.octarq/telos/` 目录进行 AES-256 加密存储，启动时需通过 `octarq unlock` 解锁。
4.  **透明性**：用户可随时通过 `octarq telos preview` 查看即将发送给 LLM 的具体脱敏上下文。

---

## 五、TELOS 注入机制详细设计


### 5.1 7 个 Hands 的个性化注入策略

### Researcher Hand：Full 模式

Researcher 的价值在于"深度理解"，需要完整上下文。

**注入字段：** 全部 10 个文件

**关键 directive：**
```
在研究时：
1. 将所有发现与用户的 GOALS.md 中的目标相关联
2. 对 CHALLENGES.md 中的挑战给予特别关注
3. 在 PROJECTS.md 中找到当前项目，优先产出可立即行动的数据
4. 用 MODELS.md 中的思维框架来组织你的分析结构
5. 报告结尾标注哪些发现与用户使命最高度对齐
```

**效果：** Researcher 不再产出泛泛的报告，而是"针对一个正在准备 B 轮融资、核心挑战是差异化叙事的 SaaS 创始人"的精准研究报告。

---

### Lead Hand：Minimal 模式

Lead 的 ICP（理想客户画像）直接来自 TELOS。

**注入字段：** MISSION + GOALS + PROJECTS

**关键 directive：**
```
从 PROJECTS.md 中提取用户的产品定位和目标客户描述。
将这作为你的 ICP 定义源，而不是要求用户重新填写 ICP 表单。
目标客户应该是能帮助用户实现 GOALS.md 中目标的人。
```

**效果：** 用户不需要每次激活 Lead Hand 都重新配置 ICP，TELOS 就是 ICP 的动态来源。当用户在 PROJECTS.md 更新产品方向，Lead Hand 自动调整潜客标准。

---

### Collector Hand：Full 模式（重点强化）

Collector 是最能受益于 TELOS 的 Hand——持续监控的靶标直接来自用户目标。

**注入字段：** 全部，重点 GOALS + PROJECTS + CHALLENGES

**关键 directive：**
```
自动构建监控靶标列表：
- PROJECTS.md 中每个项目 → 监控相关竞品和技术动态
- CHALLENGES.md 中每个挑战 → 寻找解决方案和案例
- GOALS.md 中每个目标 → 监控成功路径上的关键信号
不要等用户手动指定监控目标，从 TELOS 自动推导。
```

**效果：** 用户激活 Collector，它自动知道应该监控哪些竞品、哪些市场信号、哪些行业人物，无需任何手动配置。

---

### Predictor Hand：Full + MODELS 优先

Predictor 的推理框架应该使用用户自己相信的思维模型。

**注入字段：** MODELS + BELIEFS + STRATEGIES + GOALS

**关键 directive：**
```
在构建预测推理链时，优先使用 MODELS.md 中用户认可的思维框架。
对与 BELIEFS.md 中价值观冲突的预测结论，增加专门的对齐分析段落。
Brier Score 追踪应关联到 GOALS.md 中的目标进度。
```

**效果：** 预测报告使用用户熟悉的分析框架，增加信任度和可行性。

---

### Twitter Hand：Minimal + STRATEGIES

品牌声音来自 TELOS，不依赖每次手动配置风格。

**注入字段：** MISSION + STRATEGIES + NARRATIVES (摘要)

**关键 directive：**
```
STRATEGIES.md 定义了用户的内容策略偏好（语气、格式、话题）。
NARRATIVES.md 提供可引用的个人故事素材。
MISSION.md 决定内容主题边界——不发布与使命无关的内容。
所有内容都需要经过审批队列，不自动发布。
```

**效果：** Twitter Hand 产出的内容风格一致，且与用户的个人品牌建设目标直接挂钩。

---

### Browser Hand：Focused + STRATEGIES

Browser 执行 Web 操作时需要了解用户的行为偏好和限制。

**注入字段：** GOALS + STRATEGIES + BELIEFS

**关键 directive：**
```
BELIEFS.md 中可能包含用户的伦理边界（如"我不做竞品攻击"）。
在执行任何操作前检查是否与这些信念冲突。
购买审批门槛：任何支出都需要关联到 GOALS.md 中的目标。
```

---

### Clip Hand：None 模式

视频剪辑是纯创意/技术任务，个人目标对剪辑质量无帮助，注入反而增加噪音。

```toml
[telos]
mode = "none"
```

### 5.2 注入模式 (InjectionMode)

```toml
[telos]
mode = "full" | "focused" (默认) | "minimal" | "custom" | "none"
max_chars = 4000
position = "before_prompt" | "after_prompt"
```

### 5.3 上下文优先级算法

当超过 `max_chars` 时，按以下顺序保留：
`MISSION` > `GOALS` (Active) > `PROJECTS` > `CHALLENGES` > 其他。

---

## 六、TELOS 与 Octarq Memory 系统的协作

Octarq 已有 SQLite + 向量内存。TELOS 不替换它，而是**在其之上增加一个目标感知层**。

### 6.1 TELOS 版本化

```
openfang-memory crate 新增:
  telos_snapshots 表
  ├── snapshot_id
  ├── created_at
  ├── mission_hash      ← 检测 MISSION.md 变化
  ├── goals_hash        ← 检测目标变化
  └── full_context_json ← 完整快照

作用：让 Hand 知道"上次运行时用户的目标是 X，
现在目标变成了 Y，需要更新我的监控策略"
```

### 6.2 目标对齐评分

```
每次 Hand 完成任务后，Merkle 审计链记录:
{
  "hand": "researcher",
  "task": "竞品分析",
  "telos_alignment": {
    "goal_match": ["融资叙事准备", "市场差异化"],
    "alignment_score": 0.87,
    "telos_version": "2025-03-01"
  }
}
```

这让用户可以查询："过去 30 天，哪些 Hand 任务与我的目标最对齐？"

### 6.3 LEARNED.md 自动更新

```
Hand 完成高价值任务 → 提取关键洞察 → 追加到 LEARNED.md

例如：Researcher Hand 发现竞品 X 刚完成 B 轮
→ 自动追加到 LEARNED.md:
  "2025-03-01: [Researcher] 竞品 X B 轮后可能加速进入企业市场，
   需要在 Q2 之前建立差异化护城河"
```

---

## 七、PAI Skills 到 Octarq 的迁移路径

PAI 的 18 个 Skills 通过 SKILL.md 格式天然兼容 Octarq。

### 直接迁移（无需修改）

| PAI Skill | 目标 Octarq Hand/Skill | 迁移工作量 |
|-----------|----------------------|---------|
| `pai-research-skill` | Researcher SKILL.md 扩展 | 低 |
| `pai-osint-skill` | Collector SKILL.md 扩展 | 低 |
| `pai-brightdata-skill` | Browser Hand 工具扩展 | 低 |
| `pai-prompting-skill` | 所有 Hand 的 meta-prompting | 低 |
| `pai-firstprinciples-skill` | Predictor 推理框架 | 低 |

### 需要适配的 Skills

| PAI Skill | 适配挑战 | 方案 |
|-----------|---------|------|
| `pai-redteam-skill` (32 Agent) | Octarq 的 agent 生命周期不同 | 用 Octarq 的 A2A 协议重新实现 agent 间通信 |
| `pai-council-skill` (多 agent 辩论) | 同上 | 同上 |
| `pai-agents-skill` | PAI 依赖 Claude Code 的 sub-agent | 映射到 Octarq 的 workflow 调度器 |
| `pai-telos-skill` | PAI 版本管理 TELOS 文件 | 作为 `octarq telos` CLI 子命令实现 |

### 新增 Skills（PAI 没有，Octarq 特有价值）

| 新 Skill | 描述 |
|----------|------|
| `channel-routing-skill` | 根据 TELOS 上下文决定通知发送到哪个渠道 |
| `telos-review-skill` | 定期检查 TELOS 文件是否需要更新，主动提示用户 |
| `goal-progress-skill` | 汇总所有 Hand 的活动，生成目标进度报告 |

---

## 八、CLI 扩展设计

### 新增 `octarq telos` 子命令

```bash
# 初始化 TELOS 目录，交互式引导填写
octarq telos init

# 查看当前加载的 TELOS 状态
octarq telos status

# 编辑某个 TELOS 文件（调用 $EDITOR）
octarq telos edit goals

# 强制重新加载（文件修改后）
octarq telos reload

# 查看某个 Hand 会注入哪些 TELOS 内容
octarq telos preview researcher

# 目标对齐报告：过去 N 天 Hand 活动与目标的相关性
octarq telos report --days 30

# 导出 TELOS 为 JSON（用于备份/迁移）
octarq telos export > telos-backup.json
```

### `octarq telos preview` 输出示例

```
$ octarq telos preview researcher

━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
TELOS 注入预览 — Researcher Hand (mode: full)
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

注入位置: before_prompt
注入字符数: 2,847 / max 6,000
TELOS 文件: 8/10 已加载 (缺少 NARRATIVES.md, LEARNED.md)

─── 注入块预览 ──────────────────────────────────────
# 用户上下文 (TELOS)

## 使命
构建让创业者节省时间的工具，让人们专注于真正重要的事。

## 当前目标
- [ ] 2025 Q2 完成 ProjectFlow MVP 并获得 100 付费用户
- [ ] 2025 年完成 Pre-A 融资 (目标 $500K)
- [x] 组建 3 人核心团队

## 进行中的项目
- ProjectFlow (B2B 项目管理 SaaS) — 开发中，目标 Q2 Beta
- 融资材料准备 — 路演 PPT 60% 完成

## 当前挑战
- 需要更有力的市场差异化数据支撑融资叙事
- 竞品 Notion 最近更新了项目功能，需要了解影响
─────────────────────────────────────────────────────

[Researcher Hand 原始 500 字系统提示接在这里...]
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
```


### `octarq telos init` (渐进式引导)

- **Quick Start**: 仅初始化 `MISSION.md` 和 `GOALS.md`。适合新用户。
- **Full Profile**: 初始化全部 10 个模板。适合深度用户。

### 其他子命令
- `octarq telos status`: 显示加载状态及“数据新鲜度”提醒。
- `octarq telos preview <hand>`: 实时预览注入效果（含 TUI 仪表盘显示对齐分）。
- `octarq telos edit <file>`: 调用系统编辑器修改。


---

## 九、TELOS 模板文件设计

### MISSION.md 模板

```markdown
# 使命 (Mission)

<!-- 用一到两句话描述你存在的意义和方向。这是你的北极星。-->
<!-- 示例："帮助创业者用技术杠杆节省时间，专注于创造真实价值" -->

[你的使命陈述]

## 背景
<!-- 是什么让你走上这条路？简短的起源故事 -->

## 核心价值观
<!-- 2-4 条你在工作中绝不妥协的原则 -->
-
-

## 成功的样子
<!-- 10 年后，如果你的使命实现了，世界会有什么不同？ -->
```

### GOALS.md 模板

```markdown
# 目标 (Goals)

## 活跃目标
<!-- 当前正在追求的目标，按优先级排序。使用 - [ ] 未完成，- [x] 已完成 -->

- [ ] [目标 1] — 截止: [日期]
- [ ] [目标 2] — 截止: [日期]

## 季度目标 (Q[N] [年份])
<!-- 本季度要完成的具体可量化目标 -->

- [ ]
- [ ]

## 年度目标 ([年份])
<!-- 今年底要达成的里程碑 -->

- [ ]

## 暂缓目标
<!-- 重要但暂时搁置的目标 -->

- [目标] — 搁置原因: [原因]

## 已完成
<!-- 记录成就，增强动力 -->

- [x] [目标] — 完成于 [日期]
```

### PROJECTS.md 模板

```markdown
# 项目 (Projects)

## [项目名称]

**状态:** [开发中 / 规划中 / 暂停 / 完成]
**目标关联:** [关联到 GOALS.md 中哪个目标]
**关键里程碑:**
- [ ] [里程碑 1] — 目标日期: [日期]
- [ ] [里程碑 2]

**当前阻碍:**
- [阻碍描述]

**关键指标:**
- [KPI 1]: [当前值] → [目标值]

**竞品/参考:**
- [竞品 1]: [一句话描述与自己产品的差异]

---
```

### CHALLENGES.md 模板

```markdown
# 挑战 (Challenges)

<!-- 当前面临的关键障碍和难题。诚实记录，这会让 Agent 更有针对性地帮助你。-->

## 当前挑战

### [挑战标题]
**类型:** [技术 / 市场 / 团队 / 资源 / 认知]
**紧迫程度:** [高 / 中 / 低]
**描述:** [1-3 句话描述]
**已尝试方案:** [什么行不通]
**需要的帮助:** [你需要什么类型的支持/信息]

---
```

---

## 十、实施路线图

### Phase 1：基础 TELOS 引擎（2 周）

- 创建 `openfang-telos` crate（文件加载、解析、缓存）
- 实现 `octarq telos init/status/edit/reload` CLI
- 完成 TELOS 模板文件套装（10 个 .md 模板 + 中英文版本）
- 单元测试：解析正确性、错误处理、热更新

### Phase 2：Hand 注入集成（1.5 周）

- 修改 `openfang-hands` crate：HAND.toml 解析新增 `[telos]` 块
- 实现 `HandInjector`：4 种 injection mode + position 控制
- 优先集成：Researcher Hand、Collector Hand、Lead Hand
- 实现 `octarq telos preview <hand>` CLI

### Phase 3：Memory 协作（1 周）

- `openfang-memory` 新增 `telos_snapshots` 表
- Merkle 审计链记录 `telos_alignment` 元数据
- 实现 `octarq telos report` 目标进度报告

### Phase 4：Skills 迁移（持续）

- 迁移 PAI 5 个直接兼容 Skills（research、osint、prompting 等）
- 开发 3 个 Octarq 特有新 Skills（channel-routing、telos-review、goal-progress）
- FangHub 发布 `telos-pack`

---

## 十一、设计决策记录 (ADR)

**ADR-001：TELOS 文件用 Markdown，不用 TOML/JSON**

理由：TELOS 文件由人类写，给人类读，Markdown 降低填写摩擦。Parser 解析 Markdown 的成本可接受，且 PAI 社区生态已在 Markdown 格式上标准化。

**ADR-002：注入是 opt-in，不是 opt-out**

理由：强制注入对 Clip 等无关 Hand 增加无意义 token 消耗。HAND.toml 中必须显式声明 `[telos]` 块，默认 `mode = "none"`。（与 PAI 精神不同，PAI 默认 opt-in，但 Octarq 作为通用平台更保守）

**ADR-003：TELOS 目录位于 `~/.octarq/telos/`，而非项目目录**

理由：TELOS 是个人身份档案，不是项目配置。用户可能有多个 Octarq 项目/实例，但 TELOS 应该是单一真相来源。

**ADR-004：先不实现 TELOS 自动写回（AI 自动修改用户的 TELOS 文件）**

理由：自动写回会导致 TELOS 漂移和信任问题。Phase 1-3 中，AI 只读取 TELOS，不写入。LEARNED.md 自动更新作为 Phase 4 的可选特性，需要明确的用户授权。

**ADR-005：注入字符上限默认 4000 chars，可配置**

理由：Octarq 支持 27 个 LLM，各模型 context window 不同。4000 chars 约 1000 tokens，对所有模型（包括 8K context 的小模型）安全。高端用户可在 HAND.toml 中调高。

---

## 十二、成功指标

集成完成后，衡量价值的关键指标：

| 指标 | 衡量方式 | 目标 |
|------|---------|------|
| 配置时间减少 | 用户首次激活 Hand 到有效输出的时间 | 从 ~5 分钟降至 <30 秒 |
| 输出相关性 | 用户对 Hand 输出的 👍/👎 比率 | 提升 40%+ |
| TELOS 填写率 | 新用户完成 TELOS init 的比例 | >60% |
| 目标对齐分 | 每月 Hand 活动的 telos_alignment_score 均值 | >0.75 |
| TELOS 更新频率 | 用户主动更新 TELOS 文件的频率 | 每 2 周至少 1 次 |

---

## 十三、致谢

本集成方案的设计深度参考了 [Daniel Miessler](https://github.com/danielmiessler) 的 [Personal_AI_Infrastructure (PAI)](https://github.com/danielmiessler/Personal_AI_Infrastructure) 项目。感谢 Daniel 对个人 AI 基础设施及 TELOS 系统的前瞻性思考与开源贡献。

---

*文档版本: 1.1 | 基于 PAI v2.5 + Octarq v0.2.1 分析*