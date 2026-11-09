# The browser specs

`web-e2e/` holds the Playwright specs of the web client. They prove the browser surface only: the API-level contract suite
(`make e2e`, [e2e-contract.md](e2e-contract.md)) stays the gate for the pod's rules. The card model, the store and the key map
have their own unit tests in `web/src/` (vitest, no browser).

| Spec | What it proves |
|---|---|
| `approve-keyboard.spec.ts` | An ask the fake agent posts while the page is open appears with no reload. Keys only: in the composer `a` and Enter are text and rule nothing; out of it, `j` focuses the ask, `a` opens the confirm row on the recommended option and Enter rules. The page shows the ruling only once the pod's `message.updated` reaches it, and the pod's log agrees |
| `needs-you.spec.ts` | With one ask addressed to you and one to someone else, `g n` shows exactly yours; `Needs you · 1` drops to 0 when you rule and "Nothing needs you" shows; a reload of `?filter=needs-you` keeps the filter on |
| `stop-run.spec.ts` | The fake agent's run is going when the page opens: the pod owner sees "Scout is working on a run." with Stop. Pressing Stop (a cookie POST to the run's stop route) disables the button and changes nothing on the line while the pod's frames are held back; once `run.ended` is let through the line says "Scout's run stopped." and the pod's own log has the outcome `cancelled`. A reload shows the stopped line again from the replay. Who is offered Stop (pod owners only; the hello does not name an agent's owner) is a unit test, because the bridge always carries the owner's token |
| `device.spec.ts` | A daemon's device code at `/device`: signed out the page shows the code and a GitHub link that returns to `/device?code=...`; signed in, loading approves nothing, Approve does, and the daemon's poll then gets a token for the approver (once). An unknown code says so |
| `invite.spec.ts` | A teammate with no install opens `/invite/:token` (signed out: the page names the pod and who invited them, a GitHub link, no Join), signs in, presses Join, lands on the pod and sees its history over the cookie socket; the one-use link then says it is spent for the next person |
| `create-pod.spec.ts` | A person opens `/`: signed out it offers a GitHub link and no form; signed in, Create stays disabled until there is a name, and creating "Launch room" lands on `/pod/:id`, where a message an agent posts reaches the page over the cookie socket |
| `layout.spec.ts` | Two invariants on every route and in the states a person reaches on it (`/` signed out, in, with a long name and with a refusal showing; `/device` signed out, in, with an error, approved; `/pod/:id` empty, with an unbroken headline, an open body, an ask, the confirm row and a long draft; the needs-you filter with an ask and with none; `/pod/:id` run lines: two going runs under a name as long as the schema allows, a refused Stop showing the server's long message, and two stopped lines; `/invite/:token` signed out, in and spent, and naming a pod and an inviter as long as the schema allows; the top bar under a very long pod id, and under the longest pod name the schema allows once hello has named it; `/pod/:id` refused: signed out, no such pod, and a pod you are not in, each showing its one line with no composer and no filter, and no socket retried once the store's retry timer is fast-forwarded past the refusal): the document is no wider than the viewport, and no text is clipped. Clipped means an element with text of its own, or an ancestor, hides overflow (hidden or clip) and the text runs past it; scrolling boxes do not clip, and a line that ellipsizes with a `title` is allowed. A first test plants a clipped box, a titled ellipsis and a 3000px element in a page, so the check is shown to catch what it should. The pod test also shows the card's status chip, receipt chip and folded body |

Specs sign the page in with the session cookie GitHub's callback would set (`signIn` in `fixtures.ts`), because GitHub is not reachable from a spec.

Each spec runs twice, as the projects `phone-390` (390x844) and `desktop-1440` (1440x900), in Chromium.

## Running them

- `make e2e-web` builds the client into `web/dist` (`bun run build:web`), then runs `playwright test -c web-e2e`. It needs Node 20
  or newer and the Chromium build the pinned `@playwright/test` expects.
- **Browsers are installed in CI only.** CI's `browser` job runs `playwright install --with-deps chromium` inside the job, then
  `make e2e-web`, and uploads `web-e2e/report/` and `web-e2e/test-results/` (traces of failed tests) when it fails. On a shared
  or small machine, don't download a browser to run these: run `bun x playwright test -c web-e2e --list` to check that the config
  and specs load (21 tests x 2 projects = 42), type-check them with `bun run check:types`, and let CI run them.
- On a machine that already has the pinned Chromium, `make e2e-web` runs them as CI does.

## The harness

- `global-setup.ts` starts one pod-state server (`bun server/src/main.ts`) with `ENV=local`, a fresh PGlite data dir under the
  temp dir, `PORT` from a free port, no `DATABASE_URL`, and `JM_WEB_DIR=web/dist`, so the server itself serves the built client
  as it does when deployed (ADR 0006). It waits on `GET /healthz` and fails at once if the server exits. Its teardown stops the
  server and removes the data dir. The specs find it through `JM_WEB_E2E_BASE`.
- `fixtures.ts` gives each test a fresh pod: an owner (`POST /__test/session`), a second member who joins by invite, and the fake
  agent. Nothing is shared between tests or projects, so a ruling in one never shows in another.
- `pod.ts` is the fake agent: a computer and an agent registered through the real routes. It gets a run token as the daemon
  does (a task assigned to the agent wakes it, it acks the wake, `POST /v1/runs`) and posts asks with it. `replay()` reads the
  pod's own log by subscribing from seq 0, which is how a spec checks what the API says, apart from what the page shows.
- **The socket bridge.** On main the pod socket takes its token only in `Authorization` on the upgrade, and a browser cannot set
  that header. So the fixture routes the page's socket (`page.routeWebSocket`) to a Node-side socket on the same URL with the
  owner's token, and relays frames both ways unchanged. The page's store, frames and rendering are the real ones. The bridge
  records every frame and can hold the server's frames back: that is how `approve-keyboard` shows the card still waiting while
  the pod has already ruled. When the browser session reaches the socket, the fixture can sign the page in by cookie instead,
  and the bridge's holding becomes the only reason to keep it.
- "No reload" is a mark set in the page (`window.__specDoc`) plus the bridge's socket count, never a count of `framenavigated`.
  The client's router stamps its history entry with `replaceState` on mount, a same-document navigation that fires
  `framenavigated` a second time for the same URL (the first CI run failed on exactly that).
- No fixed ports and no sleeps: the specs wait on the server's readiness, on frames the bridge saw and on the DOM
  (Playwright's retrying `expect`). `retries` is 0, so a flaky spec fails rather than passing on a second try.
- The selectors are the roles the components render: `article` per card, the `h3` headline, the `Options` and
  `Confirm ruling` groups, the `Message` textbox, the `Needs you · n` button and its `aria-pressed`, and the `status` region.
