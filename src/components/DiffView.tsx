import { useMemo } from "react";
import type { DiffResult } from "../types";

interface Props {
  diff: DiffResult;
}

function DiffView({ diff }: Props) {
  const lines = useMemo(() => {
    return diff.diff_text.split("\n").filter((l) => l.length > 0);
  }, [diff]);

  return (
    <div className="font-mono text-xs leading-relaxed">
      <div className="mb-3 flex items-center gap-3 text-xs text-text-muted">
        <span className="flex items-center gap-1">
          <span className="w-3 h-2 rounded bg-accent-green" /> 新增
        </span>
        <span className="flex items-center gap-1">
          <span className="w-3 h-2 rounded bg-accent-red" /> 删除
        </span>
        <span className="text-text-muted">
          ← {diff.left_id.slice(0, 8)} → {diff.right_id.slice(0, 8)}
        </span>
      </div>
      <div className="border border-surface-0/30 rounded overflow-hidden">
        {lines.length === 0 && (
          <div className="p-4 text-center text-text-muted">无差异</div>
        )}
        {lines.map((line, i) => {
          const prefix = line[0];
          const content = line.slice(1);
          if (prefix === "+") {
            return (
              <div key={i} className="flex border-b border-surface-0/20 last:border-b-0">
                <span className="w-8 text-right text-text-muted select-none px-1 border-r border-surface-0/30 leading-5 shrink-0" />
                <span className="w-6 text-center text-accent-green select-none leading-5 shrink-0">+</span>
                <span className="px-2 leading-5 bg-accent-green/10 text-accent-green whitespace-pre-wrap break-all flex-1">{content}</span>
              </div>
            );
          }
          if (prefix === "-") {
            return (
              <div key={i} className="flex border-b border-surface-0/20 last:border-b-0">
                <span className="w-8 text-right text-text-muted select-none px-1 border-r border-surface-0/30 leading-5 shrink-0" />
                <span className="w-6 text-center text-accent-red select-none leading-5 shrink-0">-</span>
                <span className="px-2 leading-5 bg-accent-red/10 text-accent-red whitespace-pre-wrap break-all flex-1">{content}</span>
              </div>
            );
          }
          return (
            <div key={i} className="flex border-b border-surface-0/20 last:border-b-0">
              <span className="w-8 text-right text-text-muted select-none px-1 border-r border-surface-0/30 leading-5 shrink-0" />
              <span className="w-6 text-center text-text-muted select-none leading-5 shrink-0"> </span>
              <span className="px-2 leading-5 text-text-secondary whitespace-pre-wrap break-all flex-1">{content}</span>
            </div>
          );
        })}
      </div>
    </div>
  );
}

export default DiffView;
