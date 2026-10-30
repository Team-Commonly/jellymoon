# .loop/

The durable process files of the build loop: the brief template, the standing gates on briefs, and the standing orders.
Each rule keeps the defect that earned it, and none depends on a private record.
The loop reads these files before it writes or reviews a brief. `scripts/check-private.mjs` keeps private names, paths and ids out.
