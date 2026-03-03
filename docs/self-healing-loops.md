# 错误恢复与自愈逻辑（Self-healing Loops）优化

针对 LLM 在工具调用阶段产生的“幻觉”（如参数格式错误、调用不存在的函数等），在现有的 7 阶段 Session Repair 基础上增加闭环反馈机制。

## 2.1 智能错误截获器 (Hallucination Interceptor)

在 openfang-runtime 的 Tool 执行引擎中引入“预校验-执行-反馈”循环：

- **动态 Schema 校验**
  - 在调用工具前，使用 JSON Schema 强制校验。若校验失败，不直接报错，而是进入自愈流程。
- **反馈 Prompt 生成**
  - 自动生成描述性极强的错误提示（例如：“你提供的 api_key 参数位置错误，它应该在 header 中而不是 body 中”），将其作为一条特殊的 system_feedback 消息回填给 LLM。

## 2.2 三级防御恢复策略

1. **Level 1: 原位修正 (In-place Correction)**
   - 允许 LLM 对同一任务进行最多 2 次静默重试。如果第二次仍失败，升级策略。
2. **Level 2: 批判者介入 (Critic Agent Intervention)**
   - 唤起内部的 critic 逻辑，分析前两次失败的上下文，由更高级别的模型（或专门的调试 Prompt 链）重新生成正确的调用指令。
3. **Level 3: 熔断与状态回滚 (Circuit Breaker & Rollback)**
   - 若自愈循环超过阈值，立即触发熔断。
   - 利用 Merkle 审计链记录的快照，将 Session 状态回滚至上一个已知的“干净” Checkpoint，并通知用户手动干预或切换 Agent。

## Next

- 错误集收集: 整理过去运行中常见的 LLM 幻觉模式，构建“错误反馈模板库”以提高 Level 1 修正的成功率。

---

## 具体实施计划（针对 Octarq 代码库）

### 现状摘要

| 组件 | 位置 | 现状 |
|------|------|------|
| Session Repair | `openfang-runtime/src/session_repair.rs` | 已有：orphan 清理、空消息、同角色合并、synthetic result 等，在 `agent_loop` 每次迭代前调用 |
| 工具执行入口 | `openfang-runtime/src/agent_loop.rs` ~524–666 行 | 直接 `execute_tool(...)`，结果 `is_error` 时仅作为 ToolResult 写回，无重试/自愈 |
| 工具 Schema | `openfang-types/src/tool.rs` | `ToolDefinition.input_schema` 为 `serde_json::Value`，已有 JSON Schema，未做调用前校验 |
| 重试基础设施 | `openfang-runtime/src/retry.rs` | 通用 `retry_async` + 指数退避，可用于 LLM/网络，尚未用于工具幻觉重试 |
| 熔断/循环检测 | `openfang-runtime/src/loop_guard.rs` | 已有 tool 循环的 circuit break，无“自愈次数”熔断 |
| 上下文恢复 | `openfang-runtime/src/context_overflow.rs` | 4 阶段 overflow 恢复，无 session 快照回滚 |
| Merkle 审计 | `openfang-runtime/src/audit.rs` | 按动作追加审计链，无 session 快照存储 |

### Phase 1：预校验 + 反馈消息（Level 1 基础）

1. **新增 JSON Schema 校验**
   - 在 `openfang-runtime` 增加依赖（如 `jsonschema` 或手写基于 `input_schema` 的校验）。
   - 在 `agent_loop` 中，在调用 `execute_tool` 之前：
     - 根据 `available_tools` 找到当前 `tool_call.name` 对应的 `ToolDefinition`，取其 `input_schema`。
     - 用 schema 校验 `tool_call.input`；若失败，**不执行工具**，进入“自愈分支”。
   - 自愈分支：构造一条**描述性错误**（如 “Parameter 'url' is required but missing. Schema: …” 或 “'limit' must be a number, got string.”），作为 `ContentBlock::ToolResult { content: feedback, is_error: true }` 插入，并增加“同一 tool_use_id 的静默重试计数”。

2. **同一任务的静默重试（Level 1 次数）**
   - 在 agent loop 的**每轮迭代**内维护：`tool_use_id -> 重试次数`（或按 `(tool_name, 当前 assistant 消息内顺序)` 计数）。
   - 当某次工具调用因 **schema 校验失败** 或 **执行返回 is_error** 且判定为“可重试”时：
     - 若该 call 的重试次数 < 2：将反馈 ToolResult 追加到 `messages`，**继续下一轮 LLM 调用**（不返回用户，不增加迭代上限外的惩罚）。
     - 若已满 2 次：升级到 Level 2 或直接返回错误给用户（见下）。

3. **可重试判定**
   - 在 `tool_runner::execute_tool` 或 agent_loop 侧：根据 `ToolResult.content` 或错误类型打标签（如 capability denied、timeout、schema/format 错误）。仅对“幻觉类”（参数错误、错误工具名等）计入 Level 1 重试；permission/taint/timeout 不重试。

**交付物**  
- `openfang-runtime` 内新模块 `tool_schema_validate.rs`（或放在 `tool_runner` 中）：`validate_tool_input(schema, input) -> Result<(), String>`。  
- `agent_loop.rs`：在 `execute_tool` 前调用校验；失败时生成 feedback、写入 ToolResult、维护 per-call 重试计数并决定 continue 或升级。

### Phase 2：Critic 介入（Level 2）

1. **触发条件**
   - 同一逻辑任务（同一 user turn 内同一工具名 + 同一意图）已用满 Level 1 的 2 次重试仍失败（schema 或执行错误）。

2. **Critic 实现二选一**
   - **A. 轻量**：在 `openfang-runtime` 内增加一次“修复专用”的 LLM 调用：system prompt 为“你是指令修复专家。根据以下工具定义、错误信息与最近一次错误调用，输出**唯一**正确的 JSON 对象，不要解释。”，输入为 `tool_def + 最后一条错误 ToolResult + 最后一条 ToolUse`，输出解析为 `tool_call.input` 再执行一次。
   - **B. 重量**：在 `openfang-kernel` 或独立 crate 中引入“Critic Agent”（可复用现有 agent 框架），专门处理 tool-call 修复请求。

3. **与现有 loop 的衔接**
   - Critic 若成功：用新 `input` 执行工具，得到成功结果后，将该 ToolUse + ToolResult 正常插入 session，继续主 agent loop。
   - Critic 若失败或超时：进入 Level 3。

**交付物**  
- 配置项：如 `[runtime] self_healing_critic = true`，以及可选的 critic 模型/endpoint。  
- `openfang-runtime` 中 `critic.rs` 或扩展现有 `llm_driver` 调用：单次“修复调用” + 结果解析与一次 re-execute。

### Phase 3：熔断与 Session 回滚（Level 3）

1. **自愈次数熔断**
   - 在单次 agent run（一次 `run_agent_loop` 调用）内维护：**总自愈次数**（Level 1 重试 + Level 2 调用）。
   - 当总自愈次数超过阈值（如 5）时：触发熔断，不再重试，并进入回滚流程。

2. **Session Checkpoint**
   - 当前代码没有“会话快照”。需要新增：
     - 在 **每次 LLM 回合开始前**（或每轮 tool 执行前）将当前 `session.messages`（或至少其哈希/长度）与关键状态写入“checkpoint”。
     - Checkpoint 存储：可放在内存（`Vec<(iteration, SessionSnapshot)>`），或写入 `openfang-memory` 的临时表/结构；Merkle 链可**记录 checkpoint 的引用**（如 hash(session_state)）作为审计条目，而不是把整段 session 放进现有 `AuditEntry`。
   - 文档中“利用 Merkle 审计链记录的快照”可落实为：**每次 checkpoint 时 append 一条 AuditAction::SessionCheckpoint，detail 含 checkpoint_id 或 state_hash**，回滚时根据 checkpoint_id 取回对应 session 状态。

3. **回滚与通知**
   - 熔断时：从最近一次“干净”的 checkpoint 恢复 `session.messages`（及必要元数据），写回 memory；可选将“自愈失败，已回滚”通过现有 channel 或 API 通知用户（与现有 AgentLoopEnd hook 结合）。

**交付物**  
- `openfang-runtime`：自愈计数器 + 熔断阈值（可配置）；在 agent_loop 内每轮迭代前 checkpoint；熔断时恢复并可选触发 hook/通知。  
- `openfang-runtime/src/audit.rs`：新增 `AuditAction::SessionCheckpoint`，记录 checkpoint 标识或 state_hash。  
- 若 checkpoint 存 memory：在 `openfang-memory` 中定义轻量“session_checkpoint”存储（或先用内存，后续再持久化）。

### Phase 4：错误反馈模板库（Next 落地）

1. **收集与分类**
   - 从现有日志/测试中整理：schema 校验失败、错误工具名、参数类型/必填缺失、权限/taint 等。
   - 为每类错误定义“反馈模板”，例如：`"Parameter '{}' is required but missing. Expected type: {}."`。

2. **集成到 Level 1**
   - 在生成“描述性错误”时，优先匹配模板库，再 fallback 到通用描述，提高 LLM 理解率与 Level 1 修正成功率。

**交付物**  
- `openfang-runtime` 内 `tool_error_templates.rs` 或 TOML/JSON 配置：错误类型 -> 模板字符串。  
- 预校验/执行错误分支中调用模板生成 feedback。

### 建议实现顺序

1. **Phase 1**：预校验 + 反馈 + Level 1 静默重试（2 次）。不依赖 Critic 与 checkpoint，可独立上线并观察“幻觉”减少比例。  
2. **Phase 4（部分）**：先做少量高频错误模板，与 Phase 1 一起用。  
3. **Phase 3**：熔断 + checkpoint + 回滚（可先内存 checkpoint，Merkle 只记 hash）。避免恶性循环。  
4. **Phase 2**：Critic 介入。在 Level 1/3 稳定后再加，避免增加复杂度导致难以排查。

### 配置建议（后续可加）

```toml
[runtime.self_healing]
enabled = true
level1_max_retries = 2
level2_critic_enabled = true
level3_circuit_break_after = 5
checkpoint_every_iteration = true
```

### 测试要点

- 单元：`validate_tool_input` 对合法/非法 input 的校验结果。  
- 集成：构造“错误 tool call”的 session，跑一轮 loop，断言出现 1 次反馈 + 1 次重试后成功或升级。  
- 熔断：构造连续 5 次可重试错误，断言触发熔断并回滚到上一 checkpoint（消息条数/最后一条内容一致）。
