# Glossary

The words below mean exactly this in code, docs and tests. A new term is added here in the same PR that introduces it.

| Term | Meaning | Not to be confused with |
|---|---|---|
| **page** | One local day of one person's wishes, read from their agent sessions and shown by the app (ADR 0011) | a web page |
| **wish** | Something a person wants to exist or be true, worked out from what they typed to their agents; carries their own words and a state: working, done, needs you or let go | an ask, which needs a ruling |
| **little one** | The character that stands for one wish on a page | an agent |
| **star** | One finished job an agent reported for a wish | proof the job was done; it is the model's reading |
| **clash** | Two things one person said that cannot both stand. Shown as an ask; the person's tap is the ruling | a merge conflict |
| **ask** | A question put to the person with fixed options and an open decision | a question in prose |
| **ruling** | The person's choice on an ask, made by a tap and handed to each running session once. A tap the other way is a new ruling that replaces it | approval in chat |
| **settlement** | The file beside a day's kept page that says which statement of each clash stands, so the page draws it settled. No ruling is made from it | the ruling |
| **hook** | The one entry Jellymoon adds to the harness settings; the harness runs it at each step of a session, and it hands over the rulings that wait (`jellymoon hook`) | a git hook |
| **harness** | The agent program a person already uses: `claude-code` or `codex` | Jellymoon, which only reads its logs and calls it |
| **session log** | The file a harness writes for one session. Private, and untrusted input: it can hold any text an agent ever printed | page data, which code has checked |
| **turn** | One thing a person typed, with the agent's last reply to it | a line of a session log |
| **reader** | The part of core that collects one day of what the person typed from session logs (`jellymoon day`) | the reading call, which is the model's part |
| **reading call** | The one headless model call, through the person's own harness with every tool off, that proposes a day's wishes and clashes (`jellymoon page`) | the reader, which is code; a run |
| **page data** | What a page is drawn from: the day's wishes, stars and clashes after code has checked them. A typed shape in core | the reading call's raw reply |
| **core** | `crates/jellymoon-core`: every rule of the journal, with no UI | the `jellymoon` binary, which is core behind commands |
| **shell** | The app's window (`app/`): it draws page data with the kit and decides nothing | core |
| **kit** | The fixed set of drawable pieces a page is arranged from (`crates/sticker-kit`). The model picks pieces; it never draws one | generated UI |
| **theme** | How a page looks: colours, fonts, paper, the art for each piece and how much things move. Data and art only, never code | the kit, which is the pieces and their rules |
| **pack** | Extra pieces added to the kit as data and art only | a plugin, which would run code |
| **made wish** | A wish the person made in the app or with `jellymoon wish`, kept in their exact words beside the day's page and drawn with the reading's wishes | a wish the reading call worked out; a ruling |
| **run** | One agent session Jellymoon starts itself for a made wish: the person's harness, headless, in its own copy of the repo (ADR 0011, section 7; ADR 0014) | a session the person started, which Jellymoon only reads; the reading call, which has every tool off |
| **copy** | A run's own working copy of the person's repo: a git worktree on a branch of its own. The folder is removed when the run ends; the branch, with the work on it, is kept | the person's checkout, which a run never touches |
| **feature** | One entry of `feature_list.json`: a test at the binary's boundary that defines done | an issue |
| **unit** | One piece of work given to one worker: usually one feature, and one file in `progress/` | a feature, though often one-to-one |
