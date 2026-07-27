import type { RequestSummary } from "../types";

interface Props {
  requests: RequestSummary[];
  selectedId: string | null;
  onSelect: (id: string) => void;
}

function statusColor(s: number | null): string {
  if (!s) return "text-slate-400 dark:text-slate-500";
  if (s < 300) return "text-green-600 dark:text-green-500";
  if (s < 400) return "text-yellow-600 dark:text-yellow-500";
  return "text-red-600 dark:text-red-500";
}

function cacheBar(rate: number | null): [string, string] {
  if (rate === null) return ["bg-slate-200 dark:bg-slate-700", "—"];
  const pct = (rate * 100).toFixed(0) + "%";
  if (rate > 0.7) return ["bg-green-500 dark:bg-green-500", pct];
  if (rate > 0.3) return ["bg-yellow-500 dark:bg-yellow-500", pct];
  return ["bg-red-500 dark:bg-red-500", pct];
}

function RequestList({ requests, selectedId, onSelect }: Props) {
  return (
    <div className="divide-y divide-slate-100 dark:divide-slate-800">
      {requests.length === 0 && (
        <div className="p-8 text-center text-slate-500 dark:text-slate-400 text-sm">
          暂无请求记录
          <br />
          发送请求到代理后自动显示
        </div>
      )}
      {requests.map((r) => {
        const [barColor, barLabel] = cacheBar(r.cache_hit_rate);
        const isSelected = selectedId === r.id;
        
        return (
          <button
            key={r.id}
            className={`group w-full text-left px-4 py-3 transition-colors ${
              isSelected 
                ? "bg-blue-50 dark:bg-slate-800/80 border-l-[3px] border-blue-600 dark:border-blue-500" 
                : "border-l-[3px] border-transparent hover:bg-slate-50 dark:hover:bg-slate-800/40"
            }`}
            onClick={() => onSelect(r.id)}
          >
            <div className="flex items-center justify-between gap-2 mb-1.5">
              <span className="text-xs font-mono text-slate-500 dark:text-slate-400">{r.timestamp}</span>
              <div className="flex items-center gap-2">
                <span className="flex items-center gap-1.5">
                  <span className={`w-2 h-2 rounded-full ${barColor}`} />
                  <span className="text-[11px] font-mono text-slate-500 dark:text-slate-400">{barLabel}</span>
                </span>
                <span className={`text-xs font-bold shrink-0 ${statusColor(r.response_status)}`}>
                  {r.response_status ?? "—"}
                </span>
              </div>
            </div>
            
            <div className="flex items-center gap-2 mb-1.5">
              {r.model && (
                <span className="text-[10px] px-1.5 py-0.5 rounded-sm bg-purple-100 text-purple-700 dark:bg-purple-900/30 dark:text-purple-300 font-mono truncate max-w-[140px] border border-purple-200 dark:border-purple-800/50">
                  {r.model}
                </span>
              )}
              <span className="text-[10px] px-1.5 py-0.5 rounded-sm bg-slate-100 dark:bg-slate-800 text-slate-600 dark:text-slate-300 border border-slate-200 dark:border-slate-700 truncate">
                {r.api_key_label}
              </span>
            </div>
            
            <div className="text-xs text-slate-600 dark:text-slate-400 truncate leading-relaxed">
              {r.request_preview}
            </div>
          </button>
        );
      })}
    </div>
  );
}

export default RequestList;
