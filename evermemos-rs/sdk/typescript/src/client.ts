import {
  ApiEnvelope,
  DeleteOptions,
  EverMemOSConfig,
  FetchOptions,
  MemorizePayload,
  MemoryItem,
  SearchOptions,
} from "./types.js";

function defaultNowIso(): string {
  return new Date().toISOString();
}

function defaultMessageId(): string {
  if (typeof crypto !== "undefined" && typeof crypto.randomUUID === "function") {
    return crypto.randomUUID();
  }
  return `msg_${Date.now()}_${Math.random().toString(36).slice(2)}`;
}

export class EverMemOSError extends Error {
  constructor(message: string, public readonly responseBody?: unknown) {
    super(message);
    this.name = "EverMemOSError";
  }
}

function mergeHeaders(
  baseHeaders: Record<string, string>,
  overrideHeaders?: HeadersInit,
): Record<string, string> {
  const merged = new Headers();
  for (const [key, value] of Object.entries(baseHeaders)) {
    merged.set(key, value);
  }
  if (overrideHeaders) {
    const overrides = new Headers(overrideHeaders);
    overrides.forEach((value, key) => {
      merged.set(key, value);
    });
  }

  const result: Record<string, string> = {};
  merged.forEach((value, key) => {
    result[key] = value;
  });
  return result;
}

export class EverMemOSClient {
  private readonly baseUrl: string;
  private readonly orgId: string;
  private readonly userId?: string;
  private readonly groupId?: string;
  private readonly apiKey?: string;
  private readonly fetchImpl: typeof fetch;

  constructor(config: EverMemOSConfig = {}) {
    this.baseUrl = (config.baseUrl ?? "http://localhost:8080").replace(/\/$/, "");
    this.orgId = config.orgId ?? "default-org";
    this.userId = config.userId;
    this.groupId = config.groupId;
    this.apiKey = config.apiKey;
    this.fetchImpl = config.fetchImpl ?? fetch;
  }

  async health(): Promise<unknown> {
    return this.request("/health", { method: "GET" }, false);
  }

  async memorize(payload: MemorizePayload): Promise<Record<string, unknown>> {
    const body = {
      message_id: payload.messageId ?? defaultMessageId(),
      create_time: payload.createTime ?? defaultNowIso(),
      sender: payload.sender ?? "User",
      sender_name: payload.senderName ?? payload.sender ?? "User",
      content: payload.content,
      role: payload.role ?? "user",
      user_id: payload.userId ?? this.userId,
      group_id: payload.groupId ?? this.groupId,
      ...(payload.history ? { history: payload.history } : {}),
    };

    return this.request<Record<string, unknown>>("/api/v1/memories", {
      method: "POST",
      body: JSON.stringify(body),
    });
  }

  async addConversation(params: {
    userMessage: string;
    assistantMessage: string;
    userName?: string;
    assistantName?: string;
    userId?: string;
    groupId?: string;
  }): Promise<{ user: Record<string, unknown>; assistant: Record<string, unknown> }> {
    const user = await this.memorize({
      content: params.userMessage,
      sender: params.userName ?? "User",
      senderName: params.userName ?? "User",
      role: "user",
      userId: params.userId,
      groupId: params.groupId,
    });

    const assistant = await this.memorize({
      content: params.assistantMessage,
      sender: params.assistantName ?? "Assistant",
      senderName: params.assistantName ?? "Assistant",
      role: "assistant",
      userId: params.userId,
      groupId: params.groupId,
      history: [{ role: "user", content: params.userMessage }],
    });

    return { user, assistant };
  }

  async search(query: string, options: SearchOptions = {}): Promise<MemoryItem[]> {
    const params = new URLSearchParams();
    params.set("query", query);
    params.set("retrieve_method", options.retrieveMethod ?? "HYBRID");
    params.set("top_k", String(options.topK ?? 5));

    const userId = options.userId ?? this.userId;
    const groupId = options.groupId ?? this.groupId;
    if (userId) params.set("user_id", userId);
    if (groupId) params.set("group_id", groupId);
    if (options.memoryTypes?.length) params.set("memory_types", options.memoryTypes.join(","));
    if (typeof options.radius === "number") params.set("radius", String(options.radius));

    const result = await this.request<{ memories?: MemoryItem[] }>(`/api/v1/memories/search?${params.toString()}`, {
      method: "GET",
    });
    return result.memories ?? [];
  }

  async fetchMemories(options: FetchOptions = {}): Promise<Record<string, unknown>> {
    const params = new URLSearchParams();
    params.set("limit", String(options.limit ?? 20));
    params.set("offset", String(options.offset ?? 0));

    const userId = options.userId ?? this.userId;
    const groupId = options.groupId ?? this.groupId;
    if (userId) params.set("user_id", userId);
    if (groupId) params.set("group_id", groupId);
    if (options.memoryType) params.set("memory_type", options.memoryType);

    return this.request(`/api/v1/memories?${params.toString()}`, { method: "GET" });
  }

  async getProfile(memoryType = "profile", options: Omit<FetchOptions, "memoryType"> = {}): Promise<MemoryItem[]> {
    const result = await this.fetchMemories({ ...options, memoryType, limit: options.limit ?? 20 });
    const memories = result.memories;
    return Array.isArray(memories) ? (memories as MemoryItem[]) : [];
  }

  async deleteMemories(options: DeleteOptions = {}): Promise<Record<string, unknown>> {
    const body = {
      user_id: options.userId ?? this.userId,
      group_id: options.groupId ?? this.groupId,
      ...(options.memoryId ? { memory_id: options.memoryId } : {}),
    };

    return this.request("/api/v1/memories", {
      method: "DELETE",
      body: JSON.stringify(body),
    });
  }

  private async request<T = Record<string, unknown>>(
    path: string,
    init: RequestInit,
    unwrapEnvelope = true,
  ): Promise<T> {
    const baseHeaders: Record<string, string> = {
      "Content-Type": "application/json",
      "X-Organization-Id": this.orgId,
      ...(this.apiKey ? { Authorization: `Bearer ${this.apiKey}` } : {}),
    };

    const response = await this.fetchImpl(`${this.baseUrl}${path}`, {
      ...init,
      headers: mergeHeaders(baseHeaders, init.headers),
    });

    const text = await response.text();
    let data: ApiEnvelope<T> | T = {} as T;
    if (text) {
      try {
        data = JSON.parse(text) as ApiEnvelope<T> | T;
      } catch {
        throw new EverMemOSError("EverMemOS returned non-JSON response", text);
      }
    }

    if (!response.ok) {
      throw new EverMemOSError(`EverMemOS request failed with status ${response.status}`, data);
    }

    if (!unwrapEnvelope) {
      return data as T;
    }

    const envelope = data as ApiEnvelope<T>;
    if (envelope.status !== "success" && envelope.status !== "ok") {
      throw new EverMemOSError(envelope.message || "EverMemOS returned a non-success response", envelope);
    }

    return (envelope.result ?? ({} as T)) as T;
  }
}
