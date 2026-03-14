import { EverMemOSClient } from "./client.js";
import { MemoryItem, OpenAIMessage, RoleContentMessage } from "./types.js";

function renderMemories(memories: MemoryItem[]): string {
  if (!memories.length) {
    return "No relevant memory found.";
  }

  return memories
    .map((memory, index) => {
      const content = String(memory.content ?? "").trim();
      const score = typeof memory.score === "number" ? ` (score=${memory.score.toFixed(3)})` : "";
      const memoryType = String(memory.memory_type ?? memory.memoryType ?? "unknown");
      return `${index + 1}. [${memoryType}]${score} ${content}`;
    })
    .join("\n");
}

export class MemoryContextBuilder {
  constructor(
    private readonly client: EverMemOSClient,
    private readonly defaults: {
      retrieveMethod?: string;
      topK?: number;
      memoryTypes?: string[];
    } = {},
  ) {}

  async build(query: string): Promise<string> {
    const memories = await this.client.search(query, {
      retrieveMethod: this.defaults.retrieveMethod ?? "HYBRID",
      topK: this.defaults.topK ?? 5,
      memoryTypes: this.defaults.memoryTypes,
    });
    return renderMemories(memories);
  }
}

export function composeSystemPrompt(basePrompt: string, memoryContext: string): string {
  if (!memoryContext.trim()) {
    return basePrompt;
  }
  return `${basePrompt.trim()}\n\nLong-term memory context:\n${memoryContext.trim()}`;
}

export function buildOpenAIMessages(params: {
  userInput: string;
  memoryContext: string;
  baseSystemPrompt?: string;
}): OpenAIMessage[] {
  const systemPrompt = composeSystemPrompt(
    params.baseSystemPrompt ?? "You are a helpful assistant.",
    params.memoryContext,
  );

  return [
    { role: "system", content: systemPrompt },
    { role: "user", content: params.userInput },
  ];
}

export function buildLangChainMessages(params: {
  userInput: string;
  memoryContext: string;
  baseSystemPrompt?: string;
}): RoleContentMessage[] {
  const systemPrompt = composeSystemPrompt(
    params.baseSystemPrompt ?? "You are a helpful assistant.",
    params.memoryContext,
  );

  return [
    { role: "system", content: systemPrompt },
    { role: "human", content: params.userInput },
  ];
}

export function buildLlamaIndexChatHistory(params: {
  userInput: string;
  memoryContext: string;
  baseSystemPrompt?: string;
}): RoleContentMessage[] {
  const systemPrompt = composeSystemPrompt(
    params.baseSystemPrompt ?? "You are a helpful assistant.",
    params.memoryContext,
  );

  return [
    { role: "system", content: systemPrompt },
    { role: "user", content: params.userInput },
  ];
}
