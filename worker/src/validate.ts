// Request validation, driven by the generated schema bundle (gen/ts/schemas.json). No limit is copied by hand: a route names a
// definition from schema/ and this walks it. Workers forbid code generation, so a compiling validator (ajv) cannot run here.
// It implements the keywords the contract uses and nothing else; a keyword it does not know fails closed (see `walk`).
import bundle from '../../gen/ts/schemas.json';

interface Schema {
  $ref?: string;
  type?: 'string' | 'integer' | 'number' | 'boolean' | 'object' | 'array';
  enum?: unknown[];
  pattern?: string;
  minLength?: number;
  maxLength?: number;
  minimum?: number;
  maximum?: number;
  minItems?: number;
  maxItems?: number;
  minProperties?: number;
  maxProperties?: number;
  required?: string[];
  properties?: Record<string, Schema>;
  additionalProperties?: boolean;
  items?: Schema;
}

const DEFS = (bundle as unknown as { $defs: Record<string, Schema> }).$defs;
const KEYWORDS = new Set(['$ref', 'type', 'enum', 'pattern', 'minLength', 'maxLength', 'minimum', 'maximum', 'minItems', 'maxItems',
  'minProperties', 'maxProperties', 'required', 'properties', 'additionalProperties', 'items', 'description', 'title', 'default', 'examples']);
const patterns = new Map<string, RegExp>();
const codePoints = (s: string): number => Array.from(s).length;
const isObject = (v: unknown): v is Record<string, unknown> => v !== null && typeof v === 'object' && !Array.isArray(v);

function resolve(s: Schema): Schema {
  if (s.$ref === undefined) return s;
  const name = /^#\/\$defs\/(.+)$/.exec(s.$ref)?.[1];
  const hit = name === undefined || !Object.hasOwn(DEFS, name) ? undefined : DEFS[name];
  if (hit === undefined) throw new Error(`validate: unresolved ${s.$ref}`);
  return resolve(hit);
}

function scalar(s: Schema, v: unknown, at: string): string | null {
  if (s.enum !== undefined && !s.enum.includes(v)) return `${at}: must be one of ${s.enum.map(String).join(', ')}`;
  if (s.type === 'string') {
    if (typeof v !== 'string') return `${at}: must be a string`;
    const n = codePoints(v);
    if (s.minLength !== undefined && n < s.minLength) return `${at}: must be at least ${s.minLength} characters`;
    if (s.maxLength !== undefined && n > s.maxLength) return `${at}: must be at most ${s.maxLength} characters`;
    if (s.pattern !== undefined) {
      let re = patterns.get(s.pattern);
      if (re === undefined) { re = new RegExp(s.pattern, 'u'); patterns.set(s.pattern, re); }
      if (!re.test(v)) return `${at}: does not match the required pattern`;
    }
  } else if (s.type === 'integer' || s.type === 'number') {
    if (typeof v !== 'number' || !Number.isFinite(v) || (s.type === 'integer' && !Number.isInteger(v))) return `${at}: must be ${s.type === 'integer' ? 'an integer' : 'a number'}`;
    if (s.minimum !== undefined && v < s.minimum) return `${at}: must be at least ${s.minimum}`;
    if (s.maximum !== undefined && v > s.maximum) return `${at}: must be at most ${s.maximum}`;
  } else if (s.type === 'boolean' && typeof v !== 'boolean') return `${at}: must be true or false`;
  return null;
}

function walk(raw: Schema, v: unknown, at: string): string | null {
  const s = resolve(raw);
  const unknown = Object.keys(s).find((k) => !KEYWORDS.has(k));
  if (unknown !== undefined) throw new Error(`validate: keyword ${unknown} is not supported`);
  if (s.type === 'object') {
    if (!isObject(v)) return `${at}: must be an object`;
    const keys = Object.keys(v);
    if (s.minProperties !== undefined && keys.length < s.minProperties) return `${at}: needs at least ${s.minProperties} fields`;
    if (s.maxProperties !== undefined && keys.length > s.maxProperties) return `${at}: allows at most ${s.maxProperties} fields`;
    for (const r of s.required ?? []) if (!Object.hasOwn(v, r)) return `${at}.${r}: is required`;
    for (const k of keys) {
      const sub = s.properties !== undefined && Object.hasOwn(s.properties, k) ? s.properties[k] : undefined;
      if (sub === undefined) { if (s.additionalProperties === false) return `${at}.${k}: is not a field of this request`; continue; }
      const bad = walk(sub, v[k], `${at}.${k}`);
      if (bad !== null) return bad;
    }
    return null;
  }
  if (s.type === 'array') {
    if (!Array.isArray(v)) return `${at}: must be an array`;
    if (s.minItems !== undefined && v.length < s.minItems) return `${at}: needs at least ${s.minItems} items`;
    if (s.maxItems !== undefined && v.length > s.maxItems) return `${at}: allows at most ${s.maxItems} items`;
    for (let i = 0; i < v.length && s.items !== undefined; i += 1) {
      const bad = walk(s.items, v[i], `${at}[${i}]`);
      if (bad !== null) return bad;
    }
    return null;
  }
  return scalar(s, v, at);
}

/**
 * Checks `value` against the named definition of the generated bundle. `fromPath` names fields the URL supplies (`taskId` in
 * `/tasks/{taskId}`): they are not repeated in the body, so one present is refused like any unknown field. Returns the first
 * problem as a sentence, or null when the value is valid. `optional` names fields that are required by the schema but whose
 * absence the pod answers with a code of its own (a post's summary: summary_required); they are still checked when present.
 */
export function check(def: string, value: unknown, fromPath: readonly string[] = [], optional: readonly string[] = []): string | null {
  const s = resolve({ $ref: `#/$defs/${def}` });
  const own: Schema = {
    ...s,
    required: (s.required ?? []).filter((r) => !fromPath.includes(r) && !optional.includes(r)),
    properties: Object.fromEntries(Object.entries(s.properties ?? {}).filter(([k]) => !fromPath.includes(k))),
  };
  return walk(own, value, 'body');
}
