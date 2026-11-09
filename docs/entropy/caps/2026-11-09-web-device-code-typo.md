# Cap note 2026-11-09: web-device-code-typo

2026-11-09: web cap 2,490 -> 2,550; `/device` now tells a code outside the alphabet from an expired one: `ApiError` carries the server's error code, `approveDevice` reads it, and the page checks the shape against the generated schema before sending, with tests in `web/src/DeviceScreen.test.tsx` and `web/src/http.test.ts` and one browser spec in `web-e2e/device.spec.ts`. Merged with main at 6831240, the tree printed `LOC budget: web 2542 > 2490`; the cap is the count rounded up to ten. No other cap changes.
