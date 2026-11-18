# Release apple first

Not gated; no row of `feature_list.json` changes.

## Built
- `.github/workflows/release.yml`: the build matrix is two jobs, `build-arm64` and `build-x86_64`, with one list of
  steps (a YAML anchor). `release` needs only `build-arm64`: it sums the Apple-chip archive into a one-line
  `SHA256SUMS`, creates a draft with both files (`--verify-tag`, a dash makes a pre-release) and publishes it.
  `release-intel` needs `release` and `build-x86_64`: it downloads the published `SHA256SUMS`, adds the Intel line,
  uploads the Intel archive, then the new `SHA256SUMS` with `--clobber`. `apple-only` (no write) runs when the release
  went out and `release-intel` did not succeed, and says so as a warning and in the step summary.
- `install.sh` reads `SHA256SUMS` for this Mac's archive before it downloads the archive, and when the sums do not
  name it says "this release has no build for an Intel Mac yet; nothing was installed" (or "an Apple-chip Mac").
  Before, an Intel Mac got curl's error and "could not download <url>; nothing was installed".
- `docs/development/install.md`: steps 2 and 3, and "Cutting a release".

## Red
`tests/f33/chips.rs` (made-up `sysctl` and `uname` make the script read an Intel Mac): a release with only the arm64
archive and line refused with "could not download ...jellymoon-macos-x86_64.tar.gz".

## Mutations (install.sh, against `tests/f33/chips.rs`)
| Mutation | Killed |
|---|---|
| the Intel words swapped for the Apple-chip ones | yes, the exact line |
| the archive fetched before the sums are read for it | yes, "could not download" |
| no check that the sums name the archive | yes, "could not download" |

## Open
- The workflow has not run: the YAML anchor, the split jobs, `gh release download --output`, the upload order and the
  `apple-only` job are proved by reading only. No actionlint or shellcheck is in the repo's tools.
- While `--clobber` replaces `SHA256SUMS` there is a moment with none; every installer then refuses with nothing
  installed.
- The baseline (`./init.sh`) had one failure outside this change: `f53`
  `gate::another_repo_than_the_followed_work_s_is_a_run_s_refusal_however_the_folder_was_found`.
