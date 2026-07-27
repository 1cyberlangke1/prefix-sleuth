import { useState, useEffect, useMemo } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import type { RequestSummary } from "./types";
import RequestList from "./components/RequestList";
import RequestDetail from "./components/RequestDetail";
import Settings from "./components/Settings";

type Page = "requests" | "settings";

function avgCacheRate(requests: RequestSummary[]): number | null {
  const rates = requests
    .map((r) => r.cache_hit_rate)
    .filter((r): r is number => r !== null);
  if (rates.length === 0) return null;
  return rates.reduce((a, b) => a + b, 0) / rates.length;
}

function App() {
  const [page, setPage] = useState<Page>("requests");
  const [requests, setRequests] = useState<RequestSummary[]>([]);
  const [selectedId, setSelectedId] = useState<string | null>(null);
  const [apiKeyFilter, setApiKeyFilter] = useState("");
  const [apiKeys, setApiKeys] = useState<string[]>([]);
  const [proxyRunning, setProxyRunning] = useState(false);

  const refreshRequests = async (filter?: string) => {
    try {
      const list = await invoke<RequestSummary[]>("get_requests", {
        apiKeyFilter: filter || undefined,
      });
      setRequests(list);
    } catch (e) {
      console.error("fetch requests failed", e);
    }
  };

  const refreshApiKeys = async () => {
    try {
      const keys = await invoke<string[]>("get_api_keys");
      setApiKeys(keys);
    } catch (e) {
      console.error("fetch keys failed", e);
    }
  };

  const refreshStatus = async () => {
    try {
      const running = await invoke<boolean>("proxy_status");
      setProxyRunning(running);
    } catch (e) {
      console.error("fetch status failed", e);
    }
  };

  useEffect(() => {
    refreshRequests(apiKeyFilter);
    refreshApiKeys();
    refreshStatus();

    const unlistenPromises: Promise<() => void>[] = [];

    unlistenPromises.push(
      listen<string>("new-record", () => {
        refreshRequests(apiKeyFilter);
        refreshApiKeys();
      })
    );

    unlistenPromises.push(
      listen<string>("record-updated", () => {
        refreshRequests(apiKeyFilter);
      })
    );

    unlistenPromises.push(
      listen("config-changed", () => {
        refreshRequests(apiKeyFilter);
        refreshApiKeys();
        refreshStatus();
      })
    );

    unlistenPromises.push(
      listen<boolean>("proxy-status-changed", (event) => {
        setProxyRunning(event.payload);
      })
    );

    return () => {
      Promise.all(unlistenPromises).then((fns) => fns.forEach((fn) => fn()));
    };
  }, [apiKeyFilter]);

  const avgRate = useMemo(() => avgCacheRate(requests), [requests]);

  const toggleProxy = async () => {
    try {
      if (proxyRunning) {
        await invoke("stop_proxy");
      } else {
        await invoke("start_proxy");
      }
      refreshStatus();
    } catch (e) {
      console.error("toggle proxy failed", e);
    }
  };

  const selected = requests.find((r) => r.id === selectedId) ?? null;

  return (
    <div className="h-screen flex flex-col bg-surface-0 text-text-primary">
      <header className="h-10 flex items-center justify-between px-4 bg-surface-1 border-b border-surface-0/50 shrink-0">
        <div className="flex items-center gap-4">
          <span className="font-bold text-accent-blue text-sm tracking-wide">
            🔍 PrefixSleuth
          </span>
          <nav className="flex gap-1">
            <button
              className={`px-3 py-1 rounded text-xs font-medium transition-colors ${
                page === "requests"
                  ? "bg-accent-blue/20 text-accent-blue"
                  : "text-text-muted hover:text-text-primary hover:bg-surface-0/50"
              }`}
              onClick={() => setPage("requests")}
            >
              请求
            </button>
            <button
              className={`px-3 py-1 rounded text-xs font-medium transition-colors ${
                page === "settings"
                  ? "bg-accent-blue/20 text-accent-blue"
                  : "text-text-muted hover:text-text-primary hover:bg-surface-0/50"
              }`}
              onClick={() => setPage("settings")}
            >
              设置
            </button>
          </nav>
        </div>
        <div className="flex items-center gap-3 text-xs text-text-muted">
          {avgRate !== null && (
            <span className="flex items-center gap-1">
              <span className={`w-2 h-2 rounded-full ${avgRate > 0.7 ? "bg-accent-green" : avgRate > 0.3 ? "bg-accent-yellow" : "bg-accent-red"}`} />
              缓存 {(avgRate * 100).toFixed(0)}%
            </span>
          )}
          <span className="flex items-center gap-0.5">
            {requests.slice(0, 20).reverse().map((r, i) => (
              <span
                key={r.id}
                className={`w-1 h-3 rounded-sm ${
                  r.cache_hit_rate === null
                    ? "bg-surface-0/30"
                    : r.cache_hit_rate > 0.7
                    ? "bg-accent-green"
                    : r.cache_hit_rate > 0.3
                    ? "bg-accent-yellow"
                    : "bg-accent-red"
                }`}
                title={`#${requests.length - i}: ${r.cache_hit_rate !== null ? (r.cache_hit_rate * 100).toFixed(0) + "%" : "—"}`}
              />
            ))}
          </span>
          <span>{requests.length} 条</span>
          <button
            className={`flex items-center gap-1 px-2 py-0.5 rounded text-xs font-medium transition-colors ${
              proxyRunning
                ? "bg-accent-green/20 text-accent-green hover:bg-accent-green/30"
                : "bg-accent-red/20 text-accent-red hover:bg-accent-red/30"
            }`}
            onClick={toggleProxy}
          >
            <span className={`w-1.5 h-1.5 rounded-full ${proxyRunning ? "bg-accent-green animate-pulse" : "bg-accent-red"}`} />
            {proxyRunning ? "运行中" : "已停止"}
          </button>
        </div>
      </header>

      <main className="flex-1 flex overflow-hidden">
        {page === "requests" && (
          <>
            <aside className="w-72 shrink-0 flex flex-col bg-surface-1 border-r border-surface-0/50">
              <div className="p-2 border-b border-surface-0/50">
                <select
                  className="w-full bg-surface-0 text-text-primary border border-surface-0/50 rounded px-2 py-1 text-xs outline-none focus:border-accent-blue"
                  value={apiKeyFilter}
                  onChange={(e) => setApiKeyFilter(e.target.value)}
                >
                  <option value="">全部 Key</option>
                  {apiKeys.map((k) => (
                    <option key={k} value={k}>{k}</option>
                  ))}
                </select>
              </div>
              <div className="flex-1 overflow-y-auto">
                <RequestList
                  requests={requests}
                  selectedId={selectedId}
                  onSelect={setSelectedId}
                />
              </div>
            </aside>
            <section className="flex-1 overflow-hidden">
              {selected ? (
                <RequestDetail requestId={selected.id} />
              ) : (
                <div className="h-full flex items-center justify-center text-text-muted text-sm">
                  选择左侧请求查看详情
                </div>
              )}
            </section>
          </>
        )}
        {page === "settings" && <Settings />}
      </main>
    </div>
  );
}

export default App;
