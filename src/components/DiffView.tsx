import { useMemo } from "react";
import { diffLines, type Change } from "diff";
import type { DiffResult } from "../types";

interface Props {
  diff: DiffResult;
}

function DiffView({ diff }: Props) {
  const changes = useMemo(() => {
    return diffLines(diff.left_messages, diff.right_messages);
  }, [diff]);

  return (
    <div className="font-mono text-xs leading-relaxed">
      <div className="mb-3 flex items-center gap-3 text-xs text-text-muted">
        <span className="flex items-center gap-1">
          <span className="w-3 h-3 rounded bg-accent-green/30" /> 新增
        </span>
        <span className="flex items-center gap-1">
          <span className="w-3 h-3 rounded bg-accent-red/30" /> 删除
        </span>
        <span className="text-text-muted">
          ← {diff.left_id.slice(0, 8)} vs {diff.right_id.slice(0, 8)} →
        </span>
      </div>
      <div className="border border-surface-0/30 rounded overflow-hidden">
        <table className="w-full border-collapse">
          <tbody>
            {changes.map((change: Change, i: number) => {
              if (change.removed && changes[i + 1]?.added) {
                const next = changes[i + 1];
                return (
                  <tr key={i} className="diff-add">
                    <td className="w-8 text-right text-text-muted select-none px-1 border-r border-surface-0/30 align-top">
                      {/* line number placeholder */}
                    </td>
                    <td className="w-6 text-center text-accent-red select-none align-top">−</td>
                    <td className="w-6 text-center text-accent-green select-none align-top">+</td>
                    <td className="px-2 whitespace-pre bg-accent-red/10 text-accent-red">
                      {change.value.replace(/\n$/, "")}
                    </td>
                    <td className="px-2 whitespace-pre bg-accent-green/10 text-accent-green">
                      {next.value.replace(/\n$/, "")}
                    </td>
                  </tr>
                );
              }
              if (change.added) return null;
              if (change.removed) return null;
              return (
                <tr key={i}>
                  <td className="w-8 text-right text-text-muted select-none px-1 border-r border-surface-0/30 align-top">
                    {/* line number placeholder */}
                  </td>
                  <td className="w-6 text-center select-none align-top text-text-muted" colSpan={2}>
                    {' '}
                  </td>
                  <td className="px-2 whitespace-pre text-text-secondary" colSpan={2}>
                    {change.value.replace(/\n$/, "")}
                  </td>
                </tr>
              );
            })}
          </tbody>
        </table>
      </div>
    </div>
  );
}

export default DiffView;
