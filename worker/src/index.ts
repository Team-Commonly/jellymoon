// The production entry: the public routes and the pod object, and nothing else. It does not import src/test/, and
// worker/test/bundle.test.ts bundles this file and fails if the result contains a test route.
import { handle } from './front';

export { PodDO } from './pod-do';
export default { fetch: handle };
