// /pod/:id: the stream of cards, the composer and the needs-you filter, over one pod store.
import { useMemo, useRef, useState } from 'react';
import type { KeyboardEvent as ReactKeyEvent } from 'react';
import { createPortal } from 'react-dom';
import { useOutletContext, useParams, useSearchParams } from 'react-router-dom';
import { CardView } from './CardView';
import { buildCard, needsYou } from './cards';
import type { Card } from './cards';
import type { Connect } from './store';
import { usePod } from './usePod';
import { usePodKeys } from './usePodKeys';

function Composer(p: { onPost(text: string): void; error?: string; inputRef: React.RefObject<HTMLTextAreaElement | null> }) {
  const [text, setText] = useState('');
  const send = () => {
    if (!text.trim()) return;
    p.onPost(text);
    setText('');
  };
  const onKeyDown = (e: ReactKeyEvent<HTMLTextAreaElement>) => {
    if (e.key !== 'Enter' || e.shiftKey || e.nativeEvent.isComposing) return;
    e.preventDefault();
    send();
  };
  return (
    <form onSubmit={(e) => { e.preventDefault(); send(); }}>
      <textarea ref={p.inputRef} aria-label="Message" value={text} onChange={(e) => setText(e.target.value)} onKeyDown={onKeyDown} rows={2} />
      <button type="submit">Send</button>
      {p.error && <p role="alert">{p.error}</p>}
    </form>
  );
}

function useNeedsYouFilter() {
  const [params, setParams] = useSearchParams();
  const on = params.get('filter') === 'needs-you';
  return { on, toggle: () => setParams(on ? {} : { filter: 'needs-you' }, { replace: true }) };
}

export function PodPage({ connect }: { connect: (podId: string) => Connect }) {
  const { id = '' } = useParams();
  const slot = useOutletContext<HTMLElement | null>();
  const { store, state } = usePod(id, connect);
  const filter = useNeedsYouFilter();
  const composer = useRef<HTMLTextAreaElement>(null);
  const mine = useMemo(() => (state.you ? needsYou(state.messages, state.you, state.members) : []), [state.messages, state.you, state.members]);
  const cards = useMemo<Card[]>(() => (filter.on ? mine : state.messages).map((m) => {
    const d = m.decision?.id ?? '';
    return buildCard(m, { pending: state.pending[d], error: state.ruleErrors[d] });
  }), [filter.on, mine, state.messages, state.pending, state.ruleErrors]);
  const k = usePodKeys({
    cards,
    rule: (card, optionId) => store.rule(state.messages.find((m) => m.id === card.id)!.decision!.id, optionId),
    toggleFilter: filter.toggle,
    focusComposer: () => composer.current?.focus(),
  });
  const working = state.agents.filter((a) => a.state === 'working').map((a) => a.agent.displayName);
  const pending = k.keys.confirm ? cards[k.keys.focus]?.ask?.options.find((o) => o.id === k.keys.confirm)?.label : undefined;
  return (
    <>
      {slot && createPortal(
        <button type="button" aria-pressed={filter.on} onClick={filter.toggle}>{`Needs you · ${mine.length}`}</button>,
        slot,
      )}
      <section aria-label="Stream">
        {filter.on && mine.length === 0 && <p>Nothing needs you</p>}
        {cards.map((card, i) => (
          <CardView
            key={card.id}
            card={card}
            open={k.folds.has(card.id)}
            current={i === k.keys.focus}
            confirm={k.keys.confirm}
            elRef={k.setEl(card.id)}
            onFocus={() => k.focusCard(i)}
            act={{ toggleFold: () => k.toggleFold(card.id), pick: (o) => k.pick(i, o), rule: k.rule, cancel: k.cancel }}
          />
        ))}
      </section>
      <p aria-live="polite">{working.length === 0 ? '' : `${working.join(', ')} ${working.length === 1 ? 'is' : 'are'} working.`}</p>
      <div role="status" aria-live="polite">{pending ? `Rule ${pending}? Press Enter to rule, Escape to cancel.` : ''}</div>
      <Composer inputRef={composer} onPost={(t) => store.post(t)} error={state.postError} />
    </>
  );
}
