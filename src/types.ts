export interface DownstreamKey {
  key: string;
  label: string;
}

export interface ProxyConfig {
  upstream_base: string;
  local_port: number;
  upstream_api_key: string;
  downstream_keys: DownstreamKey[];
}

export interface CacheInfo {
  prompt_cache_hit_tokens: number | null;
  prompt_cache_miss_tokens: number | null;
  cache_hit_rate: number | null;
}

export interface RequestRecord {
  id: string;
  timestamp: string;
  api_key_label: string;
  method: string;
  path: string;
  model: string | null;
  request_body: string;
  request_headers: Record<string, string>;
  response_body: string | null;
  response_status: number | null;
  duration_ms: number;
  cache_info: CacheInfo;
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
  duration_ms: number;
  cache_hit_rate: number | null;
}

export interface DiffResult {
  left_id: string;
  right_id: string;
  left_messages: string;
  right_messages: string;
  diff_text: string;
}

export type Tab = "diff" | "headers" | "request" | "response";
