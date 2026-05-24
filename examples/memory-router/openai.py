from meat_memory import MeatMemoryClient


def build_memory_context_prompt(query: str, scope_id: str, base_url: str = "http://127.0.0.1:8080") -> dict[str, str]:
    memory = MeatMemoryClient(base_url=base_url)
    context = memory.search(query=query, scope_id=scope_id, max_records=5)
    return {
        "role": "system",
        "content": f"Use the following Meat Memory context when relevant:\n{context}",
    }
