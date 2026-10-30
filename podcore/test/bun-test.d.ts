// Just enough of bun:test for tsc; the runner itself ships with Bun, so no dependency is added.
declare module 'bun:test' {
  export const test: (name: string, fn: () => void) => void;
  export const describe: (name: string, fn: () => void) => void;
  export const expect: (value: unknown) => {
    toBe(v: unknown): void; toEqual(v: unknown): void; toBeNull(): void; toThrow(): void; toMatchObject(v: unknown): void; toBeGreaterThan(n: number): void;
    not: { toContain(v: unknown): void };
  };
}
declare module 'node:fs' {
  export function readdirSync(dir: URL, opts?: { recursive: boolean }): string[];
  export function readFileSync(file: URL, encoding: 'utf8'): string;
}
