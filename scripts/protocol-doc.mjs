// Renders docs/protocol.md, the human-readable protocol reference, from the schema data scripts/gen.mjs
// already loaded. Pure: same input, same text. Nothing here defines a wire shape.
const esc = (s) => String(s).replace(/\|/g, '\\|').replace(/\s*\n\s*/g, ' ');
const code = (s) => `\`${s}\``;
const link = (name) => `[${name}](#def-${name})`;
const refName = (ref) => /\$defs\/([A-Za-z0-9]+)$/.exec(ref)[1];

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
function fieldTable(s) {
  const req = new Set(s.required || []);
  let out = '| Field | Type | Required | Notes |\n|---|---|---|---|\n';
  for (const [k, p] of Object.entries(s.properties)) {
    const lim = [...limits(p), ...(p.items ? limits(p.items) : [])].join(', ');
    const notes = p.description && lim ? `${p.description.replace(/\.$/, '')}. Limit: ${lim}` : p.description || lim;
    out += `| ${code(k)} | ${esc(typeText(p))} | ${req.has(k) ? 'yes' : 'no'} | ${esc(notes)} |\n`;
  }
  return `${out}\n`;
}

function renderDef(name, s) {
  let out = `<a id="def-${name}"></a>\n### ${name}\n\n`;
  if (s.description) out += `${s.description}\n\n`;
  const lim = s.type === 'object' ? range(s.minProperties, s.maxProperties, 'field') : null;
  if (lim) out += `Limit: ${lim}.\n\n`;
  if (s.type === 'object' && Object.keys(s.properties || {}).length) return out + fieldTable(s);
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
const rules = (list) => (list?.length ? `### Rules\n\n${list.map((r) => `- ${r}`).join('\n')}\n\n` : '');
const head = (docs, file, title) => `## ${title}\n\n${docs[file].description ? `${docs[file].description}\n\n` : ''}${rules(docs[file]['x-rules'])}`;
const defsOf = (ctx, file, skip = []) => [...ctx.defs].filter(([n, d]) => d.file === file && !skip.includes(n));
const renderDefs = (list) => list.map(([n, d]) => renderDef(n, d.schema)).join('');

function commonSection(ctx) {
  const rest = defsOf(ctx, 'common', ['ErrorCode', 'ErrorBody']);
  const ids = rest.filter(([, d]) => isUlidId(d.schema)).map(([n]) => n);
  return head(ctx.docs, 'common', 'Ids, actors and harnesses') + `### Ids\n\n${idTable(ids, ctx.defs)}`
    + renderDefs(rest.filter(([n]) => !ids.includes(n)));
}

function errorsSection(ctx) {
  const places = [
    ...Object.entries(ctx.docs).filter(([, d]) => d['x-rules']).map(([f, d]) => ({ where: `${d.title} rules`, texts: d['x-rules'] })),
    ...ctx.tools.map((t) => ({ where: `tool ${code(t.name)}`, texts: [t.description] })),
  ];
  let out = '## Errors\n\n### Error codes\n\n| Code | Named in |\n|---|---|\n';
  for (const c of ctx.defs.get('ErrorCode').schema.enum) {
    const re = new RegExp(`(?<![A-Za-z_])${c}(?![A-Za-z_])`);
    const hits = places.filter((p) => p.texts.some((t) => re.test(t))).map((p) => p.where);
    out += `| ${code(c)} | ${hits.length ? hits.join(', ') : '—'} |\n`;
  }
  return `${out}\n${renderDef('ErrorCode', ctx.defs.get('ErrorCode').schema)}${renderDef('ErrorBody', ctx.defs.get('ErrorBody').schema)}`;
}

function tokensSection(ctx) {
  const prefixes = ctx.docs.tokens['x-prefixes'];
  let out = `${head(ctx.docs, 'tokens', 'Tokens')}| Token | Prefix |\n|---|---|\n`;
  for (const [kind, p] of Object.entries(prefixes)) out += `| ${kind} | ${code(p)} |\n`;
  return `${out}\n${renderDefs(defsOf(ctx, 'tokens'))}`;
}

function podApiSection(ctx) {
  const who = { none: 'anyone', user: 'a signed-in user', computer: 'a computer', run: 'an agent run' };
  const pre = ctx.docs.tokens['x-prefixes'];
  const auth = (a) => a.split('|').map((x) => `${who[x]}${pre[x] ? ` (${code(pre[x])})` : ''}`).join(' or ');
  const cell = (n) => (n ? link(n) : '—');
  let out = `${head(ctx.docs, 'pod-api', 'Worker HTTP API')}### Routes\n\n| Method | Path | Who may call | Request body | Response |\n|---|---|---|---|---|\n`;
  for (const r of ctx.docs['pod-api']['x-routes']) {
    const resp = r.response ? link(r.response) : r.upgrade ? `${r.upgrade === 'websocket' ? 'WebSocket' : r.upgrade} upgrade` : r.protocol || '—';
    out += `| ${r.method} | ${code(r.path)} | ${auth(r.auth)} | ${cell(r.request)} | ${resp} |\n`;
  }
  return `${out}\n${renderDefs(defsOf(ctx, 'pod-api'))}`;
}

function eventsSection(ctx) {
  const dir = { s2c: 'pod to client', c2s: 'client to pod', local: 'daemon to TUI only' };
  let out = `${head(ctx.docs, 'events', 'Socket events')}### Event list\n\n`;
  out += '| Event | Direction | Data |\n|---|---|---|\n';
  for (const t of ctx.eventTypes) out += `| ${code(t)} | ${ctx.variants[t].dir}: ${dir[ctx.variants[t].dir]} | ${link(ctx.variantData[t])} |\n`;
  return `${out}\n${renderDefs(defsOf(ctx, 'events'))}`;
}

function toolsSection(ctx) {
  let out = `${head(ctx.docs, 'tools', 'MCP tools')}### The ${ctx.tools.length} tools\n\n`;
  for (const t of ctx.tools) {
    out += `<a id="tool-${t.name}"></a>\n#### ${code(t.name)}\n\n${t.description}\n\n`;
    out += `- Input: ${link(t.inputDef)}. Fields: ${fieldNames(t.inputDef, ctx.defs)}.\n`;
    out += `- Output: ${link(t.outputDef)}. Fields: ${fieldNames(t.outputDef, ctx.defs)}.\n\n`;
  }
  return out + renderDefs(defsOf(ctx, 'tools'));
}

const INTRO = `<!-- Generated by scripts/gen.mjs from schema/. DO NOT EDIT. Run \`make gen\`. -->

# Jellymoon protocol

The wire protocol as the schema defines it, for review without reading JSON Schema. Terms are used as
defined in [GLOSSARY.md](GLOSSARY.md). Every definition appears once and is linked wherever it is used.

How to read a table: **Required** says whether the field must be present. Limits in **Notes** come from the
schema.`;

export function renderProtocol(ctx) {
  const closed = [...ctx.defs.values()].every(({ schema }) => schema.type !== 'object' || schema.additionalProperties === false);
  const c = ctx;
  const toc = ['Ids, actors and harnesses', 'Errors', 'Tokens', 'Worker HTTP API', 'Socket events', 'Messages and cards', 'Tasks', 'MCP tools'];
  return [
    `${INTRO}${closed ? ' An object accepts only the fields listed; anything else fails validation.' : ''}\n`,
    `Contents: ${toc.map((t) => `[${t}](#${t.toLowerCase().replace(/[^a-z0-9 ]/g, '').replace(/ /g, '-')})`).join(' · ')}\n`,
    commonSection(c),
    errorsSection(c),
    tokensSection(c),
    podApiSection(c),
    eventsSection(c),
    head(c.docs, 'message', 'Messages and cards') + renderDefs(defsOf(c, 'message')),
    head(c.docs, 'task', 'Tasks') + renderDefs(defsOf(c, 'task')),
    toolsSection(c),
  ].join('\n').replace(/\n{3,}/g, '\n\n').trimEnd() + '\n';
}
