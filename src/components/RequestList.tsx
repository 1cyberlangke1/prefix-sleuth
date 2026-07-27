import type { RequestSummary } from "../types";

interface Props {
  requests: RequestSummary[];
  selectedId: string | null;
  onSelect: (id: string) => void;
}

function statusColor(s: number | null): string {
  if (!s) return "text-text-muted";
  if (s < 300) return "text-accent-green";
  if (s < 400) return "text-accent-yellow";
  return "text-accent-red";
}

function cacheBar(rate: number | null): [string, string, string] {
  if (rate === null) return ["", "text-text-muted", "—"];
  const pct = (rate * 100).toFixed(0);
  const color = rate > 0.7 ? "bg-accent-green" : rate > 0.3 ? "bg-accent-yellow" : "bg-accent-red";
  return [color, "text-accent-green", `${pct}%`];
}

function RequestList({ requests, selectedId, onSelect }: Props) {
  return (
    <div className="divide-y divide-surface-0/30">
      {requests.length === 0 && (
        <div className="p-4 text-center text-text-muted text-xs">
          暂无请求记录
          <br />
          发送请求到代理后自动显示
        </div>
      )}
      {requests.map((r) => {
        const [barColor, , barLabel] = cacheBar(r.cache_hit_rate);
        return (
          <button
            key={r.id}
            className={`w-full text-left px-3 py-2 transition-colors hover:bg-surface-0/50 ${
              selectedId === r.id ? "bg-accent-blue/10 border-l-2 border-accent-blue" : ""
            }`}
            onClick={() => onSelect(r.id)}
          >
            <div className="flex items-center justify-between gap-2">
              <span className="text-xs font-mono text-text-muted shrink-0">{r.timestamp}</span>
              <div className="flex items-center gap-2">
                {barColor && (
                  <span className="flex items-center gap-1">
                    <span className={`w-2 h-2 rounded-full ${barColor}`} />
                    <span className="text-[11px] font-mono text-text-muted">{barLabel}</span>
                  </span>
                )}
                <span className={`text-xs font-bold shrink-0 ${statusColor(r.response_status)}`}>
                  {r.response_status ?? "—"}
                </span>
              </div>
            </div>
            <div className="flex items-center gap-2 mt-0.5">
              {r.model && (
                <span className="text-[11px] px-1 rounded bg-accent-mauve/20 text-accent-mauve font-mono truncate max-w-[140px]">
                  {r.model}
                </span>
              )}
              <span className="text-[11px] text-text-muted truncate">{r.api_key_label}</span>
            </div>
            <div className="mt-0.5 text-[11px] text-text-muted truncate leading-relaxed">
              {r.request_preview}
            </div>
          </button>
        );
      })}
    </div>
  );
}

export default RequestList;
