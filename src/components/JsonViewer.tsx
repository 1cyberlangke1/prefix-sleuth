import { useMemo } from "react";

interface Props {
  json: string;
}

function formatJson(raw: string): string {
  try {
    return JSON.stringify(JSON.parse(raw), null, 2);
  } catch {
    return raw;
  }
}

function JsonViewer({ json }: Props) {
  const formatted = useMemo(() => formatJson(json), [json]);

  return (
    <pre className="text-xs leading-relaxed text-text-primary overflow-auto whitespace-pre-wrap break-all">
      {formatted}
    </pre>
  );
}

export default JsonViewer;
