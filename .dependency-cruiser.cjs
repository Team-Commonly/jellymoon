// Module boundaries for the TS side (docs/development/workflow.md). Go boundaries live in entropy.json.
module.exports = {
  forbidden: [
    { name: 'no-circular', severity: 'error', from: {}, to: { circular: true } },
    { name: 'gen-is-a-leaf', severity: 'error', from: { path: '^gen/' }, to: { pathNot: '^gen/' } },
    { name: 'nothing-imports-e2e', severity: 'error', from: { pathNot: '^e2e/' }, to: { path: '^e2e/' } },
    { name: 'ee-stays-in-ee', severity: 'error', comment: 'ADR 0004: core never imports ee/', from: { pathNot: '^ee/' }, to: { path: '^ee/' } },
    { name: 'podcore-src-is-closed', severity: 'error', comment: 'ADR 0003: the pod core imports only itself and gen/, nothing from node_modules either', from: { path: '^podcore/src/' }, to: { pathNot: '^(podcore|gen)/' } },
    { name: 'podcore-is-pure', severity: 'error', comment: 'ADR 0003: nothing under podcore/, tests included, reaches into the adapter or any other part of the repo; tests may use Node built-ins, bun:test and node_modules', from: { path: '^podcore/' }, to: { pathNot: ['^(podcore|gen)/', '^node_modules/', '^bun:'], dependencyTypesNot: ['core'] } },
    { name: 'worker-uses-podcore-only-via-index', severity: 'error', comment: 'ADR 0003: the adapter reaches the core through its public index, never its internals', from: { path: '^worker/' }, to: { path: '^podcore/', pathNot: '^podcore/src/index\\.ts$' } },
    { name: 'no-test-routes-from-prod', severity: 'error', from: { path: '^worker/src/', pathNot: '^worker/src/test/' }, to: { path: '^worker/src/test/' } }
  ],
  options: { doNotFollow: { path: 'node_modules' }, tsPreCompilationDeps: true }
};
