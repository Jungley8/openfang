# 链路追踪（Tracing）增强：从审计到洞察

目前系统具备 Merkle 审计链，主要解决「存证与合规」问题。本方案引入 OpenTelemetry (OTel) 体系，解决「性能瓶颈分析」与「跨 Agent 调用拓扑」的可视化问题。

## 1.1 分层追踪架构

在 openfang-kernel 和 openfang-runtime 中集成 tracing 库，并建立三级 Span 结构：

- **Root Span (Session)**：跟踪整个用户会话，承载 `session_id`、`agent_id`；结束时写入当前 `merkle_root`（审计链 tip），实现执行流与审计链的映射。
- **Agent Span**：记录单个 Agent 的思考（Reasoning）周期，包含 `agent_type`、`model_name`、`input_tokens`、`output_tokens`。
- **Tool/Task Span**：记录具体工具调用（`tool_name`）及耗时（由 span 生命周期隐式记录）。

## 1.2 关键实施策略

- **上下文透传 (Context Propagation)**  
  在 HTTP 入口（openfang-api）从请求头提取 W3C TraceContext（`traceparent`/`tracestate`），将提取的 Context 设为当前请求的父级，保证从 API 到 kernel/runtime 的链路连续；为未来分布式 runtime 预留 B3/W3C 注入。

- **自定义 Metadata 注入**  
  自动向 Span 注入 `agent_type`、`model_name`、`merkle_root`；将 Merkle 树的当前 Leaf Hash 与 Trace ID 关联。

- **导出器设置**  
  使用 `tracing-opentelemetry` + OTLP (gRPC)，通过环境变量配置 endpoint，支持将数据推送到 Jaeger（私有化部署）或 Honeycomb/Loki（云原生）。

## 2. 配置与环境变量

### 2.1 日志级别（RUST_LOG）

- 未设置时默认：`info`（CLI/Desktop 为 `octarq=info` 或 `octarq=info,tauri=info`）。
- 示例：`RUST_LOG=openfang_kernel=debug,openfang_runtime=debug` 可提高 kernel/runtime 的日志粒度。

### 2.2 OpenTelemetry 导出（OTLP）

- **OTEL_EXPORTER_OTLP_ENDPOINT**  
  设置后，CLI/Desktop 在启动时注册 OTLP trace exporter（gRPC），将 span 发送到该 endpoint。不设置则仅使用本地 fmt 输出，不发起网络请求。

- 示例（本地 Jaeger）：  
  `OTEL_EXPORTER_OTLP_ENDPOINT=http://127.0.0.1:4317`

- 其他可选环境变量（参见 OpenTelemetry 规范）：  
  `OTEL_EXPORTER_OTLP_TRACES_ENDPOINT`、`OTEL_SERVICE_NAME` 等，按需覆盖。

### 2.3 可选 config.toml

当前未在 `config.toml` 中增加 `[telemetry]` 段；如需关闭 OTLP 或配置采样率，可后续在此扩展，由 CLI/Desktop 启动时读取并覆盖环境变量。

## 3. 三级 Span 命名与属性约定

| Span 名称 | 位置 | 属性 |
|-----------|------|------|
| `session` | openfang-kernel `send_message_with_handle` | `session_id`, `agent_id`, `merkle_root`（结束时） |
| `agent` | openfang-runtime `run_agent_loop` / `run_agent_loop_streaming` | `agent_type`, `model_name`, `input_tokens`, `output_tokens` |
| `tool` | openfang-runtime `execute_tool` | `tool_name`；耗时由 span 起止时间表示 |

## 4. 如何配置后端接收 OTLP

- **Jaeger**：`docker run -p 16686:16686 -p 4317:4317 -e COLLECTOR_OTLP_ENABLED=true jaegertracing/all-in-one:latest`，然后设置 `OTEL_EXPORTER_OTLP_ENDPOINT=http://127.0.0.1:4317`，在 http://localhost:16686 查看 trace。
- **Honeycomb / Loki**：在对应控制台创建 OTLP (gRPC) 接入，将提供的 endpoint 和（如需要）API key 配置到 `OTEL_EXPORTER_OTLP_*` 环境变量。

## 5. 基准测试

- **场景**：完整路径 `send_message` → `execute_llm_agent` → `run_agent_loop`，使用 mock LLM（provider `mock`）控制变量，无真实网络。
- **对比**：无 OTel / 仅 OTel 内存（noop exporter）/ OTel + OTLP gRPC 导出。
- **指标**：同一请求的 P50/P99 延迟、吞吐（req/s）；评估 OTel 导出器对延迟的影响。

### 5.1 运行方式

```bash
# 无 OTel（仅 tracing + fmt）
BENCH_OTEL=none cargo bench -p openfang-kernel --bench tracing_bench --features bench

# 仅 OTel 内存（span 创建与批处理，不发送）
BENCH_OTEL=memory cargo bench -p openfang-kernel --bench tracing_bench --features bench

# OTel + OTLP gRPC（需先起 Jaeger 等接收端）
OTEL_EXPORTER_OTLP_ENDPOINT=http://127.0.0.1:4317 BENCH_OTEL=otlp cargo bench -p openfang-kernel --bench tracing_bench --features bench
```

Criterion 会输出 `send_message_none` / `send_message_memory` / `send_message_otlp` 的耗时分布（含 P50/P99）与吞吐；三次需分别运行并对比结果。

### 5.2 结果

待补充（完成三次运行后在此填写 P50/P99、吞吐及简短结论）。
