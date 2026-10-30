declare module '*.sql' {
  const text: string;
  export default text;
}

// workers-types has no DOM or Node lib; podcore/test/purity.test.ts reads import.meta.url (Bun sets it).
interface ImportMeta {
  readonly url: string;
}

// The bundle test uses Bun.build; this is the part of it that the test touches.
declare const Bun: { build(o: { entrypoints: string[]; target: string; minify?: boolean; loader?: Record<string, string> }): Promise<{ success: boolean; outputs: Array<{ text(): Promise<string> }> }> };
