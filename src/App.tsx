import { useState, useEffect, useCallback, useRef } from "react";
import { invoke } from "@tauri-apps/api/core";
import type { RequestSummary } from "./types";
import RequestList from "./components/RequestList";
import RequestDetail from "./components/RequestDetail";
import Settings from "./components/Settings";

type Page = "requests" | "settings";

function App() {
  const [page, setPage] = useState<Page>("requests");
  const [requests, setRequests] = useState<RequestSummary[]>([]);
  const [selectedId, setSelectedId] = useState<string | null>(null);
  const [apiKeyFilter, setApiKeyFilter] = useState("");
  const [apiKeys, setApiKeys] = useState<string[]>([]);
  const intervalRef = useRef<ReturnType<typeof setInterval> | null>(null);

  const fetchRequests = useCallback(async () => {
    try {
      const filter = apiKeyFilter || undefined;
      const list = await invoke<RequestSummary[]>("get_requests", {
        apiKeyFilter: filter,
      });
      setRequests(list);
    } catch (e) {
      console.error("fetch requests failed", e);
    }
  }, [apiKeyFilter]);

  const fetchApiKeys = useCallback(async () => {
    try {
      const keys = await invoke<string[]>("get_api_keys");
      setApiKeys(keys);
    } catch (e) {
      console.error("fetch keys failed", e);
    }
  }, []);

  useEffect(() => {
    fetchRequests();
    fetchApiKeys();
    intervalRef.current = setInterval(() => {
      fetchRequests();
      fetchApiKeys();
    }, 1000);
    return () => {
      if (intervalRef.current) clearInterval(intervalRef.current);
    };
  }, [fetchRequests, fetchApiKeys]);

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
          <span className="flex items-center gap-1">
            <span className="w-2 h-2 rounded-full bg-accent-green" />
            代理运行中
          </span>
          <span>{requests.length} 条记录</span>
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
