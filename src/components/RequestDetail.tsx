import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { Database, FileJson, Clock } from "lucide-react";
import type { RequestRecord, DiffResult, Tab } from "../types";
import DiffView from "./DiffView";
import JsonViewer from "./JsonViewer";

interface Props {
  requestId: string;
}

function formatDuration(ms: number): string {
  if (ms < 1000) return `${ms}ms`;
  return `${(ms / 1000).toFixed(1)}s`;
}

function RequestDetail({ requestId }: Props) {
  const [record, setRecord] = useState<RequestRecord | null>(null);
  const [diff, setDiff] = useState<DiffResult | null>(null);
  const [tab, setTab] = useState<Tab>("diff");
  const [loading, setLoading] = useState(true);

  const load = () => {
    Promise.all([
      invoke<RequestRecord | null>("get_request_detail", { id: requestId }),
      invoke<DiffResult | null>("get_diff", { id: requestId }),
    ]).then(([rec, diffRes]) => {
      setRecord(rec);
      setDiff(diffRes);
      setLoading(false);
    });
  };

  useEffect(() => {
    setLoading(true);
    setTab("diff");
    setRecord(null);
    setDiff(null);
    load();

    const unlisten = listen<string>("record-updated", (event) => {
      if (event.payload === requestId) {
        load();
      }
    });

    return () => {
      unlisten.then((fn) => fn());
    };
  }, [requestId]);

  if (loading) {
    return (
      <div className="h-full flex items-center justify-center text-slate-500 dark:text-slate-400 text-sm">
        加载中...
      </div>
    );
  }

  if (!record) {
    return (
      <div className="h-full flex items-center justify-center text-slate-500 dark:text-slate-400 text-sm">
        请求不存在
      </div>
    );
  }

  const tabs: { key: Tab; label: string; icon: React.ReactNode }[] = [
    { key: "diff", label: diff ? "Prompt Diff" : "Diff (无上一请求)", icon: <Database className="w-3.5 h-3.5" /> },
    { key: "request", label: "请求体", icon: <FileJson className="w-3.5 h-3.5" /> },
    { key: "response", label: "响应体", icon: <FileJson className="w-3.5 h-3.5" /> },
  ];

  let cacheBadge = null;
  if (record.cache_info && record.cache_info.cache_hit_rate !== null) {
    const rate = record.cache_info.cache_hit_rate;
    const pct = (rate * 100).toFixed(1) + "%";
    const color = rate > 0.7 ? "text-green-700 bg-green-100 border-green-200 dark:text-green-400 dark:bg-green-900/30 dark:border-green-800" :
                  rate > 0.3 ? "text-yellow-700 bg-yellow-100 border-yellow-200 dark:text-yellow-400 dark:bg-yellow-900/30 dark:border-yellow-800" :
                               "text-red-700 bg-red-100 border-red-200 dark:text-red-400 dark:bg-red-900/30 dark:border-red-800";
    cacheBadge = (
      <span className={`px-2 py-0.5 rounded-md text-[11px] font-medium border flex items-center gap-1 ${color}`} title={`Hit: ${record.cache_info.prompt_cache_hit_tokens} / Miss: ${record.cache_info.prompt_cache_miss_tokens}`}>
        <Database className="w-3 h-3" /> {pct}
      </span>
    );
  }

  return (
    <div className="h-full flex flex-col">
      <div className="px-5 py-4 bg-white dark:bg-slate-900 border-b border-slate-200 dark:border-slate-800 shrink-0 shadow-sm">
        <div className="flex items-center justify-between">
          <div className="flex items-center gap-3 text-xs flex-wrap font-medium">
            <span className="font-mono text-blue-600 dark:text-blue-400 px-2 py-0.5 bg-blue-50 dark:bg-blue-900/20 rounded border border-blue-100 dark:border-blue-800/50">
              {record.method}
            </span>
            <span className="text-slate-800 dark:text-slate-200 truncate max-w-[240px]" title={record.path}>
              {record.path}
            </span>
            {record.model && (
              <span className="px-2 py-0.5 rounded text-[11px] font-mono text-purple-700 bg-purple-100 border border-purple-200 dark:text-purple-400 dark:bg-purple-900/20 dark:border-purple-800/50">
                {record.model}
              </span>
            )}
            <span className="px-2 py-0.5 rounded text-[11px] text-slate-600 bg-slate-100 border border-slate-200 dark:text-slate-400 dark:bg-slate-800 dark:border-slate-700">
              {record.api_key_label}
            </span>
            {record.response_status && (
              <span className={`flex items-center gap-1.5 ${record.response_status < 300 ? "text-green-600 dark:text-green-500" : "text-red-600 dark:text-red-500"}`}>
                <span className={`w-1.5 h-1.5 rounded-full ${record.response_status < 300 ? "bg-green-500" : "bg-red-500"}`} />
                {record.response_status}
              </span>
            )}
            {record.duration_ms > 0 && (
              <span className="text-slate-500 dark:text-slate-400 font-mono flex items-center gap-1">
                <Clock className="w-3.5 h-3.5" />
                {formatDuration(record.duration_ms)}
              </span>
            )}
            
            {/* The cache badge exactly on this line! */}
            {cacheBadge}
            
          </div>
          <span className="text-xs text-slate-400 dark:text-slate-500 font-mono shrink-0 px-2 py-1 bg-slate-50 dark:bg-slate-800 rounded-md">
            {record.timestamp}
          </span>
        </div>
      </div>

      <div className="flex gap-2 px-5 pt-3 bg-slate-50 dark:bg-slate-900/50 border-b border-slate-200 dark:border-slate-800 shrink-0">
        {tabs.map((t) => (
          <button
            key={t.key}
            className={`flex items-center gap-1.5 px-4 py-2 text-xs font-medium rounded-t-lg transition-colors border-b-2 ${
              tab === t.key
                ? "bg-white dark:bg-slate-800 text-blue-600 dark:text-blue-400 border-blue-600 dark:border-blue-500"
                : "text-slate-500 dark:text-slate-400 border-transparent hover:text-slate-900 dark:hover:text-slate-200 hover:bg-slate-100 dark:hover:bg-slate-800"
            }`}
            onClick={() => setTab(t.key)}
          >
            {t.icon}
            {t.label}
          </button>
        ))}
      </div>

      <div className="flex-1 overflow-auto p-4 bg-white dark:bg-slate-950">
        {tab === "diff" && (
          diff ? (
            <DiffView diff={diff} />
          ) : (
            <div className="h-full flex items-center justify-center text-slate-500 dark:text-slate-400 text-sm">
              需要至少两条请求才能计算 diff
            </div>
          )
        )}
        {tab === "request" && (
          <JsonViewer json={record.request_body} />
        )}
        {tab === "response" && (
          record.response_body ? (
            <JsonViewer json={record.response_body} />
          ) : (
            <div className="h-full flex items-center justify-center text-slate-500 dark:text-slate-400 text-sm">
              等待响应中...
            </div>
          )
        )}
      </div>
    </div>
  );
}

export default RequestDetail;
