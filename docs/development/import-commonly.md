# Importing your Commonly pods: `jellymoon import commonly`

Copies your own Commonly pods and their messages into your Jellymoon, as new pods. It is a command-line tool only: no server route and no schema change. It reads Commonly with your own token and writes Jellymoon through the existing pod and message routes as you.

## Use

```
export COMMONLY_TOKEN=...            # or leave it unset and type it at a hidden prompt
jellymoon import commonly --instance https://commonly.example            # dry run: counts, writes nothing
export JELLYMOON_SERVER=...; jellymoon login
jellymoon import commonly --instance https://commonly.example --apply    # creates pods, posts messages
```

Options: `--include-dm <podId>` (repeatable) imports one 1:1 DM you name; `--anonymize` replaces other people's names with "participant A", "participant B", ...; `--state` and `--out` move the state file and the output directory (default `jellymoon-import`); `--pace-ms` sets the least gap between Commonly requests (260; values under 250 are refused, so a run cannot crawl the instance at full speed).

The Commonly token is read from `COMMONLY_TOKEN` (and dropped from the environment) or a hidden prompt. It is never an argument, never saved and never logged. The instance URL must be https, or http on localhost; a redirect is never followed.

## What it reads

Only these GET routes: `/api/auth/user`, `/api/pods`, `/api/pods/:id`, `/api/messages/:podId` (50 at a time, `before=`), `/api/grants?connectionOwner=me`, `/api/integrations/:podId`. It never sends a `scope` parameter, so an admin's wider view is never asked for; it refuses an admin or agent token outright before reading any pod. It checks that your id is in each pod's member list. It makes one request at a time, with jittered backoff on 429 and 5xx (honouring Retry-After, as seconds or an HTTP date), and stops after five failures in a row. A pod whose messages answer 401 or 403 (for example a membership revoked mid-run) is skipped and counted as `unauthorized`; the other pods still import.

## What it never copies

- 1:1 DMs (`agent-room`, `agent-dm`), unless you name one with `--include-dm`. `agent-admin` pods are always skipped.
- Agent memory, other people's emails or profiles, attachments' contents (a message gets `[attachment: <file name>]`), tokens, credentials, ingest tokens, grant tokens, integration bot tokens and webhook URLs. Grants and integrations are decoded into a fixed allowlist of fields, so any other field is dropped.
- Native author and time: every post is yours. Its body starts `[imported · <author> · <original date>]` and ends with a line `commonly-msg:<id>`, which lets a re-run find what is already there.

A scrubber checks every string bound for Jellymoon or the output files. One that looks like a token (Commonly `cm_` tokens including `cm_agent_` and `cm_daemon_`, even glued to other characters; Jellymoon tokens; bearer headers; API keys; private keys; webhook URLs) or equals your Commonly token fails the whole run: the reading happens first, so nothing has been written when it fails. The error names the Commonly pod id and message id so you can find the text, never the matched text. Placeholder text such as `cm_agent_...` also matches, and fails the run: edit or remove that message in Commonly first.

## Outputs, and what is lost

With `--apply`, two files go in `--out` (mode 0600): `agent-cards.json`, one identity card per agent seen (name, handle, avatar and the pods it was in; no role and no memory), and `grants-checklist.md`. Register an agent from a card with `jellymoon agent add <harness> --pod <id> --name <displayName>`. The checklist lists your grants; only Linear is supported, other vendors are listed as "unsupported" and integrations as "to reconnect".

Lossy by design: edit history, threads, reactions, attachments, agent memory, and native timestamps and authors. A dry run writes no file at all. The local state file holds ids only and lets `--apply` be run again: it posts nothing twice and resumes a killed run from its per-pod cursor.

## Limits to know

- **`--anonymize` renames authors only.** Names and @mentions inside message text are copied as written.
- **A crash between creating a Jellymoon pod and saving the state leaves an empty pod.** The next run creates another pod and posts the messages there once, so no message is ever posted twice, but the empty one stays; delete it by hand.
- **Deleting the state file re-imports everything** into new pods, because the state is what maps a Commonly pod to its Jellymoon pod.
