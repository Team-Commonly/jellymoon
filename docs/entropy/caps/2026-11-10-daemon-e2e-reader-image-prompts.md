# Cap note 2026-11-10: daemon-e2e-reader-image-prompts

2026-11-10: daemon cap 8,500 -> 8,510 and e2e cap 10,490 -> 10,510; F26 follow-up (ADR 0011 section 1): the reader keeps the text parts of a prompt typed with a pasted image, and `f26_test.go` gains the image, tool-result and old-file cases. The tree printed `LOC budget: daemon 8506 > 8500` and `LOC budget: e2e 10504 > 10490`; each cap is the count rounded up to ten. No other cap changes.
