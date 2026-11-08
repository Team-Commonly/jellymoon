// The keyboard map: a pure function from (focus state, key, whether a text field has focus) to the next
// state and at most one action. The view applies the action; nothing here touches the DOM.
import type { OptionId } from '../../gen/ts/protocol';

export interface KeyCard { open: boolean; options: OptionId[]; recommended?: OptionId }
export interface KeyCtx { cards: KeyCard[]; inTextField: boolean }
export interface KeyState { focus: number; confirm: OptionId | null; g: boolean }
type KeyAction =
  | { type: 'focus'; index: number }
  | { type: 'toggle-fold' }
  | { type: 'confirm'; optionId: OptionId }
  | { type: 'rule'; optionId: OptionId }
  | { type: 'cancel' }
  | { type: 'toggle-needs-you' }
  | { type: 'focus-composer' };
interface Result { state: KeyState; action: KeyAction | null }

export const initialKeys: KeyState = { focus: 0, confirm: null, g: false };

const stay = (state: KeyState): Result => ({ state, action: null });

function confirming(s: KeyState, key: string): Result {
  if (key === 'Enter') return { state: { ...s, confirm: null }, action: { type: 'rule', optionId: s.confirm! } };
  if (key === 'Escape') return { state: { ...s, confirm: null }, action: { type: 'cancel' } };
  return stay(s);
}

function pick(s: KeyState, optionId: OptionId | undefined): Result {
  if (!optionId) return stay(s);
  return { state: { ...s, confirm: optionId }, action: { type: 'confirm', optionId } };
}

function onCard(s: KeyState, key: string, card: KeyCard): Result {
  if (!card.open) return stay(s);
  if (key === 'a') return pick(s, card.recommended);
  if (/^[1-5]$/.test(key)) return pick(s, card.options[Number(key) - 1]);
  return stay(s);
}

function move(s: KeyState, to: number, count: number): Result {
  if (to < 0 || to >= count || to === s.focus) return stay(s);
  return { state: { ...s, focus: to }, action: { type: 'focus', index: to } };
}

export function handleKey(s: KeyState, key: string, ctx: KeyCtx): Result {
  if (ctx.inTextField) return stay(s);
  if (s.confirm) return confirming(s, key);
  if (s.g) {
    const next = { ...s, g: false };
    return key === 'n' ? { state: next, action: { type: 'toggle-needs-you' } } : stay(next);
  }
  const count = ctx.cards.length;
  if (key === 'g') return stay({ ...s, g: true });
  if (key === '/') return { state: s, action: { type: 'focus-composer' } };
  if (key === 'j') return move(s, s.focus + 1, count);
  if (key === 'k') return move(s, s.focus - 1, count);
  const card = ctx.cards[s.focus];
  if (!card) return stay(s);
  if (key === 'Enter') return { state: s, action: { type: 'toggle-fold' } };
  return onCard(s, key, card);
}

/** After a ruling lands: the next open ask after `from`, else the nearest one before it, else `from`. */
export function nextOpenAsk(cards: KeyCard[], from: number): number {
  const after = cards.findIndex((c, i) => i > from && c.open);
  if (after >= 0) return after;
  for (let i = Math.min(from, cards.length) - 1; i >= 0; i--) if (cards[i].open) return i;
  return from;
}
