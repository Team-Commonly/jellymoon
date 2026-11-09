# Cap note 2026-11-09: daemon-clean-stop

2026-11-09: daemon cap 7,510 -> 7,620; a clean stop (Ctrl-C or SIGTERM) now closes the pod sockets without the reconnect line and logs "stopped", and two tests pin it: a stop logs no failure and one "stopped" line, and a socket that ends while the daemon runs still logs the reconnect line. The tree printed `LOC budget: daemon 7613 > 7510`; the cap is the count rounded up to ten. No other cap changes.
