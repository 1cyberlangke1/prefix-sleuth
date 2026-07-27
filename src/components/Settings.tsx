import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import type { DownstreamKey, ProxyConfig } from "../types";

function Settings() {
  const [config, setConfig] = useState<ProxyConfig | null>(null);
  const [saving, setSaving] = useState(false);
  const [message, setMessage] = useState("");

  useEffect(() => {
    invoke<ProxyConfig>("get_config").then(setConfig);
  }, []);

  const updateDownstream = (i: number, field: keyof DownstreamKey, value: string) => {
    if (!config) return;
    const keys = [...config.downstream_keys];
    keys[i] = { ...keys[i], [field]: value };
    setConfig({ ...config, downstream_keys: keys });
  };

  const addDownstream = () => {
    if (!config) return;
    setConfig({
      ...config,
      downstream_keys: [...config.downstream_keys, { key: "", label: "" }],
    });
  };

  const removeDownstream = (i: number) => {
    if (!config) return;
    setConfig({
      ...config,
      downstream_keys: config.downstream_keys.filter((_, idx) => idx !== i),
    });
  };

  const save = async () => {
    if (!config) return;
    setSaving(true);
    setMessage("");
    try {
      await invoke("update_config", { config });
      setMessage("保存成功");
    } catch (e) {
      setMessage(`保存失败: ${e}`);
    }
    setSaving(false);
  };

  if (!config) {
    return (
      <div className="flex-1 flex items-center justify-center text-text-muted text-sm">
        加载中...
      </div>
    );
  }

  return (
    <div className="flex-1 overflow-auto p-6 max-w-3xl">
      <h2 className="text-base font-bold mb-6 text-accent-blue">代理设置</h2>

      <div className="space-y-5">
        <div>
          <label className="block text-xs font-medium text-text-secondary mb-1.5">
            DeepSeek API 地址
          </label>
          <input
            type="text"
            className="w-full bg-surface-1 text-text-primary border border-surface-0/50 rounded px-3 py-2 text-sm outline-none focus:border-accent-blue transition-colors"
            value={config.upstream_base}
            onChange={(e) => setConfig({ ...config, upstream_base: e.target.value })}
            placeholder="https://api.deepseek.com"
          />
          <p className="text-[11px] text-text-muted mt-1">转发目标的基础 URL</p>
        </div>

        <div>
          <label className="block text-xs font-medium text-text-secondary mb-1.5">
            本地代理端口
          </label>
          <input
            type="number"
            className="w-32 bg-surface-1 text-text-primary border border-surface-0/50 rounded px-3 py-2 text-sm outline-none focus:border-accent-blue transition-colors"
            value={config.local_port}
            onChange={(e) => setConfig({ ...config, local_port: parseInt(e.target.value) || 9527 })}
          />
          <p className="text-[11px] text-text-muted mt-1">
            本地监听的端口，客户端连接这个端口
          </p>
        </div>

        <div>
          <label className="block text-xs font-medium text-text-secondary mb-1.5">
            上游 API Key（真正的 DeepSeek Key）
          </label>
          <input
            type="password"
            className="w-full bg-surface-1 text-text-primary border border-surface-0/50 rounded px-3 py-2 text-sm outline-none focus:border-accent-blue transition-colors font-mono"
            value={config.upstream_api_key}
            onChange={(e) => setConfig({ ...config, upstream_api_key: e.target.value })}
            placeholder="sk-xxxxxxxxxxxxxxxx"
          />
          <p className="text-[11px] text-text-muted mt-1">
            所有客户端请求都会使用这个 Key 转发给 DeepSeek
          </p>
        </div>

        <div>
          <label className="block text-xs font-medium text-text-secondary mb-1.5">
            下游 Key 列表（客户端连接时使用的虚拟 Key）
          </label>
          <p className="text-[11px] text-text-muted mb-2">
            每个 Key 对应一个客户端，方便区分请求来源。留空则透传原始 Authorization 头。
          </p>

          {config.downstream_keys.map((dk, i) => (
            <div key={i} className="flex items-center gap-2 mb-2">
              <input
                type="text"
                className="flex-1 bg-surface-1 text-text-primary border border-surface-0/50 rounded px-2 py-1.5 text-xs outline-none focus:border-accent-blue transition-colors font-mono"
                value={dk.key}
                onChange={(e) => updateDownstream(i, "key", e.target.value)}
                placeholder="sk-xxxxxx（客户端用的 Key）"
              />
              <input
                type="text"
                className="w-28 bg-surface-1 text-text-primary border border-surface-0/50 rounded px-2 py-1.5 text-xs outline-none focus:border-accent-blue transition-colors"
                value={dk.label}
                onChange={(e) => updateDownstream(i, "label", e.target.value)}
                placeholder="标签"
              />
              <button
                className="px-2 py-1.5 text-xs text-accent-red rounded hover:bg-accent-red/10 transition-colors"
                onClick={() => removeDownstream(i)}
              >
                删除
              </button>
            </div>
          ))}

          <button
            className="mt-1 px-3 py-1 text-xs text-accent-blue bg-accent-blue/10 rounded hover:bg-accent-blue/20 transition-colors"
            onClick={addDownstream}
          >
            + 添加下游 Key
          </button>
        </div>

        <div className="flex items-center gap-3 pt-2">
          <button
            className="px-4 py-1.5 bg-accent-blue text-surface-2 text-sm font-medium rounded hover:bg-accent-blue/90 transition-colors disabled:opacity-50"
            onClick={save}
            disabled={saving}
          >
            {saving ? "保存中..." : "保存"}
          </button>
          {message && (
            <span
              className={`text-xs ${
                message.includes("失败") ? "text-accent-red" : "text-accent-green"
              }`}
            >
              {message}
            </span>
          )}
        </div>

        <div className="pt-4 border-t border-surface-0/30">
          <h3 className="text-xs font-medium text-text-secondary mb-2">用法</h3>
          <div className="text-xs text-text-muted space-y-1.5 leading-relaxed">
            <p>
              将 OpenAI 客户端指向 <code className="text-accent-blue bg-surface-1 px-1 rounded">http://localhost:{config.local_port}</code>
            </p>
            <p>客户端使用你配置的下游 Key 连接：</p>
            <pre className="bg-surface-1 p-3 rounded mt-1 text-[11px] leading-relaxed">
{`# ChatBox
client = OpenAI(
    base_url="http://localhost:${config.local_port}",
    api_key="sk-chatbox-key"  # 你在下游 Key 里配的
)

# 脚本
client = OpenAI(
    base_url="http://localhost:${config.local_port}",
    api_key="sk-script-key"   # 另一个下游 Key
)`}
            </pre>
            <p className="text-accent-yellow">
              请求记录会按下游 Key 的标签分类显示，方便追踪不同客户端的缓存表现。
            </p>
          </div>
        </div>
      </div>
    </div>
  );
}

export default Settings;
