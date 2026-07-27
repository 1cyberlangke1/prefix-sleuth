import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
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

function cacheTag(rate: number | null): [string, string] {
  if (rate === null) return ["text-text-muted", "—"];
  const pct = (rate * 100).toFixed(1);
  if (rate > 0.7) return ["text-accent-green", `${pct}%`];
  if (rate > 0.3) return ["text-accent-yellow", `${pct}%`];
  return ["text-accent-red", `${pct}%`];
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
      <div className="h-full flex items-center justify-center text-text-muted text-sm">
        加载中...
      </div>
    );
  }

  if (!record) {
    return (
      <div className="h-full flex items-center justify-center text-text-muted text-sm">
        请求不存在
      </div>
    );
  }

  const [cacheColor, cacheLabel] = cacheTag(record.cache_info?.cache_hit_rate ?? null);

  const tabs: { key: Tab; label: string }[] = [
    { key: "diff", label: diff ? "Prompt Diff" : "Diff (无上一请求)" },
    { key: "request", label: "请求体" },
    { key: "response", label: "响应体" },
  ];

  return (
    <div className="h-full flex flex-col">
      <div className="px-4 py-2 bg-surface-1 border-b border-surface-0/50 shrink-0">
        <div className="flex items-center justify-between">
          <div className="flex items-center gap-3 text-xs flex-wrap">
            <span className="font-mono text-accent-blue">{record.method}</span>
            <span className="text-text-muted truncate max-w-[240px]">{record.path}</span>
            {record.model && (
              <span className="px-1.5 py-0.5 rounded bg-accent-mauve/20 text-accent-mauve font-mono text-[11px]">
                {record.model}
              </span>
            )}
            <span className="text-text-muted">{record.api_key_label}</span>
            {record.response_status && (
              <span className={`font-bold ${record.response_status < 300 ? "text-accent-green" : "text-accent-red"}`}>
                {record.response_status}
              </span>
            )}
            {record.duration_ms > 0 && (
              <span className="text-text-muted">{formatDuration(record.duration_ms)}</span>
            )}
          </div>
          <span className="text-xs text-text-muted shrink-0">{record.timestamp}</span>
        </div>
        {record.cache_info && (record.cache_info.cache_hit_rate !== null) && (
          <div className="flex items-center gap-3 mt-1.5 text-[11px]">
            <span className={cacheColor}>缓存命中率：{cacheLabel}</span>
            <span className="text-text-muted">
              H:{record.cache_info.prompt_cache_hit_tokens ?? 0} / M:{record.cache_info.prompt_cache_miss_tokens ?? 0}
            </span>
            <div className="flex-1 max-w-[160px] h-1.5 rounded-full bg-surface-0 overflow-hidden">
              <div
                className={`h-full rounded-full transition-all ${
                  (record.cache_info.cache_hit_rate ?? 0) > 0.7
                    ? "bg-accent-green"
                    : (record.cache_info.cache_hit_rate ?? 0) > 0.3
                    ? "bg-accent-yellow"
                    : "bg-accent-red"
                }`}
                style={{ width: `${((record.cache_info.cache_hit_rate ?? 0) * 100).toFixed(0)}%` }}
              />
            </div>
          </div>
        )}
      </div>

      <div className="flex gap-0 px-4 pt-2 border-b border-surface-0/50 shrink-0">
        {tabs.map((t) => (
          <button
            key={t.key}
            className={`px-3 py-1.5 text-xs font-medium rounded-t transition-colors ${
              tab === t.key
                ? "bg-surface-0 text-accent-blue border border-surface-0/50 border-b-transparent"
                : "text-text-muted hover:text-text-primary"
            }`}
            onClick={() => setTab(t.key)}
          >
            {t.label}
          </button>
        ))}
      </div>

      <div className="flex-1 overflow-auto p-4">
        {tab === "diff" && (
          diff ? (
            <DiffView diff={diff} />
          ) : (
            <div className="h-full flex items-center justify-center text-text-muted text-sm">
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
            <div className="h-full flex items-center justify-center text-text-muted text-sm">
              等待响应中...
            </div>
          )
        )}
      </div>
    </div>
  );
}

export default RequestDetail;
