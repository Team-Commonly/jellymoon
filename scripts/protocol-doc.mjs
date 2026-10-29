// Renders docs/protocol.md, the human-readable protocol reference, from the schema data scripts/gen.mjs
// already loaded. Pure: same input, same text. Nothing here defines a wire shape.
const esc = (s) => String(s).replace(/\|/g, '\\|').replace(/\s*\n\s*/g, ' ');
const code = (s) => `\`${s}\``;
const link = (name) => `[${name}](#def-${name})`;
const refName = (ref) => /\$defs\/([A-Za-z0-9]+)$/.exec(ref)[1];
const slug = (t) => t.toLowerCase().replace(/[^a-z0-9 ]/g, '').replace(/ /g, '-');

// Section titles, in document order. head(), the contents line and the errors table all read this one list.
const TITLE = {
  common: 'Ids, actors and harnesses', errors: 'Errors', tokens: 'Tokens', 'pod-api': 'Worker HTTP API',
  events: 'Socket events', message: 'Messages and cards', task: 'Tasks', operations: 'Operations',
};
const rulesTitle = (file) => `Rules for ${TITLE[file]}`;

// ---------- types and limits in plain words ----------
function patternText(p) {
  let m = /^\^\(?([a-z|]+)\)?_\[0-9A-HJKMNP-TV-Z\]\{26\}\$$/.exec(p);
  if (m) {
    const ps = m[1].split('|').map((x) => code(`${x}_`));
    return `starts with ${ps.length > 1 ? `${ps.slice(0, -1).join(', ')} or ${ps.at(-1)}` : ps[0]} then a 26-character ULID`;
  }
  m = /^\^([a-z]+_)\[0-9A-Za-z\]\{(\d+)\}\$$/.exec(p);
  if (m) return `starts with ${code(m[1])} then ${m[2]} letters or digits`;
  if (p === '^[^\\r\\n]*$') return 'one line, no line breaks';
  return `matches ${code(p)}`;
}

function range(lo, hi, unit) {
  const u = unit ? ` ${unit}` : '';
  if (lo !== undefined && lo === hi) return `exactly ${lo}${u}`;
  if (lo !== undefined && hi !== undefined) return `${lo} to ${hi}${u}`;
  if (lo !== undefined) return `at least ${lo}${u}`;
  if (hi !== undefined) return `up to ${hi}${u}`;
  return null;
}

function limits(s) {
  return [
    range(s.minLength, s.maxLength, 'characters'),
    range(s.minItems, s.maxItems, 'items'),
    range(s.minimum, s.maximum, ''),
    s.pattern ? patternText(s.pattern) : null,
  ].filter(Boolean);
}

function typeText(s) {
  if (s.$ref) return link(refName(s.$ref));
  if (s.const !== undefined) return `always ${code(JSON.stringify(s.const))}`;
  if (s.enum) return `one of ${s.enum.map(code).join(', ')}`;
  if (s.type === 'array') return `list of ${typeText(s.items)}`;
  return s.type;
}

// ---------- definitions ----------
function fieldTable(s, ctx) {
  const req = new Set(s.required || []);
  let out = '| Field | Type | Required | Notes |\n|---|---|---|---|\n';
  for (const [k, p] of Object.entries(s.properties)) {
    const lim = [...limits(p), ...(p.items ? limits(p.items) : [])].join(', ');
    const text = p.description || (p.$ref ? ctx.defs.get(refName(p.$ref)).schema.description : '');
    const notes = text && lim ? `${text.replace(/\.$/, '')}. Limit: ${lim}` : text || lim;
    out += `| ${code(k)} | ${esc(typeText(p))} | ${req.has(k) ? 'yes' : 'no'} | ${esc(notes)} |\n`;
  }
  return `${out}\n`;
}

function renderDef(name, s, ctx) {
  let out = `<a id="def-${name}"></a>\n### ${name}\n\n`;
  if (s.description) out += `${s.description}\n\n`;
  const lim = s.type === 'object' ? range(s.minProperties, s.maxProperties, 'field') : null;
  if (lim) out += `Limit: ${lim}.\n\n`;
  if (s.type === 'object' && Object.keys(s.properties || {}).length) return out + fieldTable(s, ctx);
  if (s.type === 'object') return `${out}No fields.\n\n`;
  if (s.enum) return `${out}One of: ${s.enum.map(code).join(', ')}.\n\n`;
  return `${out}${/^[aeiou]/.test(s.type) ? 'An' : 'A'} ${[s.type, ...limits(s)].join(', ')}.\n\n`;
}

const isUlidId = (s) => s.pattern && /\[0-9A-HJKMNP-TV-Z\]\{26\}/.test(s.pattern);

function idTable(names, defs) {
  let out = '| Id | Format | Meaning |\n|---|---|---|\n';
  for (const n of names) {
    const s = defs.get(n).schema;
    out += `| <a id="def-${n}"></a>${n} | ${esc(patternText(s.pattern))} | ${esc(s.description || '')} |\n`;
  }
  return `${out}\n`;
}

function fieldNames(def, defs) {
  const s = defs.get(def).schema;
  const props = Object.keys(s.properties || {});
  if (!props.length) return 'no fields';
  const req = new Set(s.required || []);
  return props.map((p) => code(p) + (req.has(p) ? ' (required)' : '')).join(', ');
}

// ---------- sections ----------
// The raw auth value, then the token it names (tokens.json x-prefixes) or that none is needed.
const authText = (a, pre) => a.split('|').map((x) => (pre[x] ? `${code(x)}: ${x} token (${code(pre[x])})` : `${code(x)}: no token`)).join(' or ');
const rules = (file, list) => (list?.length ? `### ${rulesTitle(file)}\n\n${list.map((r) => `- ${r}`).join('\n')}\n\n` : '');
const head = (docs, file) => `## ${TITLE[file]}\n\n${docs[file].description ? `${docs[file].description}\n\n` : ''}${rules(file, docs[file]['x-rules'])}`;
const defsOf = (ctx, file, skip = []) => [...ctx.defs].filter(([n, d]) => d.file === file && !skip.includes(n));
const renderDefs = (ctx, list) => list.map(([n, d]) => renderDef(n, d.schema, ctx)).join('');

function commonSection(ctx) {
  const rest = defsOf(ctx, 'common', ['ErrorCode', 'ErrorBody']);
  const ids = rest.filter(([, d]) => isUlidId(d.schema)).map(([n]) => n);
  return head(ctx.docs, 'common') + `### Ids\n\n${idTable(ids, ctx.defs)}`
    + renderDefs(ctx, rest.filter(([n]) => !ids.includes(n)));
}

function errorsSection(ctx) {
  const errors = ctx.defs.get('ErrorCode').schema;
  // Places an error code is named, as the doc's own headings: a Rules heading or an operation.
  const places = [
    ...Object.entries(ctx.docs).filter(([, d]) => d['x-rules']).map(([f, d]) => ({ where: `[${rulesTitle(f)}](#${slug(rulesTitle(f))})`, texts: d['x-rules'] })),
    ...ctx.operations.map((o) => ({ where: `[${TITLE.operations}: ${o.name}](#op-${o.name})`, texts: [o.description] })),
  ];
  let out = `## ${TITLE.errors}\n\n### Error codes\n\n| Code | HTTP status | Meaning | Named in |\n|---|---|---|---|\n`;
  for (const c of errors.enum) {
    const re = new RegExp(`(?<![A-Za-z_])${c}(?![A-Za-z_])`);
    const hits = places.filter((p) => p.texts.some((t) => re.test(t))).map((p) => p.where);
    const e = errors['x-errors'][c];
    out += `| ${code(c)} | ${e.status} | ${esc(e.meaning)} | ${hits.length ? hits.join(', ') : '\u2014'} |\n`;
  }
  return `${out}\n${renderDef('ErrorCode', errors, ctx)}${renderDef('ErrorBody', ctx.defs.get('ErrorBody').schema, ctx)}`;
}

function tokensSection(ctx) {
  const prefixes = ctx.docs.tokens['x-prefixes'];
  let out = `${head(ctx.docs, 'tokens')}| Token | Prefix |\n|---|---|\n`;
  for (const [kind, p] of Object.entries(prefixes)) out += `| ${kind} | ${code(p)} |\n`;
  return `${out}\n${renderDefs(ctx, defsOf(ctx, 'tokens'))}`;
}

function podApiSection(ctx) {
  const pre = ctx.docs.tokens['x-prefixes'];
  const auth = (a) => authText(a, pre);
  const cell = (n) => (n ? link(n) : '—');
  let out = `${head(ctx.docs, 'pod-api')}### Routes\n\n| Method | Path | Who may call | Request body | Response |\n|---|---|---|---|---|\n`;
  for (const r of ctx.docs['pod-api']['x-routes']) {
    const resp = r.response ? link(r.response) : r.upgrade ? `${r.upgrade === 'websocket' ? 'WebSocket' : r.upgrade} upgrade` : r.protocol || '—';
    out += `| ${r.method} | ${code(r.path)} | ${auth(r.auth)} | ${cell(r.request)} | ${resp} |\n`;
  }
  return `${out}\n${renderDefs(ctx, defsOf(ctx, 'pod-api'))}`;
}

function eventsSection(ctx) {
  const dir = { s2c: 'pod to client', c2s: 'client to pod', local: 'daemon to TUI only' };
  let out = `${head(ctx.docs, 'events')}### Event list\n\n`;
  out += '| Event | Direction | Data |\n|---|---|---|\n';
  for (const t of ctx.eventTypes) out += `| ${code(t)} | ${ctx.variants[t].dir}: ${dir[ctx.variants[t].dir]} | ${link(ctx.variantData[t])} |\n`;
  return `${out}\n${renderDefs(ctx, defsOf(ctx, 'events'))}`;
}

function operationsSection(ctx) {
  const pre = ctx.docs.tokens['x-prefixes'];
  const mcp = (o) => code(ctx.prefix + o.name);
  const ops = ctx.operations;
  let out = `${head(ctx.docs, 'operations')}### The ${ops.length} operations\n\n`;
  out += '| Operation | Method and path | Who may call | MCP tool |\n|---|---|---|---|\n';
  for (const o of ops) out += `| [${o.name}](#op-${o.name}) | ${o.method} ${code(o.path)} | ${authText(o.auth, pre)} | ${mcp(o)} |\n`;
  out += '\n';
  for (const o of ops) {
    out += `<a id="op-${o.name}"></a>\n#### ${code(o.name)}\n\n${o.description}\n\n`;
    out += `- HTTP: ${o.method} ${code(o.path)}\n- Who may call: ${authText(o.auth, pre)}\n- MCP tool: ${mcp(o)}\n`;
    out += `- Input: ${link(o.inputDef)}. Fields: ${fieldNames(o.inputDef, ctx.defs)}.\n`;
    out += `- Output: ${link(o.outputDef)}. Fields: ${fieldNames(o.outputDef, ctx.defs)}.\n\n`;
  }
  return out + renderDefs(ctx, defsOf(ctx, 'operations'));
}

const INTRO = `<!-- Generated by scripts/gen.mjs from schema/. DO NOT EDIT. Run \`make gen\`. -->

# Jellymoon protocol

The wire protocol as the schema defines it, for review without reading JSON Schema. Terms are used as
defined in [GLOSSARY.md](GLOSSARY.md). Every definition appears once and is linked wherever it is used.

How to read a table: **Required** says whether the field must be present. Limits in **Notes** come from the
schema.`;

// The sentence about closed objects is true only as far as the schema makes it so: say which objects it covers.
function closedSentence(defs) {
  const objects = [...defs.values()].filter(({ schema }) => schema.type === 'object');
  if (!objects.every(({ schema }) => schema.additionalProperties === false)) return '';
  const open = [];
  for (const [name, { schema }] of defs) {
    for (const [p, s] of Object.entries(schema.properties ?? {})) if (s.type === 'object' && s.additionalProperties !== false) open.push(`${name}.${p}`);
  }
  const base = ' Every definition that lists fields accepts only those fields; anything else fails validation.';
  return open.length
    ? `${base} The exception is ${open.map(code).join(', ')}: an object whose fields come from the rule in its description.`
    : base;
}

export function renderProtocol(ctx) {
  const toc = Object.values(TITLE);
  return [
    `${INTRO}${closedSentence(ctx.defs)}\n`,
    `Contents: ${toc.map((t) => `[${t}](#${slug(t)})`).join(' \u00b7 ')}\n`,
    commonSection(ctx),
    errorsSection(ctx),
    tokensSection(ctx),
    podApiSection(ctx),
    eventsSection(ctx),
    head(ctx.docs, 'message') + renderDefs(ctx, defsOf(ctx, 'message')),
    head(ctx.docs, 'task') + renderDefs(ctx, defsOf(ctx, 'task')),
    operationsSection(ctx),
  ].join('\n').replace(/\n{3,}/g, '\n\n').trimEnd() + '\n';
}
