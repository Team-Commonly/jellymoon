// One card in the stream. It only renders a Card from the card model and reports what the person did; the screen
// owns the keyboard state, so a click and a key go through the same confirm row.
import { useState } from 'react';
import type { OptionId } from '../../gen/ts/protocol';
import type { Card } from './cards';

type Ask = NonNullable<Card['ask']>;

interface CardActions {
  toggleFold(): void;
  pick(optionId: OptionId): void;
  rule(): void;
  cancel(): void;
}

const List = ({ title, items, empty }: { title: string; items: string[]; empty: string }) => (
  <div>
    <h4>{title}</h4>
    {items.length ? <ul>{items.map((t) => <li key={t}>{t}</li>)}</ul> : <p>{empty}</p>}
  </div>
);

function Context({ ask }: { ask: Ask }) {
  return (
    <section aria-label="Context">
      <List title="What happened" items={ask.happened} empty="Nothing was reported" />
      <List title="What was checked" items={ask.checked} empty="Nothing was checked" />
      <p><strong>At stake:</strong> <span>{ask.stakes}</span></p>
      <p><strong>Recommended:</strong> {`option ${ask.recommendation.n}: `}<span>{ask.recommendation.why}</span></p>
    </section>
  );
}

function Options({ ask, act }: { ask: Ask; act: CardActions }) {
  const [hint, setHint] = useState<OptionId | null>(null);
  const shown = ask.options.find((o) => o.id === hint);
  return (
    <div>
      <div role="group" aria-label="Options">
        {ask.options.map((o) => (
          <button
            key={o.id}
            type="button"
            data-recommended={o.recommended}
            onClick={() => act.pick(o.id)}
            onFocus={() => setHint(o.id)}
            onBlur={() => setHint(null)}
            onMouseEnter={() => setHint(o.id)}
            onMouseLeave={() => setHint(null)}
          >
            {o.n} {o.label}
            {o.recommended && ' Recommended'}
          </button>
        ))}
      </div>
      {shown && <p>{shown.consequence}</p>}
    </div>
  );
}

function Confirm({ ask, optionId, act }: { ask: Ask; optionId: OptionId; act: CardActions }) {
  const label = ask.options.find((o) => o.id === optionId)?.label;
  return (
    <div role="group" aria-label="Confirm ruling">
      <span>Rule {label}? </span>
      <button type="button" onClick={act.rule}>Rule</button>
      <button type="button" onClick={act.cancel}>Cancel</button>
    </div>
  );
}

function AskBody(p: { ask: Ask; confirm: OptionId | null; act: CardActions }) {
  const { ask, confirm, act } = p;
  return (
    <>
      <Context ask={ask} />
      {ask.state === 'open' && <Options ask={ask} act={act} />}
      {ask.state === 'open' && confirm && <Confirm ask={ask} optionId={confirm} act={act} />}
      {ask.state === 'open' && ask.pending && !confirm && <p>Sending your ruling, waiting for the pod.</p>}
      {ask.ruling && <p>{`Ruled: ${ask.ruling.label} by ${ask.ruling.by}`} <time dateTime={ask.ruling.at}>{ask.ruling.at}</time></p>}
      {ask.state === 'expired' && <p>Expired without a ruling.</p>}
      {ask.error && <p role="alert">{ask.error}</p>}
    </>
  );
}

export interface CardViewProps {
  card: Card;
  open: boolean;
  current: boolean;
  confirm: OptionId | null;
  act: CardActions;
  onFocus(): void;
  elRef(el: HTMLElement | null): void;
}

export function CardView({ card, open, current, confirm, act, onFocus, elRef }: CardViewProps) {
  return (
    <article ref={elRef} tabIndex={0} onFocus={onFocus} data-kind={card.kind} data-quiet={card.quiet} data-current={current} data-card-id={card.id}>
      <div onClick={act.toggleFold}>
        <span>{card.author}</span>
        <h3>{card.headline}</h3>
        <span data-chip="status">{card.status}</span>
        {card.receipt && <span data-chip="receipt">{card.receipt}</span>}
      </div>
      {open && card.body && <pre>{card.body}</pre>}
      {card.ask && <AskBody ask={card.ask} confirm={current ? confirm : null} act={act} />}
    </article>
  );
}
