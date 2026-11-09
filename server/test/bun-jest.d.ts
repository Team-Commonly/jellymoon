// The fake clock of bun:test, which the shared stub in podcore/test does not declare.
declare module 'bun:test' {
  export const jest: { useFakeTimers(): void; useRealTimers(): void; advanceTimersByTime(ms: number): void };
}
