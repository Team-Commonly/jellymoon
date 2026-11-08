// Stub: the tests are committed red first; the real key map lands in the next commit.
import type { OptionId } from '../../gen/ts/protocol';

export interface KeyCard { open: boolean; options: OptionId[]; recommended?: OptionId }
export interface KeyCtx { cards: KeyCard[]; inTextField: boolean }
export interface KeyState { focus: number; confirm: OptionId | null; g: boolean }
export type KeyAction =
  | { type: 'focus'; index: number }
  | { type: 'toggle-fold' }
  | { type: 'confirm'; optionId: OptionId }
  | { type: 'rule'; optionId: OptionId }
  | { type: 'cancel' }
  | { type: 'toggle-needs-you' }
  | { type: 'focus-composer' };

export const initialKeys: KeyState = { focus: 0, confirm: null, g: false };

export function handleKey(_s: KeyState, _key: string, _ctx: KeyCtx): { state: KeyState; action: KeyAction | null } {
  throw new Error('not implemented');
}

export function nextOpenAsk(_cards: KeyCard[], _from: number): number {
  throw new Error('not implemented');
}
