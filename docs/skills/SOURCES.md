# Vendored skills

The stage skills live in `.agents/skills/` (`.claude/skills` links there), so any agent working from a
clone of this repo can load them. They are copied verbatim from MIT-licensed upstreams and pinned:

| Skills | Upstream | Pin | License |
|---|---|---|---|
| grill-with-docs, grilling, domain-modeling, to-tickets, tdd, codebase-design, code-review, improve-codebase-architecture, retro, handoff, writing-for-agents | [mattpocock/skills](https://github.com/mattpocock/skills) | 1.3.1 | `.agents/skills/LICENSE-pocock` |
| make-pr-easy-to-review, deslop | pstack | commit 190fc9a | `.agents/skills/LICENSE-pstack` |

Skills call each other by name; the set is closed (every skill a vendored skill calls is vendored).

**Updating:** copy the new upstream version over the directory in one PR, record the new pin here, and
note any behaviour change in a `progress/` entry. Local edits go in our own docs, never in a vendored file, so
the next update stays a clean copy.
