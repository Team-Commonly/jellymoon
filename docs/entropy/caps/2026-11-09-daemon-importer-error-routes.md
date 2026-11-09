# Cap note 2026-11-09: daemon-importer-error-routes

2026-11-09: daemon cap 6,940 -> 6,970; the importer's HTTP errors now name the route shape (`/api/messages/:podId`) instead of the raw path, and the read call sites name the pod through `where()`, so a secret-shaped pod id is never echoed; one test covers a GitHub-shaped and a cm_-shaped id under 404 and 400. The tree printed `LOC budget: daemon 6965 > 6940`; the cap is the count rounded up to ten. No other cap changes.
