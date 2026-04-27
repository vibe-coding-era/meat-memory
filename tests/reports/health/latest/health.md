# Memory Health Report

scope_id: scp_v293_acceptance
total: 2
active: 2
needs_review: 0
restricted: 1
stale: 0
low_confidence: 0
secret_findings: 2
high_risk_secret_findings: 1

## Suggested Actions

- redact
- review

## Risks

- kind=high_sensitivity severity=warning action=review memory=mem_01KQ79Q6P2WAX37SYJZPA504TX title=restricted health risk detail=restricted memory requires owner-scope recall access
- kind=secret_finding severity=critical action=redact memory=mem_01KQ79Q4X1NETYK3YKDF8CA66G title=guard redacts api key detail=api_key detected in memory body
