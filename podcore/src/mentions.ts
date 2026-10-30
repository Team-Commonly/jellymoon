// Which parts of a message are not prose: CommonMark fenced blocks and code spans (ADR 0003). Pure string work.
const FENCE = /^ {0,3}(`{3,}|~{3,})(.*)$/;

/** True when `line` closes a fence opened with `open`: same character, at least as long, nothing after. */
const closes = (line: string, open: string): boolean => {
  const m = /^ {0,3}(`{3,}|~{3,})[ \t]*$/.exec(line);
  return m !== null && m[1][0] === open[0] && m[1].length >= open.length;
};

/** A paragraph's text with every code span replaced by a space. A run of n backticks closes at the next run of exactly n. */
function stripSpans(text: string): string {
  let out = '';
  let i = 0;
  while (i < text.length) {
    const ch = text[i];
    if (ch === '\\' && i + 1 < text.length && /[!-/:-@[-`{-~]/.test(text[i + 1])) { out += text.slice(i, i + 2); i += 2; continue; }
    if (ch !== '`') { out += ch; i += 1; continue; }
    let n = 1;
    while (text[i + n] === '`') n += 1;
    let j = i + n;
    let end = -1;
    while (end < 0 && j < text.length) {
      if (text[j] !== '`') { j += 1; continue; }
      let run = 1;
      while (text[j + run] === '`') run += 1;
      if (run === n) end = j;
      j += run;
    }
    out += end < 0 ? text.slice(i, i + n) : ' ';
    i = end < 0 ? i + n : end + n;
  }
  return out;
}

/** The text a mention may live in: fenced blocks and code spans removed. An unclosed fence runs to the end; an unclosed backtick is literal. */
export function prose(text: string): string {
  const parts: string[] = [];
  let para: string[] = [];
  let open: string | null = null;
  const flush = (): void => { if (para.length > 0) parts.push(stripSpans(para.join('\n'))); para = []; };
  for (const line of text.split(/\r\n|\r|\n/)) {
    const m: RegExpExecArray | null = open === null ? FENCE.exec(line) : null;
    if (open !== null) { if (closes(line, open)) open = null; } else if (m !== null && !(m[1][0] === '`' && m[2].includes('`'))) { flush(); open = m[1]; } else if (line.trim() === '') flush();
    else para.push(line);
  }
  flush();
  return parts.join('\n');
}
