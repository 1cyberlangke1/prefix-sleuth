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
      <div className="mb-3 flex items-center gap-4 text-xs text-slate-500 dark:text-slate-400">
        <span className="flex items-center gap-1.5">
          <span className="w-3 h-2 rounded bg-green-500" /> 新增
        </span>
        <span className="flex items-center gap-1.5">
          <span className="w-3 h-2 rounded bg-red-500" /> 删除
        </span>
        <span className="text-slate-400 dark:text-slate-500 bg-slate-100 dark:bg-slate-800 px-2 py-0.5 rounded-md">
          {diff.left_id.slice(0, 8)} ⋯ {diff.right_id.slice(0, 8)}
        </span>
      </div>
      <div className="border border-slate-200 dark:border-slate-800 rounded-lg overflow-hidden bg-white dark:bg-slate-900 shadow-sm">
        {lines.length === 0 && (
          <div className="p-8 text-center text-slate-500 dark:text-slate-400">无差异</div>
        )}
        {lines.map((line, i) => {
          const prefix = line[0];
          const content = line.slice(1);
          if (prefix === "+") {
            return (
              <div key={i} className="flex border-b border-slate-100 dark:border-slate-800/50 last:border-b-0">
                <span className="w-8 text-right text-slate-400 dark:text-slate-500 select-none px-1 border-r border-slate-100 dark:border-slate-800/50 leading-5 shrink-0" />
                <span className="w-6 text-center text-green-600 dark:text-green-500 select-none leading-5 shrink-0">+</span>
                <span className="px-2 leading-5 bg-green-50 dark:bg-green-900/20 text-green-700 dark:text-green-400 whitespace-pre-wrap break-all flex-1">{content}</span>
              </div>
            );
          }
          if (prefix === "-") {
            return (
              <div key={i} className="flex border-b border-slate-100 dark:border-slate-800/50 last:border-b-0">
                <span className="w-8 text-right text-slate-400 dark:text-slate-500 select-none px-1 border-r border-slate-100 dark:border-slate-800/50 leading-5 shrink-0" />
                <span className="w-6 text-center text-red-600 dark:text-red-500 select-none leading-5 shrink-0">-</span>
                <span className="px-2 leading-5 bg-red-50 dark:bg-red-900/20 text-red-700 dark:text-red-400 whitespace-pre-wrap break-all flex-1">{content}</span>
              </div>
            );
          }
          return (
            <div key={i} className="flex border-b border-slate-100 dark:border-slate-800/50 last:border-b-0 hover:bg-slate-50 dark:hover:bg-slate-800/30 transition-colors">
              <span className="w-8 text-right text-slate-400 dark:text-slate-500 select-none px-1 border-r border-slate-100 dark:border-slate-800/50 leading-5 shrink-0" />
              <span className="w-6 text-center text-slate-400 dark:text-slate-500 select-none leading-5 shrink-0"> </span>
              <span className="px-2 leading-5 text-slate-700 dark:text-slate-300 whitespace-pre-wrap break-all flex-1">{content}</span>
            </div>
          );
        })}
      </div>
    </div>
  );
}

export default DiffView;
