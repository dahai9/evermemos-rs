export interface EverMemOSConfig {
  baseUrl?: string;
  orgId?: string;
  userId?: string;
  groupId?: string;
  apiKey?: string;
  fetchImpl?: typeof fetch;
}

export interface ApiEnvelope<T> {
  status: string;
  message: string;
  result?: T;
}

export interface MemoryItem {
  id?: string;
  content?: string;
  score?: number;
  memory_type?: string;
  memoryType?: string;
  timestamp?: string;
  [key: string]: unknown;
}

export interface MemorizePayload {
  content: string;
  sender?: string;
  role?: string;
  senderName?: string;
  messageId?: string;
  createTime?: string;
  userId?: string;
  groupId?: string;
  history?: Array<Record<string, unknown>>;
}

export interface SearchOptions {
  retrieveMethod?: "KEYWORD" | "VECTOR" | "HYBRID" | "RRF" | "AGENTIC" | string;
  memoryTypes?: string[];
  topK?: number;
  radius?: number;
  userId?: string;
  groupId?: string;
}

export interface FetchOptions {
  memoryType?: string;
  limit?: number;
  offset?: number;
  userId?: string;
  groupId?: string;
}

export interface DeleteOptions {
  userId?: string;
  groupId?: string;
  memoryId?: string;
}

export interface OpenAIMessage {
  role: "system" | "user" | "assistant";
  content: string;
}

export interface RoleContentMessage {
  role: string;
  content: string;
}
