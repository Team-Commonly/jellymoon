#!/bin/sh
# Installs Jellymoon on a Mac, in your own folders, with no sudo and no Apple Developer signature:
#
#   curl -fsSL https://raw.githubusercontent.com/Team-Commonly/jellymoon/main/install.sh | sh
#
# It downloads the latest release for this Mac's chip and its SHA256SUMS, refuses the download unless the checksum
# matches, and then puts Jellymoon.app in ~/Applications and `jellymoon` in ~/.local/bin. Running it again updates
# both in place. `jellymoon uninstall` removes them. Nothing else on the machine is touched, and no shell profile is
# edited. docs/development/install.md says what the checksum does and does not prove.
#
# Settings, all optional:
#   JELLYMOON_APP_DIR      where Jellymoon.app goes          (default: $HOME/Applications)
#   JELLYMOON_BIN_DIR      where `jellymoon` goes            (default: $HOME/.local/bin)
#   JELLYMOON_RELEASE_URL  the folder the release is read from, https:// or file://
set -eu

RELEASE_URL=https://github.com/Team-Commonly/jellymoon/releases/latest/download
tmp="" new_app="" new_bin=""

say() { printf '%s\n' "$*"; }
die() {
  printf 'jellymoon install: %s\n' "$*" >&2
  exit 1
}

# Runs on every exit: the download folder and anything half placed go, whether the install worked or not.
cleanup() {
  [ -z "$tmp" ] || rm -rf "$tmp"
  [ -z "$new_app" ] || rm -rf "$new_app"
  [ -z "$new_bin" ] || rm -f "$new_bin"
}

# fetch <url> <file>. A web download may only ever speak https, redirects included.
fetch() {
  case "$1" in
    https://*) curl --proto '=https' --proto-redir '=https' --tlsv1.2 -fsSL -o "$2" "$1" ;;
    file://*) curl -fsS -o "$2" "$1" ;;
    *) die "JELLYMOON_RELEASE_URL must start with https:// or file://" ;;
  esac || die "could not download $1; nothing was installed"
}

# The archive name's last part for this Mac. A shell running under Rosetta reports x86_64 on an Apple chip.
chip() {
  if [ "$(sysctl -n hw.optional.arm64 2>/dev/null || true)" = 1 ]; then
    say arm64
    return
  fi
  case "$(uname -m)" in
    arm64 | x86_64) uname -m ;;
    *) die "no Jellymoon release for a $(uname -m) Mac; nothing was installed" ;;
  esac
}

# Downloads the archive and SHA256SUMS into $tmp and unpacks the archive into $tmp/unpacked, only once its checksum
# matches. Until then everything downloaded is data: it is hashed and compared, never unpacked or run.
download() {
  archive=jellymoon-macos-$(chip).tar.gz
  say "Downloading Jellymoon for this Mac ($archive)..."
  fetch "$base/SHA256SUMS" "$tmp/SHA256SUMS"
  fetch "$base/$archive" "$tmp/$archive"
  want=$(awk -v name="$archive" '$2 == name || $2 == "*" name { print $1; exit }' "$tmp/SHA256SUMS")
  got=$(shasum -a 256 "$tmp/$archive" | awk '{ print $1 }')
  [ "${#want}" -eq 64 ] || die "SHA256SUMS has no checksum for $archive; nothing was installed"
  [ "$want" = "$got" ] || die "the checksum of $archive does not match SHA256SUMS; nothing was installed"
  mkdir "$tmp/unpacked"
  tar -xzf "$tmp/$archive" -C "$tmp/unpacked" || die "could not unpack $archive; nothing was installed"
  for item in "$tmp/unpacked/Jellymoon.app" "$tmp/unpacked/jellymoon"; do
    [ ! -L "$item" ] && [ -e "$item" ] || die "$archive does not hold Jellymoon.app and jellymoon; nothing was installed"
  done
  [ -d "$tmp/unpacked/Jellymoon.app" ] || die "Jellymoon.app in $archive is not a folder; nothing was installed"
  # curl sets no quarantine flag, so Gatekeeper has nothing to ask about. Should one be there all the same, it is
  # taken off these files of ours and nothing else; Gatekeeper itself is never switched off.
  xattr -dr com.apple.quarantine "$tmp/unpacked" 2>/dev/null || true
}

# Each piece is copied next to where it goes and then renamed over the old one, so a copy that stops half way
# leaves the old install as it was.
place() {
  mkdir -p "$app_dir" "$bin_dir"
  new_app=$app_dir/.Jellymoon.app.new new_bin=$bin_dir/.jellymoon.new
  rm -rf "$new_app"
  cp -R "$tmp/unpacked/Jellymoon.app" "$new_app"
  cp "$tmp/unpacked/jellymoon" "$new_bin"
  chmod 755 "$new_bin"
  rm -rf "$app_dir/Jellymoon.app"
  mv "$new_app" "$app_dir/Jellymoon.app"
  mv -f "$new_bin" "$bin_dir/jellymoon"
}

main() {
  [ $# -eq 0 ] || die "this script takes no arguments; its settings are the JELLYMOON_ variables named at its top"
  [ "$(uname -s)" = Darwin ] || die "this installer is for the Mac only for now; nothing was installed"
  [ -n "${HOME:-}" ] || die "HOME is not set; nothing was installed"
  base=${JELLYMOON_RELEASE_URL:-$RELEASE_URL}
  app_dir=${JELLYMOON_APP_DIR:-$HOME/Applications}
  bin_dir=${JELLYMOON_BIN_DIR:-$HOME/.local/bin}
  did=installed
  [ ! -e "$app_dir/Jellymoon.app" ] || did=updated

  trap cleanup EXIT
  trap 'exit 1' HUP INT TERM
  tmp=$(mktemp -d "${TMPDIR:-/tmp}/jellymoon-install.XXXXXX")
  download
  place

  say ""
  say "  .  *  .   Jellymoon is $did.   .  *  ."
  say "  the app      $app_dir/Jellymoon.app"
  say "  the command  $bin_dir/jellymoon"
  case ":$PATH:" in
    *":$bin_dir:"*) ;;
    *) say "  $bin_dir is not on your PATH yet: add  export PATH=\"$bin_dir:\$PATH\"  to your shell profile." ;;
  esac
  say ""
  say "Next:"
  say "  jellymoon page > ~/jellymoon-today.json    reads today into a page, with one call through your own Claude Code"
  say "  open \"$app_dir/Jellymoon.app\" --args --page ~/jellymoon-today.json"
  say "  jellymoon uninstall                        takes Jellymoon off this Mac again"
}

# The whole script is read before any of it runs, so a download of this file that stops half way does nothing.
main "$@"
