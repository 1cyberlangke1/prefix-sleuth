import { useState, useEffect, useMemo } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { Search, Settings as SettingsIcon, Play, Square, Activity, LayoutList, Sun, Moon } from "lucide-react";
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
  const [theme, setTheme] = useState<"light" | "dark">(() => {
    return window.matchMedia("(prefers-color-scheme: dark)").matches ? "dark" : "light";
  });

  useEffect(() => {
    const mediaQuery = window.matchMedia("(prefers-color-scheme: dark)");
    const handleChange = (e: MediaQueryListEvent) => {
      setTheme(e.matches ? "dark" : "light");
    };
    mediaQuery.addEventListener("change", handleChange);
    return () => mediaQuery.removeEventListener("change", handleChange);
  }, []);

  useEffect(() => {
    if (theme === "dark") {
      document.documentElement.classList.add("dark");
    } else {
      document.documentElement.classList.remove("dark");
    }
  }, [theme]);

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
    <div className="h-screen flex flex-col font-sans transition-colors duration-300">
      <header className="h-14 flex items-center justify-between px-4 sm:px-6 bg-white dark:bg-slate-900 border-b border-slate-200 dark:border-slate-800 shrink-0 z-10">
        <div className="flex items-center gap-6">
          <span className="font-bold text-slate-800 dark:text-slate-100 text-base tracking-wide flex items-center gap-2">
            <Search className="w-5 h-5 text-blue-600 dark:text-blue-400" /> 
            PrefixSleuth
          </span>
          <nav className="flex gap-2">
            <button
              className={`flex items-center gap-1.5 px-3 py-1.5 rounded-md text-sm font-medium transition-colors ${
                page === "requests"
                  ? "bg-slate-100 dark:bg-slate-800 text-slate-900 dark:text-slate-100"
                  : "text-slate-500 dark:text-slate-400 hover:text-slate-900 dark:hover:text-slate-100 hover:bg-slate-50 dark:hover:bg-slate-800/50"
              }`}
              onClick={() => setPage("requests")}
            >
              <LayoutList className="w-4 h-4" /> 请求
            </button>
            <button
              className={`flex items-center gap-1.5 px-3 py-1.5 rounded-md text-sm font-medium transition-colors ${
                page === "settings"
                  ? "bg-slate-100 dark:bg-slate-800 text-slate-900 dark:text-slate-100"
                  : "text-slate-500 dark:text-slate-400 hover:text-slate-900 dark:hover:text-slate-100 hover:bg-slate-50 dark:hover:bg-slate-800/50"
              }`}
              onClick={() => setPage("settings")}
            >
              <SettingsIcon className="w-4 h-4" /> 设置
            </button>
          </nav>
        </div>
        
        <div className="flex items-center gap-5 text-sm text-slate-600 dark:text-slate-400 hidden sm:flex">
          {avgRate !== null && (
            <span className="flex items-center gap-1.5 px-3 py-1 bg-slate-100 dark:bg-slate-800 rounded-full border border-slate-200 dark:border-slate-700">
              <Activity className={`w-3.5 h-3.5 ${avgRate > 0.7 ? "text-green-600 dark:text-green-400" : avgRate > 0.3 ? "text-yellow-600 dark:text-yellow-400" : "text-red-600 dark:text-red-400"}`} />
              缓存 {(avgRate * 100).toFixed(0)}%
            </span>
          )}
          
          <span className="font-medium">{requests.length} 条</span>
          
          <button
            className={`flex items-center gap-2 px-4 py-1.5 rounded-md text-sm font-medium transition-colors border ${
              proxyRunning
                ? "bg-red-50 dark:bg-red-900/20 text-red-700 dark:text-red-400 border-red-200 dark:border-red-800 hover:bg-red-100 dark:hover:bg-red-900/40"
                : "bg-blue-600 text-white border-blue-600 hover:bg-blue-700 shadow-sm"
            }`}
            onClick={toggleProxy}
          >
            {proxyRunning ? (
              <><Square className="w-4 h-4 fill-current" /> 停止代理</>
            ) : (
              <><Play className="w-4 h-4 fill-current" /> 启动代理</>
            )}
          </button>
          
          <button
            className="p-1.5 rounded-md text-slate-500 hover:text-slate-900 hover:bg-slate-100 dark:text-slate-400 dark:hover:text-slate-100 dark:hover:bg-slate-800 transition-colors"
            onClick={() => setTheme(theme === "light" ? "dark" : "light")}
            title="切换深浅色模式"
          >
            {theme === "light" ? <Moon className="w-5 h-5" /> : <Sun className="w-5 h-5" />}
          </button>
        </div>
      </header>

      <main className="flex-1 flex flex-col sm:flex-row overflow-hidden bg-slate-50 dark:bg-slate-950">
        {page === "requests" && (
          <>
            <aside className={`w-full sm:w-80 shrink-0 flex-col bg-white dark:bg-slate-900 border-r border-slate-200 dark:border-slate-800 ${selectedId ? 'hidden sm:flex' : 'flex'}`}>
              <div className="p-3 border-b border-slate-200 dark:border-slate-800">
                <select
                  className="w-full bg-slate-50 dark:bg-slate-800 text-slate-900 dark:text-slate-100 border border-slate-200 dark:border-slate-700 rounded-md px-3 py-2 text-sm outline-none focus:border-blue-500 focus:ring-1 focus:ring-blue-500 transition-colors"
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
            <section className={`flex-1 flex-col overflow-hidden bg-white dark:bg-slate-900 ${selectedId ? 'flex' : 'hidden sm:flex'}`}>
              {selected ? (
                <div className="flex flex-col h-full relative">
                  <div className="sm:hidden px-4 py-2 border-b border-slate-200 dark:border-slate-800 bg-slate-50 dark:bg-slate-800 flex items-center">
                    <button
                      className="text-sm text-blue-600 dark:text-blue-400 flex items-center gap-1 hover:underline"
                      onClick={() => setSelectedId(null)}
                    >
                      ← 返回列表
                    </button>
                  </div>
                  <RequestDetail requestId={selected.id} />
                </div>
              ) : (
                <div className="h-full flex flex-col items-center justify-center text-slate-500 dark:text-slate-400 space-y-4">
                  <Search className="w-12 h-12 opacity-20" />
                  <p>选择左侧请求查看详情</p>
                </div>
              )}
            </section>
          </>
        )}
        {page === "settings" && (
          <div className="flex-1 overflow-hidden bg-white dark:bg-slate-900 flex">
            <Settings />
          </div>
        )}
      </main>
    </div>
  );
}

export default App;
