# 0018 · A run asks, and is told more

Status: Accepted, by the coordinator's decision, 2026-11-12, on the owner's direction of that day
([ADR 0017](0017-a-runs-work-is-shown-kept-or-tossed.md), "Why"). It changes two sentences of
[ADR 0014](0014-a-run-is-started-headless.md), decision 2, and adds to ADR 0017.

## Why

Step 3 of the loop is "it asks in plain words when it must, and the person answers". A run today is one shot: the
wish goes in, and the harness works until it ends. An agent that lacks one fact guesses, and the person learns of
the guess when they look at the work. And a person who likes what was made can only wish again from nothing.

Two things are decided here: how a run asks and is answered, and how finished work is followed with more words
("more like this").

## Decisions

1. **A question ends the harness, and an answer starts it again.** No process waits while the person thinks. A run
   that must ask ends its turn; the person answers whenever they like; a new process of the harness resumes the
   run's own session, in the same copy, with the answer on its input. One mechanism carries both an answer and a
   follow-up: the harness's kept session, resumed by its id. Nothing is held open, so the app can be closed and the
   machine restarted between a question and its answer.
2. **How a turn ended is data.** The harness is started with a schema for its last word: `state`, which is `done`
   or `asks`, and with `asks` one `question`. Core reads that from the stream's result and nothing from the
   agent's prose. A result that is a success, with exit 0 and `done`, is a finished run, as now. One with `asks`
   and a question that passes is a run that asks. Anything else is a failed run, as now.
3. **A question is an agent's text.** It is cleaned at the door every kept text is cleaned at, kept on one line
   and cut at a stated length, drawn as text, and never run. It is shown as the little one's words, apart from
   Jellymoon's own, which stay fixed. A question that cleaning empties or goes on changing is a failed run.
4. **A run that asks has not ended.** Its record says it asks, with the question; its lock is let go; its copy
   stays as the agent left it, with nothing committed; the wish needs the person and carries the question. It is
   still the wish's one run: an answer goes on with the same run, copy, branch and session. A look takes a run
   that asks for neither one going nor one cut off.
5. **An answer is the person's typed words.** `jellymoon run answer --wish <id>` takes them on its input, as
   `jellymoon wish` takes a wish, and they pass the same door: credential-shaped text is replaced before it leaves
   the process, so an answer to "paste your key" does not carry the key. It is refused for a run that does not
   ask. It waits and prints as `jellymoon run` does. A run may ask again; nothing counts the questions, since each
   costs the person an answer and they can stop or toss instead.
6. **A run's session is kept, in the harness's own store.** ADR 0014 said a run's session is not kept. It is kept
   now, under an id core chooses before the harness starts (`--session-id`) and writes in the run's record. It
   lies where the person's own sessions lie and ages out as they do (thirty days unless they set otherwise).
   Jellymoon never removes anything from that store. A run whose session is gone when the answer comes fails in
   fixed words, and its copy is kept.
7. **The reader never reads a run back.** A kept session holds the wish and every answer as lines the reader would
   take for typed turns (measured below). So the reader leaves out every session whose working directory is a
   run's copy, as it leaves out a folder the person switched off. What a person types into a harness they start by
   hand inside a kept copy is left out with it: a copy is the run's place.
8. **A follow-up is a new made wish that names the wish it follows.** It has its own one run, copy, branch and
   record. Its copy is made at the followed work's commit when that work waits, and where the person's checkout
   stands when it was kept. Its session is a fork of the followed run's, so the agent knows what was asked and
   made; with that session gone it starts a new one, and the record says which. Tossed work cannot be followed.
9. **Followed work and its follow-up are kept in order.** `run keep` of a follow-up refuses while the work it
   follows still waits, and names the wish to keep first. `run toss` of work that a waiting follow-up follows
   refuses and names the follow-up. So no keep ever lands work the person did not see or tossed.
10. **Nothing is asked or answered while a process goes.** That sentence of ADR 0014 stands. The list of what a
    run may do is fixed at each start and passed again at each resume; no permission is asked, and no hook, MCP
    server or permission tool is given to the harness.

## Not chosen

- **Deferring a tool call from a hook** (`permissionDecision: "defer"`, then `--resume`). It is the harness's own
  documented way to ask, but it is honoured only when the agent makes a single tool call in the turn, the asking
  tool is offered only to a run that has a permission host, and a host means an MCP tool and a hook passed to
  every run. Decision 10 would go.
- **One process kept alive, spoken to on its input** (`--input-format stream-json`). The process must outlive the
  person's thinking and the app; the messages a bare CLI takes on its input are not documented outside the SDK.
- **The Agent SDK, or ACP through an adapter.** Each is a Node or Python package beside the harness. That fails
  the install bar of ADR 0014, decision 1. ACP stays the plan for a later starter (ADR 0017, "What comes after").

## What changes in earlier notes

| Note | It said | It says |
|---|---|---|
| ADR 0014, decision 2 | "a run's session is not kept, so the next day's reader never reads a run back" | A run's session is kept in the harness's store; the reader leaves out a run's copy (decisions 6 and 7 here) |
| ADR 0014, "The command line" | `--no-session-persistence` | `--session-id <id>` at a start, `--resume <id>` at an answer, and a schema for the last word |
| ADR 0017, "What comes after", 1 | to be decided | this note |

## Measured

The coordinator measured these on Claude Code 2.1.323, on a Mac, in a scratch repo with a made-up task, with the
run's command line less `--no-session-persistence` and with `--json-schema` and `--session-id` added.

- **A run that asks.** Given a wish that cannot be done without one fact, and fixed words that say to change no
  file and ask, the result was a success with `structured_output` of `{"state":"asks","question":"…"}`. No file
  was changed. The stream's init line lists one more tool, `StructuredOutput`, which the check of that list must
  admit. The session file was named for the id that was passed.
- **An answer.** A second process with `--resume <id>`, the same flags and one word on its input edited the file
  and ended `{"state":"done"}`, under the same session id, in the same session file.
- **A follow-up after the copy is gone.** The first worktree was committed and removed, a second was made at that
  commit, and a third process there with `--resume <id> --fork-session` did the follow-up in the second
  worktree. It got a new session id and its own file; the first session's file was not changed.
- **A session that is not kept.** `--resume` of an id nothing has: exit 1, a result with
  `subtype: error_during_execution`, and `No conversation found with session ID` on stderr and in the result.
- **The reader would read a run back.** In both session files the wish, the answer and the follow-up are plain
  user lines that the reader's rule keeps, each with the copy as its working directory. The fork's file repeats
  the first session's lines under the second copy's folder.

The coordinator measured three more on 2026-11-12, on the same Claude Code, in a scratch repo, while F51 was
built:

- **An agent that edits and then asks.** It edited a file and ended `asks`, with a success and exit 0. So the copy
  of a run that asks may hold changes; they stay in it, uncommitted (decision 4).
- **`--session-id` with a fork.** `--session-id <uuid>` works with `--resume <id> --fork-session`: the fork's id,
  in the stream and in its file's name, was the id asked for. A follow-up's id can be core's too.
- **Every line names its folder.** Every `user` and `assistant` line of both kept session files carried a working
  directory.

The coordinator tried the whole of F51 once on the real harness, in a scratch repo, with a scratch home and
empty logs:

- **A wish that can be done** ended `completed` with one commit.
- **A wish that needs the person** ended `asks` with one plain question of 217 characters. The copy was kept
  and nothing was committed.
- **A toss of the run that asked.** `run toss` without `--repo` removed the copy, let the wish go and left the
  branch; with `--repo` it deleted the branch.
- **The resume was not yet measured** with core's own command line.

## Found while building F51

- **The last word is read strictly.** Exactly `{"state":"done"}` or `{"state":"asks","question":<text>}`. `done`
  with a question, `asks` with anything more, and every other value is a failed run: a harness that enforces the
  schema never sends them, and one that does not is not believed.
- **The stated length is 300 characters**, cut with `…`; a question of more than 4096 bytes, the most the door
  looks at, is a failed run and is not cut.
- **The session is checked.** An init line that names another session than the one given, or none, stops the
  child and fails the run, in its own words. The record a run that asks is believed by names a session id of
  the shape core makes. Only the shape is checked: whoever can write the home can write a record by hand that
  names another session of that shape, and an answer would go on with that one.
- **The tool list is exact.** With a schema the init line lists `StructuredOutput` beside the five, once.
- **A copy that asks is never marked failed**, so it is not one of the five kept and nothing prunes it. Only a
  toss removes it until F52.
- **`jellymoon run` prints the question**, as a seventh name, cleaned. It is the first text of an agent's that a
  run prints.
- **A harness too old is told by any of three options**, `--restricted`, `--json-schema` and `--session-id`, and
  the line names the one it did not know.
- **A toss holds the run's lock** for a run that asks, since nothing else holds such a run and F52's answer will
  take the same lock.

## Found while building F52

- **An answer is 600 characters at most, and never cut.** A question is cut because it is an agent's; an answer
  is the person's, and cut words would not be theirs.
- **The answer is kept with the wish**, as `answer`, and nowhere else Jellymoon writes: not in the run's record,
  which other commands print from. Only the last answer is kept.
- **The command line is a run's with one word changed**: `--resume` where `--session-id` stood. The schema, the
  list of what a run may do and the fixed prompt are passed again (decision 10).
- **An answer holds three locks in one order**: the keeps', the one runs are counted behind, the run's own. So an
  answered run is counted among the three, of two answers one goes on, and a toss and an answer never meet.
- **The wish is told before the record.** An answer killed between the two leaves a record that still asks, and
  a look gives the question back.
- **A harness that cannot take the answer leaves the run asking.** Too old for `--resume`, or not startable:
  the record and the wish are put back, and the copy is not marked failed.
- **A session that is not kept is told by the harness's own words**, `No conversation found with session ID`, in
  an error result or on stderr, with a failing exit, and only in an answered run. The run fails in fixed words.
  A stream that did not start as a run's counts as a failing exit: its harness is ended at once, so its own
  exit is chance (CI on Windows found it still going, where macOS and Linux had found it gone).
- **A stop left before the question is not a stop of the answered run.** The answer removes it.
- **The door reads across a line break.** A credential typed on two lines was two words that each passed. The
  door now also reads a stretch with the break taken out, and keeps the stretch as `[redacted]` when that
  reading cleans differently. It holds for every text the door keeps.
- **A last word that names a part twice is a failed run**, whatever the two values are.
- **The gate failed F52 on the door, and the fix is one commit after it.** The reading across a break had taken
  the place of the older reading, and kept a private key's body when a long word stood on the line before its
  first line. Both readings are now made and a word is kept only where both keep it: nothing the older reading
  replaced is kept. Words of which only marks are left are refused wherever something is made through the door.
- **The run's lock is a file that stays**, as the keeps' lock and the one runs are counted behind do. Removed
  when it was let go of, it could be held under no name by a command that had opened it just before.
- **The harness's words for a lost session are read in a result's `errors` and on stderr**, not in the whole
  line: an agent that writes them in its own prose has not lost its session.
- **The gate failed that fix on the door once more.** A stretch was read joined whole or not at all, so a word
  that touched a split key id across one more break (`the id is`, a break, half the id, a break, the rest)
  took the word's edge the id is found by, and both halves were kept and given to the harness. A stretch is
  now also read with every two to eight lines in a row joined and the lines around them apart. What is left
  open is in wish.md, "Not closed": a key over more than eight lines with a word touching it, and a private
  key's first line split inside a word.
- **A refused answer may leave the run's own lock file**, empty, where an older build had taken it away: the
  answer holds the run before it reads anything again. run.md said only `runs/keep.lock` was written.
- **Not measured in the build:** the resume itself on a real harness with core's command line. The coordinator
  measured it afterwards, below.

The coordinator tried F52 on the real harness (Claude Code 2.1.325), in a scratch repo, with a scratch home and
empty logs, on a wish that cannot be done without one answer:

- **The answer went on with the run.** The run ended `asks`; `run answer` with the answer on its input ended the
  same run `completed`, with one commit on its branch that holds the answer's edit. The record names the same
  session before and after, and the harness's store holds one session file for the run's copy, not two. So
  `--resume` took the schema, the list and the prompt again, and the init line named the session core gave.
- **A session that is not kept.** With the run's session file removed from the store before the answer, the run
  failed in the fixed words (`Claude Code no longer keeps the session it asked in`), and its copy was kept.
- **Still not measured:** a harness too old to know `--resume`.

## Read, not measured

- Claude Code's hooks page, read 2026-11-12, for "Not chosen": `defer` "requires non-interactive mode", is ignored
  when there are "several tool calls at once", and a `-p` run "offers `AskUserQuestion` only when it has a
  permission host".
- Claude Code's legal page, read the same day: a developer may not "offer Claude.ai login into their own
  applications" or "collect, store, or intermediate Claude.ai credentials or session tokens", and that does not
  "prevent an end user from signing in to the unmodified Claude Code binary with their own Claude subscription".
  Jellymoon starts the person's own installed harness and holds no login. Whether the page's terms for
  "preinstalling or running Claude Code in your products" reach an app that only starts the copy a person
  installed themselves is the owner's to settle, and is not settled here.

## Not measured

- A session resumed after the harness was updated, on Windows, or on a Claude Code too old for `--json-schema` or
  `--session-id`. Such a harness is expected to answer as one too old for `--restricted` does, and is told the
  same way (`run.md`, "A run that was never started").
- An agent that returns `asks` with no question, one whose last word does not fit the schema, and one that
  reaches a turn limit. The harness is said to retry such a word and to end with a subtype of its own. Core
  takes each for a failed run whatever the harness does (`run.md`, "A run that asks"), proved on the made-up
  agent only.
- The fixed words that tell the agent to end with `done` or `asks`, as F51 wrote them, on a real harness.
- Two answers at once. The run's lock decides it, as it decides two starts.
- How a kept session's size grows over many answers, and what a resume costs.

## The slices

`feature_list.json` gets these as F50 to F54. F53 waits on F45 and F46, and F54 on F47, which is stated here since
those rows are written already.

| Feature | What it adds | Waits on | Changes what leaves the machine |
|---|---|---|---|
| F50 | The reader leaves out a session whose working directory is a run's copy | nothing | yes: the reader |
| F51 | A run's session is kept under core's id, its last word is data, and a run can ask | F48, F50 | yes |
| F52 | `jellymoon run answer`: the person's words resume the run | F51 | yes |
| F53 | A follow-up: `jellymoon wish --after <id>`, a copy at the followed work, a forked session | F45, F46, F52 | yes |
| F54 | The entry shows a question with a field to answer in, and `More like this` beside shown work | F47, F52 | no |

## What this does not do

- It shows nothing of a run while it goes. Watching is the page's little one and its fixed words, as now.
- It gives a run no new tool. A run still reads and edits files in its copy and runs nothing.
- It does not let an agent ask for a permission. A question is words, and so is its answer.
- It does not carry a run's session to another machine or another harness.
