import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import type { ProxyConfig } from "../types";

function Settings() {
  const [config, setConfig] = useState<ProxyConfig | null>(null);
  const [saving, setSaving] = useState(false);
  const [message, setMessage] = useState("");
  const [keysText, setKeysText] = useState("");

  useEffect(() => {
    invoke<ProxyConfig>("get_config").then((cfg) => {
      setConfig(cfg);
      setKeysText(cfg.allowed_keys.join("\n"));
    });
  }, []);

  const save = async () => {
    if (!config) return;
    setSaving(true);
    setMessage("");
    try {
      const newConfig: ProxyConfig = {
        ...config,
        allowed_keys: keysText
          .split("\n")
          .map((k) => k.trim())
          .filter(Boolean),
      };
      await invoke("update_config", { config: newConfig });
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
            允许的 API Key（每行一个，留空则全部放行）
          </label>
          <textarea
            className="w-full h-28 bg-surface-1 text-text-primary border border-surface-0/50 rounded px-3 py-2 text-sm outline-none focus:border-accent-blue transition-colors font-mono"
            value={keysText}
            onChange={(e) => setKeysText(e.target.value)}
            placeholder="sk-xxxxxxxxxxxx"
          />
        </div>

        <div className="flex items-center gap-3">
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
            <p>SDK 示例：</p>
            <pre className="bg-surface-1 p-3 rounded mt-1 text-[11px] leading-relaxed">
{`from openai import OpenAI
client = OpenAI(
    base_url="http://localhost:${config.local_port}",
    api_key="sk-xxxx"
)`}
            </pre>
            <p>或者 curl：</p>
            <pre className="bg-surface-1 p-3 rounded mt-1 text-[11px] leading-relaxed">
{`curl http://localhost:${config.local_port}/v1/chat/completions \\
  -H "Content-Type: application/json" \\
  -H "Authorization: Bearer sk-xxxx" \\
  -d '{"model":"deepseek-chat","messages":[{"role":"user","content":"Hello"}]}'`}
            </pre>
            <p className="text-accent-yellow">
              提示：请求之间的 prompt diff 会自动在请求详情页显示。
              找出哪些 token 前缀变了导致缓存失效！
            </p>
          </div>
        </div>
      </div>
    </div>
  );
}

export default Settings;
