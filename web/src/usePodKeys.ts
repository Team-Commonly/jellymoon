// The keyboard for the stream. The pure key map decides; this hook applies the one action it returns to the DOM
// and to the store. After a ruling lands it moves focus to the next open ask.
import { useCallback, useEffect, useRef, useState } from 'react';
import type { OptionId } from '../../gen/ts/protocol';
import type { Card } from './cards';
import { handleKey, initialKeys, nextOpenAsk } from './keymap';
import type { KeyCard, KeyState } from './keymap';

export interface KeyHost {
  cards: Card[];
  rule(card: Card, optionId: OptionId): void;
  toggleFilter(): void;
  focusComposer(): void;
}

const keyCard = (c: Card): KeyCard => ({
  open: c.ask?.state === 'open',
  options: c.ask?.options.map((o) => o.id) ?? [],
  recommended: c.ask?.options.find((o) => o.recommended)?.id,
});

const inTextField = (t: EventTarget | null) =>
  t instanceof HTMLElement && (['INPUT', 'TEXTAREA', 'SELECT'].includes(t.tagName) || t.isContentEditable);

type Awaiting = React.RefObject<{ id: string; index: number } | null>;

/** Once the ruled card is no longer open, move to the next open ask. A refused ruling leaves the card open and waits. */
function useAfterRuling(awaiting: Awaiting, cards: Card[], goTo: (index: number) => void) {
  useEffect(() => {
    const a = awaiting.current;
    if (!a) return;
    const kept = cards.find((c) => c.id === a.id);
    if (kept?.ask?.state === 'open') return;
    awaiting.current = null;
    const next = nextOpenAsk(cards.map(keyCard), kept ? a.index : a.index - 1);
    goTo(Math.max(0, Math.min(next, cards.length - 1)));
  });
}

function useFolds() {
  const [folds, setFolds] = useState<Set<string>>(new Set());
  const toggleFold = useCallback((id: string) => setFolds((f) => {
    const next = new Set(f);
    if (!next.delete(id)) next.add(id);
    return next;
  }), []);
  return { folds, toggleFold };
}

export function usePodKeys(host: KeyHost) {
  const [keys, setKeysState] = useState<KeyState>(initialKeys);
  const keysRef = useRef(keys);
  const hostRef = useRef(host);
  hostRef.current = host;
  const els = useRef(new Map<string, HTMLElement>());
  const { folds, toggleFold } = useFolds();
  const awaiting = useRef<{ id: string; index: number } | null>(null);

  const setKeys = useCallback((k: KeyState) => {
    keysRef.current = k;
    setKeysState(k);
  }, []);
  const goTo = useCallback((index: number) => {
    setKeys({ ...keysRef.current, focus: index, confirm: null });
    const el = els.current.get(hostRef.current.cards[index]?.id ?? '');
    el?.focus();
    el?.scrollIntoView?.({ block: 'nearest' });
  }, [setKeys]);
  const onKey = useCallback((e: KeyboardEvent) => {
    if (e.ctrlKey || e.metaKey || e.altKey) return;
    if (e.key === 'Enter' && e.target instanceof HTMLButtonElement) return;
    const { cards } = hostRef.current;
    const r = handleKey(keysRef.current, e.key, { cards: cards.map(keyCard), inTextField: inTextField(e.target) });
    setKeys(r.state);
    const card = cards[r.state.focus];
    if (!r.action) return;
    e.preventDefault();
    if (r.action.type === 'focus') goTo(r.action.index);
    else if (r.action.type === 'toggle-fold' && card) toggleFold(card.id);
    else if (r.action.type === 'rule' && card) {
      awaiting.current = { id: card.id, index: r.state.focus };
      hostRef.current.rule(card, r.action.optionId);
    } else if (r.action.type === 'toggle-needs-you') hostRef.current.toggleFilter();
    else if (r.action.type === 'focus-composer') hostRef.current.focusComposer();
  }, [goTo, setKeys, toggleFold]);

  useEffect(() => {
    document.addEventListener('keydown', onKey);
    return () => document.removeEventListener('keydown', onKey);
  }, [onKey]);

  useAfterRuling(awaiting, host.cards, goTo);

  return {
    keys,
    folds,
    toggleFold,
    setEl: (id: string) => (el: HTMLElement | null) => void (el ? els.current.set(id, el) : els.current.delete(id)),
    focusCard: (index: number) => keysRef.current.focus !== index && setKeys({ ...keysRef.current, focus: index, confirm: null }),
    pick: (index: number, optionId: OptionId) => {
      setKeys({ ...keysRef.current, focus: index, confirm: optionId });
      els.current.get(hostRef.current.cards[index]?.id ?? '')?.focus();
    },
    rule: () => onKey(new KeyboardEvent('keydown', { key: 'Enter' })),
    cancel: () => setKeys({ ...keysRef.current, confirm: null }),
  };
}
