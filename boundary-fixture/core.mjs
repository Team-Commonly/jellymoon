// Fixture for scripts/check-boundary-rule.mjs: a module outside ee/ importing ee/ must violate ee-stays-in-ee (ADR 0004).
import { leaf } from "./ee/leaf.mjs";

export const core = leaf;
