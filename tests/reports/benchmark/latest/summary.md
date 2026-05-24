# Benchmark Summary

suite: meat-code-zh
version: 2.91.0
run_id: bmr_01KSCJ76X190WBWBSN6899W8QJ
status: passed
case_count: 4
recall@1: 1.000
recall@5: 1.000
p50_latency_ms: 0
p95_latency_ms: 0
leakage_count: 0
failure_count: 0

## Cases
- compat-legacy-entrypoints: recall@1=true recall@5=true latency_ms=0 leakage=false failure_reason=none
- review-risk-first: recall@1=true recall@5=true latency_ms=0 leakage=false failure_reason=none
- secret-guard-baseline: recall@1=true recall@5=true latency_ms=0 leakage=false failure_reason=none
- benchmark-report-contract: recall@1=true recall@5=true latency_ms=0 leakage=false failure_reason=none
