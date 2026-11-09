// Scrubs a raw acprec log into the committed recording (docs/development/record-acp.md). Text the model streamed is replaced by a
// fixed placeholder (an id can be split across chunks, and the replay never reads it); known id shapes become numbered placeholders;
// tokens, paths, ports and urls are replaced. Then the result is held to an allowlist and refused if anything else id-, path-,
// host- or mail-shaped is left. Pure: text in, text out.

const PREFIXES = 'pod|usr|agt|cmp|tsk|run|wak|msg|evt|clm|inv|tcl|org|req|acc|ctc';
const ID = new RegExp(`(?<![A-Za-z0-9])(${PREFIXES})_[0-9A-Za-z]{8,}(?![A-Za-z0-9])`, 'g');
const TOOLU = /(?<![A-Za-z0-9])(toolu)_[0-9A-Za-z]{20,}/g;
const UUID = /\b[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}\b/g;
const TOKEN = /(jm[ucr]_[A-Za-z0-9_-]{8,}|gh[pousr]_[A-Za-z0-9]{16,}|sk-[A-Za-z0-9_-]{10,}|eyJ[A-Za-z0-9_-]{10,}|Bearer\s+[A-Za-z0-9._~+/=-]{8,})/g;
const STREAMED = new Set(['agent_message_chunk', 'agent_thought_chunk', 'user_message_chunk']);

// What may be left. Every word_Xxxxxxxx that is not one of our numbered placeholders must be a name from the ACP/MCP vocabulary below.
const WORDISH = /(?<![A-Za-z0-9])[a-z]+_[A-Za-z0-9]{8,}/g;
const PLACEHOLDER = /^[a-z]+_\d{24,26}$/;
// The Codex adapter adds its own vocabulary (its Guardian Review step, the pod's tool schema words).
const VOCABULARY = new Set(['in_progress', 'task_assigned', 'tool_reference', 'total_deferred', 'jm_evidence', 'jm_connector', 'guardian_assessment', 'review_required', 'repl_disabled', 'create_generative', 'update_generative']);
const PATH = /(?<=["\s=(\[,])\/[A-Za-z0-9._~-]+(?:\/[^\s"\\,)\]]*)?/g;
const PATH_OK = /^\/(usr|bin|work|repo|tmp\/rec|sandbox\/home|mcp|v1)(\/|$)/;
const TLD = 'local|lan|internal|localdomain|corp|home|com|net|org|io|dev|app|me|ai|co|cloud|xyz|info|edu|gov|us|uk|cn|ru|de';
const HOST = new RegExp(`(?<![A-Za-z0-9@.-])[A-Za-z0-9-]+(?:\\.[A-Za-z0-9-]+)*\\.(?:${TLD})(?![A-Za-z0-9-])`, 'gi');
const IPV4 = /(?<![\d.])(?:\d{1,3}\.){3}\d{1,3}(?![\d.])/g;
const LEFT = [
  [/jm[ucr]_/, 'a pod token'], [/\/Users\//, 'a home path'], [/\/home\/(?!user\b)/, 'a home path'], [/\/private\/(tmp|var)/, 'a temp path'],
  [/[A-Za-z0-9._%+-]+@[A-Za-z0-9.-]+\.[a-z]{2,}/i, 'an email address'], [/https?:\/\/(?!127\.0\.0\.1:1\b)/, 'a url'], [/\bsk-[A-Za-z0-9]/, 'an API key'],
];

/** Replaces every distinct match of re by make(prefix, n), n counting from 1 in order of first sight. */
function number(text, re, make) {
  const seen = new Map();
  return text.replace(re, (m, prefix) => { if (!seen.has(m)) seen.set(m, make(prefix, seen.size + 1)); return seen.get(m); });
}

/** A streamed chunk keeps its line and its shape but not its words. */
function flatten(line) {
  let obj;
  try { obj = JSON.parse(line); } catch { return line; }
  const u = obj?.msg?.params?.update;
  if (!u || !STREAMED.has(u.sessionUpdate) || typeof u.content?.text !== 'string') return line;
  u.content.text = '[text]';
  return JSON.stringify(obj);
}

function refuse(text, re, what, ok = () => false) {
  for (const m of text.matchAll(re)) {
    if (!ok(m[0])) throw new Error(`the scrubbed recording still holds ${what}: ...${text.slice(Math.max(0, m.index - 30), m.index + m[0].length + 30)}...`);
  }
}

/** swaps is a list of [from, to] path pairs, longest first; header is the recording's first line. */
export function scrub(raw, header, swaps) {
  let text = raw.split('\n').filter((l) => !/available_commands_update|_auth\/status_update/.test(l)).map(flatten).join('\n'); // skill list and sign-in account: noise to the daemon, private to us
  for (const [from, to] of swaps) if (from) text = text.split(from).join(to);
  text = number(text, ID, (prefix, n) => `${prefix}_${String(n).padStart(26, '0')}`);
  text = text.replace(/https?:\/\/(?!127\.0\.0\.1:\d)[^\s"\\]+/g, '[url]').replace(TOKEN, '[token]').replace(/127\.0\.0\.1:\d+/g, '127.0.0.1:1');
  text = number(text, TOOLU, (prefix, n) => `${prefix}_${String(n).padStart(24, '0')}`);
  text = number(text, UUID, (_, n) => `00000000-0000-4000-8000-${String(n).padStart(12, '0')}`);
  for (const [re, what] of LEFT) refuse(text, new RegExp(re.source, `${re.flags.replace('g', '')}g`), what);
  refuse(text, WORDISH, 'a name that is not one of our placeholders', (m) => PLACEHOLDER.test(m) || VOCABULARY.has(m));
  refuse(text, PATH, 'an absolute path', (m) => PATH_OK.test(m));
  refuse(text, HOST, 'a host that is not loopback');
  refuse(text, IPV4, 'an address that is not loopback', (m) => m === '127.0.0.1');
  return `${JSON.stringify({ t: 'header', ...header })}\n${text}`;
}
