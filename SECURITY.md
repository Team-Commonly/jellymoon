# Security policy

## Reporting a vulnerability
Report it privately through GitHub: open the **Security** tab of this repository and choose **Report a vulnerability**.
Please do not open a public issue or pull request for it, and do not post details anywhere else first.

## Scope
This repository's code: the `jellymoon` binary and its core library, the app once it exists, and the scripts and CI that
build them. Of most interest: a session log whose text makes Jellymoon run something, add a tool or a flag to the
reading call, or print a credential it should have replaced. A flaw in a third-party dependency belongs with that
project; tell us too if Jellymoon's use of it makes the flaw reachable.

The code at the tag `harness-final` is no longer maintained.

## What to expect
We will acknowledge the report, work with you on a fix, and agree when it is safe to publish an advisory. Please keep
the details private until then.
