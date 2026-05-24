import { MeatMemoryClient } from "../../sdks/typescript/src/index.ts";

export async function buildMemoryContextPrompt(input: {
  query: string;
  scopeId: string;
  baseUrl?: string;
}) {
  const memory = new MeatMemoryClient({ baseUrl: input.baseUrl });
  const context = await memory.search({
    scope_id: input.scopeId,
    query: input.query,
    max_records: 5,
  });

  return {
    role: "system",
    content: `Use the following Meat Memory context when relevant:\n${JSON.stringify(context)}`,
  };
}
