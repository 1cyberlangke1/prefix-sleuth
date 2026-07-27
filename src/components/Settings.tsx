import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { Server, Key, Trash2, Plus, Info, Save, Eraser } from "lucide-react";
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

  const removeDownstream = async (i: number) => {
    if (!config) return;
    const dk = config.downstream_keys[i];
    if (dk && dk.label) {
      try {
        await invoke("clear_requests_by_key", { label: dk.label });
      } catch (e) {
        console.error("Failed to clear records on key delete", e);
      }
    }
    setConfig({
      ...config,
      downstream_keys: config.downstream_keys.filter((_, idx) => idx !== i),
    });
  };

  const clearRecords = async (label: string) => {
    if (!label) return;
    try {
      await invoke("clear_requests_by_key", { label });
      setMessage(`已清空 ${label} 的所有请求记录`);
    } catch (e) {
      setMessage(`清空失败: ${e}`);
    }
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
      <div className="flex-1 flex items-center justify-center text-slate-500 dark:text-slate-400 text-sm">
        加载中...
      </div>
    );
  }

  return (
    <div className="flex-1 overflow-auto p-8 max-w-4xl mx-auto w-full">
      <div className="mb-8">
        <h2 className="text-xl font-bold text-slate-900 dark:text-slate-100 flex items-center gap-2">
          <Server className="w-6 h-6 text-blue-600 dark:text-blue-500" /> 代理设置
        </h2>
        <p className="text-sm text-slate-500 dark:text-slate-400 mt-2">
          配置 PrefixSleuth 代理的核心参数和多客户端 Key。
        </p>
      </div>

      <div className="space-y-6">
        <div className="bg-slate-50 dark:bg-slate-800/50 p-5 rounded-xl border border-slate-200 dark:border-slate-800">
          <label className="block text-sm font-semibold text-slate-800 dark:text-slate-200 mb-2">
            上游 API 地址
          </label>
          <input
            type="text"
            className="w-full bg-white dark:bg-slate-900 text-slate-900 dark:text-slate-100 border border-slate-300 dark:border-slate-700 rounded-lg px-4 py-2.5 text-sm outline-none focus:border-blue-500 focus:ring-1 focus:ring-blue-500 transition-shadow"
            value={config.upstream_base}
            onChange={(e) => setConfig({ ...config, upstream_base: e.target.value })}
            placeholder="https://api.deepseek.com"
          />
          <p className="text-xs text-slate-500 dark:text-slate-400 mt-2">转发目标的真实服务器基础 URL。</p>
        </div>

        <div className="bg-slate-50 dark:bg-slate-800/50 p-5 rounded-xl border border-slate-200 dark:border-slate-800">
          <label className="block text-sm font-semibold text-slate-800 dark:text-slate-200 mb-2">
            本地监听端口
          </label>
          <input
            type="number"
            className="w-40 bg-white dark:bg-slate-900 text-slate-900 dark:text-slate-100 border border-slate-300 dark:border-slate-700 rounded-lg px-4 py-2.5 text-sm outline-none focus:border-blue-500 focus:ring-1 focus:ring-blue-500 transition-shadow"
            value={config.local_port}
            onChange={(e) => setConfig({ ...config, local_port: parseInt(e.target.value) || 9527 })}
          />
          <p className="text-xs text-slate-500 dark:text-slate-400 mt-2">
            代理运行后，本地应用连接此端口。
          </p>
        </div>

        <div className="bg-slate-50 dark:bg-slate-800/50 p-5 rounded-xl border border-slate-200 dark:border-slate-800">
          <label className="block text-sm font-semibold text-slate-800 dark:text-slate-200 mb-2">
            上游 API Key (真正的服务商 Key)
          </label>
          <div className="relative">
            <Key className="absolute left-3 top-2.5 w-5 h-5 text-slate-400 dark:text-slate-500" />
            <input
              type="password"
              className="w-full bg-white dark:bg-slate-900 text-slate-900 dark:text-slate-100 border border-slate-300 dark:border-slate-700 rounded-lg pl-10 pr-4 py-2.5 text-sm outline-none focus:border-blue-500 focus:ring-1 focus:ring-blue-500 transition-shadow font-mono"
              value={config.upstream_api_key}
              onChange={(e) => setConfig({ ...config, upstream_api_key: e.target.value })}
              placeholder="sk-xxxxxxxxxxxxxxxx"
            />
          </div>
          <p className="text-xs text-slate-500 dark:text-slate-400 mt-2">
            所有从本地转发出去的请求都会自动替换为这个真实的 Key。
          </p>
        </div>

        <div className="bg-slate-50 dark:bg-slate-800/50 p-5 rounded-xl border border-slate-200 dark:border-slate-800">
          <div className="flex items-center justify-between mb-4">
            <div>
              <label className="block text-sm font-semibold text-slate-800 dark:text-slate-200">
                下游虚拟 Key 映射
              </label>
              <p className="text-xs text-slate-500 dark:text-slate-400 mt-1">
                分配给不同客户端的假 Key，用于分离追踪它们的缓存记录。
              </p>
            </div>
            <button
              className="flex items-center gap-1.5 px-3 py-1.5 text-sm font-medium text-blue-600 dark:text-blue-400 bg-blue-50 dark:bg-blue-900/20 rounded-md hover:bg-blue-100 dark:hover:bg-blue-900/40 transition-colors"
              onClick={addDownstream}
            >
              <Plus className="w-4 h-4" /> 添加
            </button>
          </div>

          <div className="space-y-3">
            {config.downstream_keys.map((dk, i) => (
              <div key={i} className="flex flex-col sm:flex-row sm:items-center gap-3 bg-white dark:bg-slate-900 p-3 rounded-lg border border-slate-200 dark:border-slate-700 shadow-sm">
                <input
                  type="text"
                  className="flex-1 bg-transparent text-slate-900 dark:text-slate-100 border border-slate-200 dark:border-slate-700 rounded-md px-3 py-2 text-sm outline-none focus:border-blue-500 focus:ring-1 focus:ring-blue-500 font-mono"
                  value={dk.key}
                  onChange={(e) => updateDownstream(i, "key", e.target.value)}
                  placeholder="sk-client-key (客户端填的假Key)"
                />
                <input
                  type="text"
                  className="w-full sm:w-48 bg-transparent text-slate-900 dark:text-slate-100 border border-slate-200 dark:border-slate-700 rounded-md px-3 py-2 text-sm outline-none focus:border-blue-500 focus:ring-1 focus:ring-blue-500"
                  value={dk.label}
                  onChange={(e) => updateDownstream(i, "label", e.target.value)}
                  placeholder="标签 (例如: ChatBox)"
                />
                <div className="flex items-center gap-1">
                  <button
                    className="p-2 text-slate-400 hover:text-orange-600 hover:bg-orange-50 dark:hover:bg-orange-900/20 rounded-md transition-colors"
                    onClick={() => clearRecords(dk.label)}
                    title="清空该客户端的历史记录"
                  >
                    <Eraser className="w-4 h-4" />
                  </button>
                  <button
                    className="p-2 text-slate-400 hover:text-red-600 hover:bg-red-50 dark:hover:bg-red-900/20 rounded-md transition-colors"
                    onClick={() => removeDownstream(i)}
                    title="删除"
                  >
                    <Trash2 className="w-4 h-4" />
                  </button>
                </div>
              </div>
            ))}
            {config.downstream_keys.length === 0 && (
              <div className="text-center py-4 text-sm text-slate-500 dark:text-slate-400 border border-dashed border-slate-300 dark:border-slate-700 rounded-lg">
                目前没有映射关系，客户端连接代理时将直接透传 Key。
              </div>
            )}
          </div>
        </div>

        <div className="flex items-center gap-4 pt-4 border-t border-slate-200 dark:border-slate-800">
          <button
            className="flex items-center gap-2 px-6 py-2.5 bg-blue-600 text-white text-sm font-semibold rounded-lg hover:bg-blue-700 shadow-sm transition-colors disabled:opacity-50"
            onClick={save}
            disabled={saving}
          >
            <Save className="w-4 h-4" />
            {saving ? "保存中..." : "保存设置"}
          </button>
          {message && (
            <span className={`text-sm font-medium ${message.includes("失败") ? "text-red-600 dark:text-red-400" : "text-green-600 dark:text-green-400"}`}>
              {message}
            </span>
          )}
        </div>

        <div className="mt-8 bg-blue-50 dark:bg-blue-900/10 p-5 rounded-xl border border-blue-100 dark:border-blue-900/30">
          <h3 className="text-sm font-bold text-blue-900 dark:text-blue-300 mb-3 flex items-center gap-2">
            <Info className="w-4 h-4" /> 使用指南
          </h3>
          <div className="text-sm text-blue-800 dark:text-blue-400/80 space-y-3">
            <p>
              1. 启动代理后，将本地应用的请求地址（Base URL）修改为：<code className="font-mono bg-white dark:bg-slate-900 px-1.5 py-0.5 rounded border border-blue-200 dark:border-blue-800">http://localhost:{config.local_port}</code>
            </p>
            <p>2. 在应用中，使用你分配好的下游虚拟 Key（如上配置）进行连接：</p>
            <pre className="bg-white dark:bg-slate-900 border border-blue-200 dark:border-blue-800 p-4 rounded-lg mt-2 text-xs leading-relaxed font-mono overflow-x-auto shadow-sm">
{`# 示例代码 (Python)
from openai import OpenAI

client = OpenAI(
    base_url="http://localhost:${config.local_port}",
    api_key="sk-client-key"  # 这里填你在上方添加的下游虚拟 Key
)`}
            </pre>
          </div>
        </div>
      </div>
    </div>
  );
}

export default Settings;
