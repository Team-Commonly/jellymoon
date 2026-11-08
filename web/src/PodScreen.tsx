// Red stage: the /pod/:id screen is not built yet.
import type { Connect } from './store';

export function PodPage(_props: { connect: (podId: string) => Connect }): never {
  throw new Error('PodPage is not built');
}
