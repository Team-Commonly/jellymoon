# Generative UI for ask cards: options paper

Research only, read 2026-10-29. Licences were read from each repo's LICENSE file through the GitHub API, including nested LICENSE files where a repo has them. Stars and last-push dates come from the same pass, and package versions from `npm view`. "Unverified" means I could not confirm it from a primary source.

**The question.** When an agent posts an `ask`, the person ruling on it often was not there when the work happened. The card should show the evidence in the shape the decision needs: a diff, a comparison, a chart, a screenshot, a timeline. Two constraints are fixed. (1) Cards render from a fixed catalogue declared in our JSON Schema, so an agent sends typed data for a named component and never markup or code. (2) Every number in a card references a server-stamped receipt or tool-call id.

## 1. Recommendation

**Build the catalogue ourselves as an `x-variants` map in our schema, and render it with seven plain React components. Adopt no generative-UI runtime in v1.** No candidate gives us the two constraints together:

- The catalogue-first projects (A2UI, json-render, OpenUI) all let the model **compose** a layout from the catalogue, and they carry an expression or data-binding layer we would have to switch off. Our cards need a fixed layout per decision type, filled with data. Composition is where numbers without a source get in.
- The tool-rendering projects (Vercel AI SDK UI, CopilotKit "controlled") have the right pattern: a tool name maps to a component the developer wrote. But they assume the model loop runs inside their runtime. Ours runs in Claude Code or Codex on someone's computer and reaches the pod over MCP. The pattern is free to copy; the runtime adds nothing.
- None of them has a provenance field, so the receipt rule is ours to build in any case.

The core is small: one schema file, a validator for the `x-rules`, seven components, and a ref check in the pod's Durable Object. Two follow-ups keep the build reversible:

- **Shape each component so it maps one-to-one onto an A2UI custom catalogue entry.** If interop demand appears (an A2UI host, or CopilotKit's declarative mode), an export adapter in one file can serve it. A2UI's own schema uses `oneOf` ([server_to_client.json](https://github.com/a2ui-project/a2ui/blob/main/specification/v0_9/json/server_to_client.json)), so we translate to it rather than import it.
- **Defer MCP Apps.** It solves the reverse direction (a server shipping UI into a host such as Claude). It becomes useful later, if we want Jellymoon cards to render inside Claude or ChatGPT.

**On OpenUI.** Commonly v3 shipped OpenUI cards in production, and they worked. Two things weigh against making OpenUI the core of v4. In v3 the model wrote OpenUI Lang source, and making that safe took a few hundred lines of server validation: refuse `Query`, `Mutation` and `Run`, cap AST expansion, pin a catalogue hash. And OpenUI Lang is a text language, so `make gen` and our JSON Schema cannot check it. The effort is better spent on the upstream asks in section 5.

## 2. What "OpenUI" is: three projects use the name

Commonly v3's history shows cards built on `@openuidev/lang-core` and `@openuidev/react-lang` (both pinned at 1.0.1). Those packages come from **thesysdev/openui** (Thesys). The other two projects are unrelated:

- **thesysdev/openui:** MIT, "The Open Standard for Generative UI" ([repo](https://github.com/thesysdev/openui)). Its centre is OpenUI Lang, "a compact, streaming-first language for model-generated UI". Components are declared with `defineComponent({ name, description, props: z.object(...) })` and grouped by `createLibrary`. A system prompt is generated from the library, and `<Renderer>` parses the model's output as it streams ([react-lang README](https://github.com/thesysdev/openui/tree/main/packages/react-lang)). The CLI can also emit JSON Schema from a library. It also ships `@openuidev/a2ui`, an A2UI v1.0 client. `Query`/`Mutation` bindings call a `toolProvider` that "run[s] in the browser" ([queries-mutations doc](https://github.com/thesysdev/openui/blob/main/docs/content/docs/openui-lang/queries-mutations.mdx)). `react-lang` declares `react >=19` as a peer dependency.
- **wandb/openui:** Apache-2.0. It turns a prompt into a live HTML page ([repo](https://github.com/wandb/openui)), so it is free-form markup, which our constraint rules out.
- **openui/open-ui:** the W3C community group for native browser controls ([repo](https://github.com/openui/open-ui)). Not a generative UI project; its licence was not read.

## 3. Candidates

| Candidate | Licence (from file) | Stars / last push | Fixed declared catalogue? | Runs on our stack? | Lock-in |
|---|---|---|---|---|---|
| OpenUI (Thesys) [1] | MIT | 14.4k / 2026-10-29; react-lang 1.0.2 | Partly: catalogue-bound, but the model writes a program (layout plus expressions) | `lang-core` is framework-agnostic and documented for Edge; React renderer | Medium: a stored text language, not JSON |
| Vercel AI SDK, RSC `streamUI` [2] | Apache-2.0 | 27.6k / 2026-10-29; `ai` 7.0.156 | n/a: "AI SDK RSC is currently experimental"; the docs recommend AI SDK UI for production | Needs React Server Components | High |
| Vercel AI SDK UI, tool parts [2] | Apache-2.0 | same | Yes, by pattern: `tool-${toolName}` parts map to your component | Runs on Workers; but `useChat` assumes we run the model | Low (a pattern) |
| MCP Apps (ext-apps) [3] | MIT→Apache-2.0 transition (file says so) | 3.0k / 2026-10-27; spec "Stable (2026-01-26)" | No: `ui://` resources are HTML in a sandboxed iframe | Wrong direction for us (section 4) | Low |
| MCP-UI [4] | Apache-2.0 | 5.2k / 2026-09-16 | No: `rawHtml` or `externalUrl`; now an SDK for MCP Apps | Same as MCP Apps | Low |
| CopilotKit + AG-UI [5] | MIT (root and every package LICENSE) | 38.3k / 2026-10-29; AG-UI 16.9k | Controlled mode yes (`useComponent`/`useFrontendTool`); declarative mode is A2UI | A Workers runtime example exists; the agent must speak AG-UI | High: runtime plus licence-key features |
| A2UI [6] | Apache-2.0 | 16.9k / 2026-10-29; v0.9.1 stable, v1.0 RC | Yes: the client owns the catalogue and the agent may only name its components | React renderer `@a2ui/react` 0.14.0; documented over MCP | Low to medium |
| json-render (Vercel Labs) [7] | Apache-2.0 | 18.8k / 2026-10-29 (created 2026-01-14); core 0.21.3 | Yes: `defineCatalog` with Zod props | React; an MCP Apps package exists | Medium: 0.x, `$state`/`$cond` expressions |
| Tambo [8] | Mixed: root and `react-sdk` MIT; `apps/api`, `packages/backend`, `packages/core` Apache-2.0 | 11.2k / 2026-10-28 | Registered React components | Has its own backend; not evaluated further | Medium |

**Licence notes.** No candidate is AGPL, ELv2, FSL, BSL or source-available. Three things need care:

- **CopilotKit is MIT but has a commercial licence layer in the code.** The React provider shows "Powered by CopilotKit" and "The … feature requires a CopilotKit license" banners when its server reports `none`, `expired` or `invalid`. Some features turn off on `expired` or `invalid` ([banner source](https://github.com/CopilotKit/CopilotKit/blob/main/packages/react-core/src/v2/components/license-warning-banner.tsx), [provider tests](https://github.com/CopilotKit/CopilotKit/blob/main/packages/react-core/src/v2/providers/__tests__/CopilotKitProvider.license.test.tsx)). The tests show no banner when the status is absent ("non-intelligence mode"). Which features are gated is unverified.
- **The MCP ext-apps LICENSE is a transition notice:** new contributions are Apache-2.0, and older ones stay MIT until their authors consent.
- **Tambo mixes MIT and Apache-2.0 by directory.**

## 4. Per-candidate verdicts

- **OpenUI.** *How an agent would emit it over MCP:* our `post` tool takes `{lang:"openui@1", source}`, as Commonly v3 did. Thesys documents no MCP server or agent-facing tool interface, and the only tool model it documents runs in the browser. Maturity is good: 1.x packages and daily commits. The catalogue is real, but the model still authors a program, and expression features keep arriving upstream, so our validator has to track every release. Fit: medium. Keep it for the later case where an agent needs to compose a free layout, not for ask cards.
- **Vercel AI SDK.** RSC `streamUI` is out: it is experimental and server-component bound ([RSC overview](https://ai-sdk.dev/docs/ai-sdk-rsc/overview)). AI SDK UI's rule is the one we want: "connecting the results of a tool call to a React component", where you branch on `part.type === 'tool-displayWeather'` ([generative UI docs](https://ai-sdk.dev/docs/ai-sdk-ui/generative-user-interfaces)). We copy the rule and skip the library, because our pod never runs the model.
- **MCP Apps / MCP-UI.** A tool declares a `ui://` HTML resource, and the host renders it "in a sandboxed iframe" ([ext-apps README](https://github.com/modelcontextprotocol/ext-apps)). That HTML is written by a server developer, not the model, but it is still arbitrary HTML and JS, and a Claude Code or Codex agent is an MCP *client*, not a UI server. MCP-UI now describes itself as an SDK for the MCP Apps standard ([README](https://github.com/MCP-UI-Org/mcp-ui)). Later use: our pod's MCP server could publish `ui://` views of ask cards so they render inside Claude. Unverified: whether Claude Code renders MCP Apps at all.
- **CopilotKit / AG-UI.** It names three kinds of generative UI. *Controlled*: "you wrote the component; the agent only picks which one to render and what data to pass." *Declarative* is A2UI, and *open-ended* is MCP Apps ([docs](https://docs.copilotkit.ai/learn/generative-ui)). Controlled mode matches our model, but it routes the agent through CopilotRuntime and AG-UI. A Cloudflare Workers runtime example exists ([examples/v2/runtime/cf-workers](https://github.com/CopilotKit/CopilotKit/tree/main/examples/v2/runtime/cf-workers)). Our agents already have MCP, so this would add a second agent protocol for something a `switch` statement does.
- **A2UI.** The closest in philosophy: "a declarative data format, not executable code", with a client-owned catalogue ([README](https://github.com/a2ui-project/a2ui); `github.com/google/A2UI` now redirects there). It documents A2UI over MCP: tools return `updateDataModel` messages bound to `a2ui://` template resources ([guide](https://github.com/a2ui-project/a2ui/blob/main/docs/public/guides/a2ui_over_mcp.md)). Three cautions. The README says "Early stage public preview… Expect changes". Its MCP catalogue adds `callMcpTool` plus `jmespath` and regex data functions that run in the client ([catalogs/mcp](https://github.com/a2ui-project/a2ui/blob/main/catalogs/mcp/README.md)); we would refuse that catalogue. And the layout is a flat component list the agent composes.
- **json-render.** "AI can only use components in your catalog", declared with `defineCatalog` and Zod; `@json-render/mcp` serves the result as an MCP App ([README](https://github.com/vercel-labs/json-render)). It has `visible` conditions and `$state`/`$cond` prop expressions that we would disable. It is under a year old and still 0.x.

## 5. Upstream first

**Send upstream:**
- **OpenUI:** a strict data-only parse mode in `lang-core` that refuses `Query`, `Mutation`, `Run`, custom functions and reactive expressions, and enforces an AST expansion budget. Commonly v3 built exactly this locally; upstreaming it lets OpenUI serve trust-bounded hosts like us. Also ask Thesys whether a JSON input mode, keyed by component name, is on their roadmap.
- **A2UI:** a provenance convention, an optional `ref` on bound values that names the tool call that produced them, and a "no functions" catalogue profile. Both would help any host that shows agent evidence to a human.

**Build ourselves:**
- The catalogue schema
- The `x-rules` validator
- Seven components
- Ref resolution in the PodDO
- The daemon change below

This is a new module, so it gets a `docs/adr/` note.

**A gap the receipt rule exposes.** Today `Receipt.toolCalls` is only a count of "MCP calls the pod saw on this run's token". Most numbers worth showing come from commands the agent ran locally (tests, benchmarks, builds), and the pod never sees those. The daemon does see them: ACP `tool_call` updates carry a required `toolCallId`, along with `status`, `rawInput` and `rawOutput` ([ACP tool calls](https://agentclientprotocol.com/protocol/tool-calls)).

**Now in the schema** (added with the grounded ask context):
1. The daemon reports each harness tool call it sees over the `run.toolcall` event: name, status, times and a digest of up to 600 characters.
2. The pod stamps it as a `ToolCallRecord` with a `tc_` id and `source: daemon`, next to the MCP calls it records itself (`source: pod`).
3. The agent reads its run's records through the `evidence` operation and cites their ids. Whether a harness exposes ACP `toolCallId`s to the model no longer matters, because the pod issues the ids and the agent reads them back.
4. A ref must resolve to a record in the pod, and every number must appear in a cited record's digest, or the post is refused with `ungrounded`.

Say this plainly on the card: a ref proves a call was recorded and lets the person open its output. It does not prove the agent copied the number correctly. Records reported by the daemon are only as trustworthy as the person's own computer.

## 6. Proposed v1 catalogue

`Num` is `{ value: number, unit?: enum, ref: ToolCallId }`. A series or table may carry one `ref` for all of its numbers when they came from one call.

| type | For deciding | data fields |
|---|---|---|
| `diff` | approve a change | `files[≤20]`: `path`, `change` (added, modified, deleted, renamed), `from?`, `additions: Num`, `deletions: Num`, `hunks?[≤5]`: `header`, `lines[≤40]`: `op` (add, del, ctx), `text ≤200`; `ref` |
| `compare` | pick an option | `columns[2–5]`: OptionId (must equal the ask's option ids); `rows[≤12]`: `label`, `cells[]`: `text? ≤60` or `num?: Num`, `best?: bool` |
| `metric` | did it regress? | `chart` (line, bar); `unit`; `series[≤4]`: `label`, `ref`, `points[≤200]`: `x` (Timestamp or label), `y: number`; `threshold?: Num`; `baseline?: Num` |
| `screenshot` | does it look right? | `shots[1–4]`: `fileId` (pod-stored, never a URL), `caption ≤80`, `viewport` (mobile, desktop), `role` (before, after, only); `ref` (the capture call) |
| `timeline` | what happened overnight | `events[≤30]`: `at: Timestamp`, `label ≤80`, `actor?: ActorId`, `status?: MessageStatus`, `ref?`. The pod can also compose this card itself from run records |
| `checks` | is it safe to ship? | `suite ≤40`; `items[≤50]`: `name`, `result` (pass, fail, skip, flaky), `durationMs?: Num`, `ref`. Totals are computed by the client, never sent |
| `excerpt` | read the actual error | `lang` (text, log, json, diff); `lines[≤40]` of text `≤200`; `ref`; `startLine?` |

How it sits in our schema (a new `schema/card.json`, referenced from `PostInput.card` and `Message.card`):

```json
"x-variants": {
  "diff": { "data": "DiffCard" },        "compare": { "data": "CompareCard" },
  "metric": { "data": "MetricCard" },    "screenshot": { "data": "ScreenshotCard" },
  "timeline": { "data": "TimelineCard" }, "checks": { "data": "ChecksCard" },
  "excerpt": { "data": "ExcerptCard" }
},
"$defs": {
  "CardType": { "type": "string", "enum": ["diff","compare","metric","screenshot","timeline","checks","excerpt"] },
  "Card": { "type": "object", "additionalProperties": false, "required": ["type", "data"],
    "properties": { "type": { "$ref": "#/$defs/CardType" },
                    "data": { "description": "Matches the definition x-variants names for type.", "type": "object" } } },
  "Num": { "type": "object", "additionalProperties": false, "required": ["value", "ref"],
    "properties": { "value": { "type": "number" }, "unit": { "$ref": "#/$defs/Unit" },
                    "ref": { "$ref": "common.json#/$defs/ToolCallId" } } }
}
```

New `x-rules`:
- `card` is allowed on `ask` and `result`, and `compare` only on `ask`.
- Every `ref` resolves to a tool call on this post's run, or the post is refused with `ungrounded`.
- A `compare` cell carries exactly one of `text` or `num`, and every row has one cell per column.
- An unknown `type` renders `summary` and `body`, never a guess.
- `body` stays the text fallback.

New common id: `FileId` (`fil_`); `ToolCallId` (`tc_`) now exists. Agents emit a card through the existing `post` tool by adding `card: { type, data }`. The tool description lists the seven types, the way AI SDK and CopilotKit describe their tools.

## 7. Risks and open questions

1. **Catalogue creep.** Every new type is a schema change, a protected path and a review gate. That is the point, but expect pressure for a generic `table`. Hold the line until three asks need it.
2. **Ref load.** Agents must quote `tc_` ids, which they get from the `evidence` operation. Measure in the slice how often an agent's first post is refused with `ungrounded`; if it is often, return the run's recent ids with every `post` refusal.
3. **Screenshots** need a file store (R2) and a capture path. Neither exists in v4 yet.
4. **Unverified:** CopilotKit's licence-gated feature list; whether Claude Code renders MCP Apps; OpenUI's plans for a JSON input mode; Tambo's self-host story.

## Sources

[1] https://github.com/thesysdev/openui/blob/main/LICENSE · https://github.com/thesysdev/openui · https://github.com/thesysdev/openui/tree/main/packages/react-lang · https://www.npmjs.com/package/@openuidev/react-lang · Commonly v3 git history (Team-Commonly/commonly) · https://github.com/wandb/openui/blob/main/LICENSE · https://github.com/openui/open-ui
[2] https://github.com/vercel/ai/blob/main/LICENSE · https://ai-sdk.dev/docs/ai-sdk-rsc/overview · https://ai-sdk.dev/docs/ai-sdk-ui/generative-user-interfaces
[3] https://github.com/modelcontextprotocol/ext-apps/blob/main/LICENSE · https://github.com/modelcontextprotocol/ext-apps/blob/main/specification/2026-01-26/apps.mdx
[4] https://github.com/MCP-UI-Org/mcp-ui/blob/main/LICENSE · https://github.com/MCP-UI-Org/mcp-ui
[5] https://github.com/CopilotKit/CopilotKit/blob/main/LICENSE · https://github.com/ag-ui-protocol/ag-ui/blob/main/LICENSE · https://docs.copilotkit.ai/learn/generative-ui · https://docs.copilotkit.ai/generative-ui
[6] https://github.com/a2ui-project/a2ui/blob/main/LICENSE · https://github.com/a2ui-project/a2ui · https://github.com/a2ui-project/a2ui/tree/main/typescript/renderers/react
[7] https://github.com/vercel-labs/json-render/blob/main/LICENSE · https://github.com/vercel-labs/json-render/tree/main/packages/mcp
[8] https://github.com/tambo-ai/tambo/blob/main/LICENSE · https://github.com/tambo-ai/tambo/tree/main/apps/api
ACP: https://agentclientprotocol.com/protocol/tool-calls
