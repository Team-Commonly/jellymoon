# Cap note 2026-11-09: device-code-limit

2026-11-09: worker cap 2,780 -> 2,880, server 4,660 -> 4,780, web 3,260 -> 3,280 and e2e 10,220 -> 10,300. The guess limit
on approve and deny (ADR 0008 step 4, "Guess limit") adds the limit to both adapters (the server's fixed-window counter, the
Worker's take and give on `rate_limits`), four unit tests on the Worker and three on the server in place of each one's old
no-limit test (the 11th failure on both routes, successes not counting, a valid code past the limit and for another person,
the window's reset, accounts sharing the address limit, and on the Worker the count outliving the start sweep), the /device
page's 429 line with its two unit tests, and two TestDENY steps in place of the old no-limit step, both adapters. The tree
printed `LOC budget: worker 2877 > 2780`, `server 4773 > 4660`, `web 3272 > 3260` and `e2e 10295 > 10220` (measured after merging main
at 18735e4, with #101 and #103, and this change committed); each cap is the count rounded up to ten. No other cap changes.
