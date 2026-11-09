// A ruling on an ask (F12): a member chooses one option, once. The core checks the decision, freezes it, tells members, and
// wakes the asking agent through the one wake writer. An adapter only parses the frame and passes the facts.
import type { Message, OptionId, WakeRefs } from '../../gen/ts/protocol';
import { own, who } from './claims';
import { emit, fail, ok } from './emit';
import { isoOf } from './time';
import { seedFits, writeWake } from './wakes';
import type { Cmd, Ctx, PodState } from './types';

/**
 * Order: who is calling (a stranger is forbidden, a dead run unauthorized), a run is forbidden before any lookup, then the
 * decision (not_found), then ruled-already before the option, so a late ruling is told what stands rather than what it got wrong.
 */
/**
 * The feed is built from `message.created`, which still holds the ask open after a ruling. This puts the pod's current decision on
 * each ask, the same value `message.updated` carried, so a reader over HTTP or MCP sees which option won. Other messages pass through.
 */
export function overlayDecisions(s: PodState, messages: Message[]): Message[] {
  return messages.map((m) => {
    const rec = m.decision === undefined ? undefined : own(s.decisions ?? {}, m.decision.id);
    return rec === undefined || rec.messageId !== m.id ? m : { ...m, decision: rec.decision };
  });
}

export function ruleCmd(c: Ctx, cmd: Extract<Cmd, { type: 'rule' }>): void {
  const w = who(c, cmd);
  if (w === null) return;
  if (w.run !== undefined) return fail(c, 'forbidden', 'A run cannot rule; only a member may.');
  const rec = own(c.s.decisions ?? {}, cmd.decisionId);
  if (rec === undefined) return fail(c, 'not_found', 'No such decision.', cmd.decisionId);
  if (rec.decision.state === 'expired') return fail(c, 'gone', 'This decision expired before it was ruled; nothing it held was released.', cmd.decisionId);
  if (rec.decision.state !== 'open') return fail(c, 'already_ruled', 'This decision was already ruled; the first ruling stays.', cmd.decisionId);
  if (!rec.options.includes(cmd.option as OptionId)) return fail(c, 'invalid_input', 'That option is not one of this ask\'s options.');
  if (!seedFits(c, cmd.wakeId, 1)) return;
  rec.decision = { id: cmd.decisionId, state: 'ruled', ruling: { optionId: cmd.option as OptionId, by: w.actor, at: isoOf(c.now) } };
  rec.options = [];
  emit(c, 'message.updated', { id: rec.messageId, decision: rec.decision });
  const refs: WakeRefs = { decisionId: cmd.decisionId, messageId: rec.messageId, optionId: cmd.option as OptionId };
  if (rec.taskId !== undefined) refs.taskId = rec.taskId;
  writeWake(c, rec.askedBy, 'decision_ruled', refs, cmd.wakeId);
  ok(c);
}
