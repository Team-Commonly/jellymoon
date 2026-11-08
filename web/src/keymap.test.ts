import { describe, expect, it } from 'vitest';
import { handleKey, initialKeys, nextOpenAsk, type KeyCard, type KeyCtx, type KeyState } from './keymap';

const quiet: KeyCard = { open: false, options: [] };
const open3: KeyCard = { open: true, options: ['a', 'b', 'c'], recommended: 'b' };
const ctx = (cards: KeyCard[], inTextField = false): KeyCtx => ({ cards, inTextField });
const at = (focus: number, over: Partial<KeyState> = {}): KeyState => ({ ...initialKeys, focus, ...over });

describe('moving and folding', () => {
  const cards = [quiet, open3, quiet];
  it('j and k move focus and stop at the ends', () => {
    expect(handleKey(at(0), 'j', ctx(cards))).toEqual({ state: at(1), action: { type: 'focus', index: 1 } });
    expect(handleKey(at(1), 'k', ctx(cards)).action).toEqual({ type: 'focus', index: 0 });
    expect(handleKey(at(0), 'k', ctx(cards)).action).toBeNull();
    expect(handleKey(at(2), 'j', ctx(cards)).action).toBeNull();
  });
  it('Enter folds or opens the focused card', () => {
    expect(handleKey(at(1), 'Enter', ctx(cards)).action).toEqual({ type: 'toggle-fold' });
  });
  it('does nothing with no cards', () => {
    expect(handleKey(at(0), 'j', ctx([])).action).toBeNull();
    expect(handleKey(at(0), 'Enter', ctx([])).action).toBeNull();
  });
});

describe('ruling takes a confirm step', () => {
  const cards = [quiet, open3];
  it('a opens a confirm row on the recommended option, and rules nothing yet', () => {
    const r = handleKey(at(1), 'a', ctx(cards));
    expect(r.action).toEqual({ type: 'confirm', optionId: 'b' });
    expect(r.state.confirm).toBe('b');
  });
  it('1..n opens a confirm row for that option; past n does nothing', () => {
    expect(handleKey(at(1), '1', ctx(cards)).action).toEqual({ type: 'confirm', optionId: 'a' });
    expect(handleKey(at(1), '3', ctx(cards)).action).toEqual({ type: 'confirm', optionId: 'c' });
    expect(handleKey(at(1), '4', ctx(cards)).action).toBeNull();
    expect(handleKey(at(1), '0', ctx(cards)).action).toBeNull();
  });
  it('a and digits do nothing on a card that is not an open ask', () => {
    expect(handleKey(at(0), 'a', ctx(cards)).action).toBeNull();
    expect(handleKey(at(0), '1', ctx(cards)).action).toBeNull();
  });
  it('Enter rules the confirmed option once, then clears the confirm', () => {
    const r = handleKey(at(1, { confirm: 'c' }), 'Enter', ctx(cards));
    expect(r.action).toEqual({ type: 'rule', optionId: 'c' });
    expect(r.state.confirm).toBeNull();
  });
  it('Esc cancels the confirm row', () => {
    const r = handleKey(at(1, { confirm: 'c' }), 'Escape', ctx(cards));
    expect(r.action).toEqual({ type: 'cancel' });
    expect(r.state.confirm).toBeNull();
  });
  it('while confirming, other keys neither move nor rule', () => {
    for (const k of ['j', 'k', 'a', '1', 'g', '/']) {
      const r = handleKey(at(1, { confirm: 'c' }), k, ctx(cards));
      expect(r.action).toBeNull();
      expect(r.state.confirm).toBe('c');
    }
  });
  it('Enter without a confirm row folds, it never rules', () => {
    expect(handleKey(at(1), 'Enter', ctx(cards)).action).toEqual({ type: 'toggle-fold' });
  });
  it('a ruled card cannot be ruled again', () => {
    expect(handleKey(at(0), 'a', ctx([{ open: false, options: ['a', 'b'], recommended: 'a' }])).action).toBeNull();
  });
});

describe('filter and composer', () => {
  it('g then n toggles the needs-you filter', () => {
    const first = handleKey(at(0), 'g', ctx([quiet]));
    expect(first.action).toBeNull();
    expect(first.state.g).toBe(true);
    const second = handleKey(first.state, 'n', ctx([quiet]));
    expect(second.action).toEqual({ type: 'toggle-needs-you' });
    expect(second.state.g).toBe(false);
  });
  it('g then another key drops the prefix and does nothing', () => {
    const r = handleKey(at(0, { g: true }), 'j', ctx([quiet, quiet]));
    expect(r.action).toBeNull();
    expect(r.state.g).toBe(false);
  });
  it('n alone does nothing; / focuses the composer', () => {
    expect(handleKey(at(0), 'n', ctx([quiet])).action).toBeNull();
    expect(handleKey(at(0), '/', ctx([quiet])).action).toEqual({ type: 'focus-composer' });
  });
});

describe('text field guard', () => {
  it('fires nothing while focus is in a text field, whatever the key', () => {
    const cards = [open3];
    for (const k of ['j', 'k', 'a', '1', 'g', 'n', '/', 'Enter', 'Escape']) {
      const start = at(0, { confirm: k === 'Enter' ? 'a' : null });
      const r = handleKey(start, k, ctx(cards, true));
      expect(r.action).toBeNull();
      expect(r.state).toEqual(start);
    }
  });
});

describe('focus after a ruling', () => {
  const cards = [open3, quiet, open3, open3];
  it('moves to the next open ask after the ruled one', () => {
    expect(nextOpenAsk([quiet, quiet, open3, open3], 0)).toBe(2);
    expect(nextOpenAsk(cards, 0)).toBe(2);
  });
  it('falls back to an earlier open ask, then stays put', () => {
    expect(nextOpenAsk([open3, quiet, quiet], 2)).toBe(0);
    expect(nextOpenAsk([quiet, quiet], 1)).toBe(1);
  });
});
