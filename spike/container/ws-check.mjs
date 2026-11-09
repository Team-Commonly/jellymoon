// Sends one message over a WebSocket and expects it echoed back. Prints `SPIKE ws: ok|fail <detail>`; exits 1 on fail.
// Usage: node ws-check.mjs <ws-or-wss-url>. Uses Node's global WebSocket (Node 22+).
const url = process.argv[2];
const sent = `spike-${Date.now()}`;
const t0 = Date.now();
const done = (ok, detail) => {
  console.log(`SPIKE ws: ${ok ? "ok" : "fail"} ${detail}`);
  process.exit(ok ? 0 : 1);
};
const timer = setTimeout(() => done(false, "timeout after 20s"), 20_000);
const ws = new WebSocket(url);
ws.addEventListener("open", () => ws.send(sent));
ws.addEventListener("message", (ev) => {
  clearTimeout(timer);
  const got = String(ev.data);
  ws.close();
  done(got === sent, got === sent ? `echo matched in ${Date.now() - t0} ms` : `echo mismatch: ${got.slice(0, 80)}`);
});
ws.addEventListener("error", (ev) => {
  clearTimeout(timer);
  done(false, `error: ${ev.message ?? ev.error?.message ?? "websocket error"}`);
});
ws.addEventListener("close", (ev) => {
  clearTimeout(timer);
  done(false, `closed before echo: code ${ev.code} ${ev.reason}`);
});
