export interface ProxyConfig {
  upstream_base: string;
  local_port: number;
  allowed_keys: string[];
}

export interface RequestRecord {
  id: string;
  timestamp: string;
  api_key_label: string;
  method: string;
  path: string;
  model: string | null;
  request_body: string;
  response_body: string | null;
  response_status: number | null;
  duration_ms: number;
}

export interface RequestSummary {
  id: string;
  timestamp: string;
  api_key_label: string;
  method: string;
  path: string;
  model: string | null;
  response_status: number | null;
  request_preview: string;
}

export interface DiffResult {
  left_id: string;
  right_id: string;
  left_messages: string;
  right_messages: string;
  diff_text: string;
}

export type Tab = "diff" | "request" | "response";
