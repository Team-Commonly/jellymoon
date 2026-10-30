> **Draft, not posted.** A proposed issue for github.com/thesysdev/openui. Posting it upstream waits on an owner decision.

**Title:** Strict data-only mode: render an ElementNode tree, and a parse option that allows only literal component calls

**Body:**

Hi! We build Commonly, where coding agents post to a shared space and a person approves or rejects their work, and we render cards with OpenUI. Two gaps make that awkward, and we'd rather fix them here than keep working around them.

In our current product the model writes Lang, and we keep it inert with our own allowlist, about 260 lines. Only our catalogue components are callable, plus one `Action` wrapping an `OpenUrl` with a literal https URL. Everything else is refused: `Query`, `Mutation`, `Run`, functions, `$state` and other capability refs, reactive expressions. Expansion is capped at 2,048 nodes and 128 levels. As far as we can tell every host that renders model-written Lang across a trust boundary writes some version of this, and has to update it by hand when the language grows.

In the next version the model won't write Lang at all. It picks a component and sends typed JSON. Our server checks that JSON against a schema, and checks that every number traces back to a recorded tool call, before anything is stored. So by the time a card reaches the browser it's plain data we already trust.

`Renderer` only takes `response: string`, though. To show that data we build an `ElementNode` tree, run `jsonToOpenUI` on it, and `Renderer` parses the result straight back. It works (thanks for `jsonToOpenUI`), but the round-trip does nothing for us.

**Proposal**

Two opt-in additions. Defaults stay as they are.

```tsx
// 1. Render a tree directly; skips lexing and parsing.
<Renderer elements={tree} library={library} />   // tree: ElementNode, mutually exclusive with `response`

// 2. Strict parse: only component calls with literal props.
const result = parse(source, library, { strict: true, maxNodes: 2048, maxDepth: 128 });
// result.errors lists each refused construct with its location:
//   { code: "strict/query", ... } | "strict/mutation" | "strict/run" | "strict/state"
//   | "strict/function" | "strict/expression" | "strict/budget"
<Renderer response={source} library={library} strict />  // same rules at render time, no toolProvider allowed
```

In strict mode an `ElementNode` with a non-literal prop is rejected rather than evaluated, and passing `toolProvider` together with `strict` is an error.

Anyone rendering model output across a trust boundary would get a documented baseline instead of a private allowlist that drifts. Apps that build UI from structured data could skip Lang and keep the same component library.

We're happy to send the PR: `elements` on `Renderer` first, since it's mostly plumbing, then `strict` in `lang-core`, porting the tests we already run for our allowlist and expansion budget. If you'd want a different shape or name, say so and we'll follow it.
