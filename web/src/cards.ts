// Stub: the tests are committed red first; the real reducer lands in the next commit.
import type { Actor, Member, Message, MessageKind, MessageStatus, OptionId } from '../../gen/ts/protocol';

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

export function buildCard(_m: Message, _view: CardView): Card {
  throw new Error('not implemented');
}

export function needsYou(_messages: Message[], _you: Actor, _members: Member[]): Message[] {
  throw new Error('not implemented');
}
