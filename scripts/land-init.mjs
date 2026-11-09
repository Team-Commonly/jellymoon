// land's local preflight child, moved out of land.mjs unchanged apart from taking IO and the token as arguments (land.mjs stays under the file-size cap).
import { quoteOutput, scrub } from './land-decide.mjs';
// Runs ./init.sh (which runs make preflight) in the throwaway worktree: env -i, stdin /dev/null, a NEW session (no
// controlling tty), output captured and quoted, a wall-clock limit, and the whole process group killed when it returns.
const LIMIT_MS = 30 * 60 * 1000;
export function runInit(IO, SECRET, wt, env) {
  return new Promise((done) => {
    let text = '';
    let over = false;
    let ended = false;
    const child = IO.spawn('env', ['-i', ...Object.entries(env).map(([k, v]) => `${k}=${v}`), 'bash', '-c', './init.sh'], { cwd: wt, detached: true, stdio: ['ignore', 'pipe', 'pipe'] });
    const take = (b) => { if (text.length < 2e6) text += b.toString('utf8'); };
    child.stdout.on('data', take);
    child.stderr.on('data', take);
    const kill = () => { try { IO.kill(-child.pid, 'SIGKILL'); } catch { /* the group is already gone */ } };
    const grace = IO.graceMs ?? 5000;
    let late;
    process.prependListener('exit', kill); // a crash or Ctrl-C of land must not leave the group running, and it dies before its dir is removed
    const timer = setTimeout(() => { over = true; kill(); late = setTimeout(() => end(-1), grace); }, IO.limitMs ?? LIMIT_MS); // an escapee holding the pipes cannot stall us
    const end = (code) => {
      if (ended) return;
      ended = true;
      clearTimeout(timer);
      clearTimeout(late);
      kill();
      process.off('exit', kill);
      if (text) console.error(scrub(quoteOutput(text), SECRET));
      if (over) return done({ ok: false, reason: `./init.sh did not finish in ${LIMIT_MS / 60000} minutes; killed` });
      return done(code === 0 ? { ok: true } : { ok: false, reason: 'clean-shell ./init.sh (baseline preflight) failed on the merge result' });
    };
    child.on('error', () => end(-1));
    // 'exit' is the child itself; 'close' waits for every holder of its pipes. Kill the group at exit so a descendant holding a pipe cannot stall land.
    child.on('exit', (code) => { kill(); late = setTimeout(() => end(code ?? -1), grace); });
    child.on('close', end);
  });
}
