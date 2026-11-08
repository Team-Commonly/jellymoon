// The card model: a pure function from a stored message (plus what the view knows) to what a card shows,
// and the needs-you selector. A ruling shows only from the message's own decision, never from a pending write.
import type { Actor, AskContext, Member, Message, MessageKind, MessageStatus, Option, OptionId } from '../../gen/ts/protocol';

export interface CardView { pending?: OptionId; error?: string }
interface CardOption { n: number; id: OptionId; label: string; consequence: string; recommended: boolean }
interface AskCard {
  state: 'open' | 'ruled' | 'expired';
  happened: string[];
  checked: string[];
  nothingChecked: boolean;
  stakes: string;
  recommendation: { n: number; why: string };
  options: CardOption[];
  ruling?: { n: number; optionId: OptionId; label: string; by: string; at: string };
  pending?: number;
  error?: string;
}
export interface Card {
  id: string;
  kind: MessageKind;
  quiet: boolean;
  author: string;
  headline: string;
  status: MessageStatus;
  receipt?: string;
  body?: string;
  ask?: AskCard;
}

const numberOf = (options: Option[], id: OptionId) => options.findIndex((o) => o.id === id) + 1;

function askCard(m: Message, ctx: AskContext, view: CardView): AskCard {
  const options = m.options ?? [];
  const rec = ctx.recommendation.option;
  const ruling = m.decision?.ruling;
  return {
    state: m.decision?.state ?? 'open',
    happened: ctx.happened.map((c) => c.text),
    checked: ctx.checked.map((c) => c.text),
    nothingChecked: ctx.checked.length === 0,
    stakes: ctx.stakes.text,
    recommendation: { n: numberOf(options, rec), why: ctx.recommendation.why.text },
    options: options.map((o, i) => ({ n: i + 1, id: o.id, label: o.label, consequence: o.consequence, recommended: o.id === rec })),
    ...(ruling && {
      ruling: { n: numberOf(options, ruling.optionId), optionId: ruling.optionId, label: options.find((o) => o.id === ruling.optionId)?.label ?? '', by: ruling.by.displayName, at: ruling.at },
    }),
    ...(view.pending && !ruling && { pending: numberOf(options, view.pending) }),
    ...(view.error && { error: view.error }),
  };
}

export function buildCard(m: Message, view: CardView): Card {
  const r = m.receipt;
  return {
    id: m.id,
    kind: m.kind,
    quiet: m.kind === 'progress' || m.kind === 'fyi',
    author: m.author.displayName,
    headline: m.summary,
    status: m.status,
    ...(r && { receipt: `${r.harness} · ${r.toolCalls} tool calls` }),
    ...(m.body && { body: m.body }),
    ...(m.kind === 'ask' && m.context && { ask: askCard(m, m.context, view) }),
  };
}

/** Open asks whose needs.to names you by userId or by your role in the pod. */
export function needsYou(messages: Message[], you: Actor, members: Member[]): Message[] {
  const role = members.find((x) => x.actor.id === you.id)?.role;
  return messages.filter((m) => {
    if (m.kind !== 'ask' || m.decision?.state !== 'open') return false;
    const to = m.needs?.to;
    return !!to && (to.userIds?.includes(you.id) === true || (!!role && to.role === role));
  });
}
