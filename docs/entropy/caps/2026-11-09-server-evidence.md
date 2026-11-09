# Cap note 2026-11-09: server-evidence

2026-11-09: server cap 4,170 -> 4,300 and e2e cap 8,520 -> 8,740; F21 makes the server serve run evidence (`server/src/evidence.ts`, the `tool_calls` table, the read route and the MCP hook), with a server unit test, and adds `TestF21` (the three-call read, the readers, the forged fields, malformed and prototype-key input, the 500 cap). The tree printed `LOC budget: server 4298 > 4170` and `LOC budget: e2e 8740 > 8520`; each cap is the count rounded up to ten. No other cap changes.
