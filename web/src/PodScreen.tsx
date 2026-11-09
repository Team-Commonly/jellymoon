// /pod/:id: the stream of cards, the composer and the needs-you filter, over one pod store.
import { useMemo, useRef, useState } from 'react';
import type { KeyboardEvent as ReactKeyEvent } from 'react';
import { createPortal } from 'react-dom';
import { useOutletContext, useParams, useSearchParams } from 'react-router-dom';
import { CardView } from './CardView';
import { buildCard, needsYou } from './cards';
import type { Card } from './cards';
import { ApiError } from './http';
import type { Http } from './http';
import type { Connect, RunLine } from './store';
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

const ENDED_WORD = { cancelled: 'stopped', completed: 'finished', failed: 'failed', expired: 'timed out' } as const;

/**
 * One line per run: a going run says so and, for someone the stop route would allow, carries Stop; an ended run is one quiet line.
 * Pressing Stop changes nothing on the line. It only disables the button; the line changes when the pod's run.ended arrives.
 */
function RunItem({ run, canStop, stop }: { run: RunLine; canStop: boolean; stop(runId: string): Promise<void> }) {
  const [busy, setBusy] = useState(false);
  const [note, setNote] = useState<string>();
  const name = run.agent.displayName;
  const press = async () => {
    setBusy(true);
    setNote(undefined);
    try {
      await stop(run.runId); // stays disabled: the server accepted it and run.ended is on its way
    } catch (e) {
      setBusy(false);
      setNote(e instanceof ApiError ? e.message : 'Could not reach the server. Try again.');
    }
  };
  if (run.state === 'ended') return <li data-state="ended">{`${name}'s run ${ENDED_WORD[run.outcome ?? 'completed']}.`}</li>;
  return (
    <li data-state="going">
      {`${name} is working on a run.`}
      {canStop && <button type="button" aria-label={`Stop ${name}'s run`} disabled={busy} onClick={press}>Stop</button>}
      {note && <p role="alert">{note}</p>}
    </li>
  );
}

function RunLines({ runs, canStop, stop }: { runs: RunLine[]; canStop: boolean; stop(runId: string): Promise<void> }) {
  if (runs.length === 0) return null;
  return <ul aria-label="Runs" aria-live="polite">{runs.map((r) => <RunItem key={r.runId} run={r} canStop={canStop} stop={stop} />)}</ul>;
}

function useNeedsYouFilter() {
  const [params, setParams] = useSearchParams();
  const on = params.get('filter') === 'needs-you';
  return { on, toggle: () => setParams(on ? {} : { filter: 'needs-you' }, { replace: true }) };
}

export function PodPage({ connect, http }: { connect: (podId: string) => Connect; http: Http }) {
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
  // The hello names members and their roles but not who owns an agent, so Stop shows to pod owners only; the route also lets an agent's owner stop it.
  const canStop = state.members.some((m) => m.actor.id === state.you?.id && m.role === 'owner');
  const runs = useMemo(() => Object.values(state.runs), [state.runs]);
  // One line per agent: an agent with a going run shows as its run line (with Stop), so the presence line leaves it out.
  const going = new Set(runs.filter((r) => r.state === 'going').map((r) => r.agent.id));
  const working = state.agents.filter((a) => a.state === 'working' && !going.has(a.agent.id)).map((a) => a.agent.displayName);
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
      <RunLines runs={runs} canStop={canStop} stop={(runId) => http.stopRun(id, runId)} />
      <Composer inputRef={composer} onPost={(t) => store.post(t)} error={state.postError} />
    </>
  );
}
