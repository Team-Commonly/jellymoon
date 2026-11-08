// Scrubs a raw acprec log into the committed recording (docs/development/record-acp.md): every pod id becomes a fixed
// placeholder numbered in order of first sight, tokens, home and work paths, ports and session ids are replaced, and the
// result is refused outright if anything token-, path-, host- or mail-shaped is left. Pure: text in, text out.

const ID = /\b(pod|usr|agt|cmp|tsk|run|wak|msg|evt|clm|inv|tcl)_[0-9A-Z]{26}\b/g;
const TOOLU = /\b(toolu)_[0-9A-Za-z]{20,}\b/g;
const UUID = /\b[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}\b/g;
const TOKEN = /\b(jm[ucr]_[A-Za-z0-9_-]{8,}|sk-[A-Za-z0-9_-]{10,}|eyJ[A-Za-z0-9_-]{10,}|Bearer\s+[A-Za-z0-9._~+/=-]{8,})/g;
const LEFT = [
  [/jm[ucr]_/, 'a pod token'], [/\/Users\//, 'a home path'], [/\/home\/(?!user\b)/, 'a home path'], [/\/private\/(tmp|var)/, 'a temp path'],
  [/[A-Za-z0-9._%+-]+@[A-Za-z0-9.-]+\.[a-z]{2,}/i, 'an email address'], [/https?:\/\/(?!127\.0\.0\.1:1\b)/, 'a host other than loopback'],
  [/\bsk-[A-Za-z0-9]/, 'an API key'],
];

/** Replaces every distinct match of re by make(match, n), n counting from 1 in order of first sight. */
function number(text, re, make) {
  const seen = new Map();
  return text.replace(re, (m, ...g) => { if (!seen.has(m)) seen.set(m, make(g[0], seen.size + 1)); return seen.get(m); });
}

/** swaps is a list of [from, to] path pairs, longest first; header is the recording's first line. */
export function scrub(raw, header, swaps) {
  let text = raw.split('\n').filter((l) => !/available_commands_update|_auth\/status_update/.test(l)).join('\n'); // the machine's skill list and sign-in account: noise to the daemon, private to us
  for (const [from, to] of swaps) if (from) text = text.split(from).join(to);
  text = number(text, ID, (prefix, n) => `${prefix}_${String(n).padStart(26, '0')}`);
  text = text.replace(/https?:\/\/(?!127\.0\.0\.1:\d)[^\s"\\]+/g, '[url]').replace(TOKEN, '[token]').replace(/127\.0\.0\.1:\d+/g, '127.0.0.1:1');
  text = number(text, TOOLU, (prefix, n) => `${prefix}_${String(n).padStart(24, '0')}`);
  text = number(text, UUID, (_, n) => `00000000-0000-4000-8000-${String(n).padStart(12, '0')}`);
  for (const [re, what] of LEFT) {
    const m = re.exec(text);
    if (m) throw new Error(`the scrubbed recording still holds ${what}: ...${text.slice(Math.max(0, m.index - 30), m.index + 40)}...`);
  }
  return `${JSON.stringify({ t: 'header', ...header })}\n${text}`;
}
