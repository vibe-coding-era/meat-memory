from __future__ import annotations

import json
from dataclasses import dataclass
from typing import Any
from urllib.parse import urlencode
from urllib.request import Request, urlopen


JsonRecord = dict[str, Any]


@dataclass(slots=True)
class MeatMemoryClient:
    base_url: str = "http://127.0.0.1:8080"
    api_key: str | None = None

    def remember(
        self,
        *,
        scope_id: str,
        title: str,
        body: str,
        memory_kind: str = "fact",
    ) -> JsonRecord:
        return self._post(
            "/api/v1/memories",
            {
                "scope_id": scope_id,
                "title": title,
                "body": body,
                "memory_kind": memory_kind,
            },
        )

    def search(self, *, query: str, scope_id: str | None = None, max_records: int = 10) -> JsonRecord:
        payload: JsonRecord = {"query": query, "max_records": max_records}
        if scope_id:
            payload["scope_id"] = scope_id
        return self._post("/api/v1/context/search", payload)

    def profile(self, scope_id: str) -> JsonRecord:
        return self._get("/api/v1/health/report", {"scope_id": scope_id})

    def trace_latest(self, *, query: str, scope_id: str | None = None, max_records: int = 10) -> JsonRecord:
        payload: JsonRecord = {"query": query, "max_records": max_records}
        if scope_id:
            payload["scope_id"] = scope_id
        return self._post("/api/v1/recall/traces/latest", payload)

    def passport_export(self, *, scope_id: str, output_dir: str) -> JsonRecord:
        return self._post(
            "/api/v1/passports/export",
            {"scope_id": scope_id, "output_dir": output_dir},
        )

    def connector_dry_run(
        self,
        *,
        connector: str,
        root_path: str,
        max_items: int = 50,
    ) -> JsonRecord:
        return self._get(
            "/api/v1/compat/connectors/dry-run",
            {"connector": connector, "root_path": root_path, "max_items": max_items},
        )

    def _get(self, path: str, query: JsonRecord | None = None) -> JsonRecord:
        query_string = f"?{urlencode(query or {})}" if query else ""
        return self._request("GET", f"{path}{query_string}")

    def _post(self, path: str, payload: JsonRecord) -> JsonRecord:
        return self._request("POST", path, payload)

    def _request(self, method: str, path: str, payload: JsonRecord | None = None) -> JsonRecord:
        body = None if payload is None else json.dumps(payload).encode("utf-8")
        headers = {"accept": "application/json"}
        if payload is not None:
            headers["content-type"] = "application/json"
        if self.api_key:
            headers["authorization"] = f"Bearer {self.api_key}"
        request = Request(
            f"{self.base_url.rstrip('/')}{path}",
            data=body,
            headers=headers,
            method=method,
        )
        with urlopen(request, timeout=30) as response:
            return json.loads(response.read().decode("utf-8"))
