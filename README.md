# 🔍 PrefixSleuth

DeepSeek API 请求代理调试工具。透传请求并记录，自动对比相邻请求的 prompt 差异，追踪缓存命中率变化，帮你找出哪些 token 前缀变了导致缓存失效。

## 功能

- **透明代理** — 暴露 OpenAI 兼容端口，客户端无感接入
- **流式透传** — SSE 流式响应完整双写，不丢失数据
- **Prompt Diff** — 相同下游 Key 的相邻请求自动对比 prompt 差异
- **缓存追踪** — 解析 DeepSeek `usage.prompt_cache_hit_tokens`，实时展示命中率
- **多客户端** — 下游虚拟 Key 区分不同来源（ChatBox / 脚本等）
- **事件驱动** — 后端实时推送到前端，无轮询
- **持久化** — 请求日志和配置自动保存到 `./data/`

## 使用

### 设置

1. 启动 PrefixSleuth
2. 点「设置」，填入：
   - **上游 API Key** — 你的 DeepSeek API Key
   - **下游 Key** — 客户端用的虚拟 Key + 标签（如 `sk-chatbox` → ChatBox）
3. 点「保存」→ 点顶栏「运行中」按钮启动代理

### 客户端配置

```python
from openai import OpenAI

client = OpenAI(
    base_url="http://localhost:9527",
    api_key="sk-chatbox-key"  # 你在下游 Key 里配的
)
```

请求会自动转发到 DeepSeek，所有记录在 UI 中实时查看。

### 分析缓存

1. 发送两条或更多请求（保持相同 system prompt 前缀）
2. 在请求列表中点击任意一条
3. 默认显示 **Prompt Diff** — 高亮出新增/删除/修改的部分
4. 头部进度条显示缓存命中率
5. 顶栏趋势微条直观对比多条请求的缓存表现

## 开发

```bash
# 依赖
npm install

# 开发模式
npm run tauri dev

# 构建
npm run tauri build
```

## 技术栈

- **前端**: React + TypeScript + Tailwind CSS + Tauri v2
- **后端**: Rust + hyper 1.x + reqwest + tokio
- **存储**: JSONL 日志文件 + JSON 配置文件

## 协议

MIT © 2026 1cyberlangke1
