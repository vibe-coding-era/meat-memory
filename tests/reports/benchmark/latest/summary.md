# Benchmark Summary

suite: meat-code-zh
version: 2.91.0
run_id: bmr_01KSCJCE2Y9QF4TS69YMA63TF1
status: passed
case_count: 4
recall@1: 1.000
recall@5: 1.000
p50_latency_ms: 0
p95_latency_ms: 0
leakage_count: 0
failure_count: 0
estimated_input_tokens: 38
estimated_output_tokens: 38
estimated_total_tokens: 76
estimated_cost_microusd: 76

## Cases
- compat-legacy-entrypoints: recall@1=true recall@5=true latency_ms=0 leakage=false failure_reason=none
- review-risk-first: recall@1=true recall@5=true latency_ms=0 leakage=false failure_reason=none
- secret-guard-baseline: recall@1=true recall@5=true latency_ms=0 leakage=false failure_reason=none
- benchmark-report-contract: recall@1=true recall@5=true latency_ms=0 leakage=false failure_reason=none
