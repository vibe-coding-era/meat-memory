export type JsonRecord = Record<string, unknown>;

export interface MeatMemoryClientOptions {
  baseUrl?: string;
  apiKey?: string;
  fetchImpl?: typeof fetch;
}

export interface RememberRequest {
  scope_id: string;
  title: string;
  body: string;
  memory_kind?: string;
}

export interface SearchRequest {
  scope_id?: string;
  query: string;
  max_records?: number;
}

export interface ConnectorDryRunRequest {
  connector: string;
  root_path: string;
  max_items?: number;
}

export class MeatMemoryClient {
  private readonly baseUrl: string;
  private readonly apiKey?: string;
  private readonly fetchImpl: typeof fetch;

  constructor(options: MeatMemoryClientOptions = {}) {
    this.baseUrl = (options.baseUrl ?? "http://127.0.0.1:8080").replace(/\/$/, "");
    this.apiKey = options.apiKey;
    this.fetchImpl = options.fetchImpl ?? fetch;
  }

  remember(request: RememberRequest): Promise<JsonRecord> {
    return this.post("/api/v1/memories", request);
  }

  search(request: SearchRequest): Promise<JsonRecord> {
    return this.post("/api/v1/context/search", request);
  }

  profile(scopeId: string): Promise<JsonRecord> {
    return this.get(`/api/v1/health/report?scope_id=${encodeURIComponent(scopeId)}`);
  }

  traceLatest(request: SearchRequest): Promise<JsonRecord> {
    return this.post("/api/v1/recall/traces/latest", request);
  }

  passportExport(scopeId: string, outputDir: string): Promise<JsonRecord> {
    return this.post("/api/v1/passports/export", {
      scope_id: scopeId,
      output_dir: outputDir,
    });
  }

  connectorDryRun(request: ConnectorDryRunRequest): Promise<JsonRecord> {
    const params = new URLSearchParams({
      connector: request.connector,
      root_path: request.root_path,
      max_items: String(request.max_items ?? 50),
    });
    return this.get(`/api/v1/compat/connectors/dry-run?${params.toString()}`);
  }

  private async get(path: string): Promise<JsonRecord> {
    return this.request("GET", path);
  }

  private async post(path: string, body: unknown): Promise<JsonRecord> {
    return this.request("POST", path, body);
  }

  private async request(method: string, path: string, body?: unknown): Promise<JsonRecord> {
    const headers: Record<string, string> = { accept: "application/json" };
    if (body !== undefined) headers["content-type"] = "application/json";
    if (this.apiKey) headers.authorization = `Bearer ${this.apiKey}`;

    const response = await this.fetchImpl(`${this.baseUrl}${path}`, {
      method,
      headers,
      body: body === undefined ? undefined : JSON.stringify(body),
    });
    const payload = await response.json();
    if (!response.ok) {
      throw new Error(`Meat Memory API ${response.status}: ${JSON.stringify(payload)}`);
    }
    return payload as JsonRecord;
  }
}
