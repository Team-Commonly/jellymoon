declare module '*.sql' {
  const text: string;
  export default text;
}

// workers-types has no DOM or Node lib; podcore/test/purity.test.ts reads import.meta.url (Bun sets it).
interface ImportMeta {
  readonly url: string;
}

// The bundle test uses Bun.build and the preview test Bun.spawnSync; this is the part of them that the tests touch.
declare const Bun: {
  spawnSync(cmd: string[], o?: { env?: Record<string, string> }): { exitCode: number; stderr: { toString(): string } };
  build(o: { entrypoints: string[]; target: string; minify?: boolean; loader?: Record<string, string> }): Promise<{ success: boolean; outputs: Array<{ text(): Promise<string> }> }>;
};
