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
# The oldest macOS the app is built for. app/macos/bundle.sh reads this line into the bundle's Info.plist.
MIN_MACOS=11.0
tmp="" new_app="" new_bin="" old_app=""

say() { printf '%s\n' "$*"; }
die() {
  printf 'jellymoon install: %s\n' "$*" >&2
  exit 1
}

# The bundle an update moved aside. With no bundle in its place the update stopped half way, and the old one goes
# back; with one in place the update is done, and the old one is deleted.
settle() {
  [ -n "$old_app" ] && [ -e "$old_app" ] || return 0
  if [ -e "$app_dir/Jellymoon.app" ]; then rm -rf "$old_app"; else mv "$old_app" "$app_dir/Jellymoon.app"; fi
}

# Runs on every exit: the old app goes back if the new one is not in place, and the download folder and anything
# half placed go, whether the install worked or not.
cleanup() {
  settle || true
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

# Each piece is copied next to where it goes, so a copy that stops half way changes nothing. The command is then
# renamed over the old one. A folder cannot be renamed over a folder that holds files, so the app takes three steps:
# the old bundle is moved aside under a name of ours in the same folder, the new one is moved in, and the old one is
# deleted. If the script stops between the two moves, by a failure or an interrupt, `cleanup` moves the old bundle
# back; if it is killed outright, the next run does, before anything else.
place() {
  # A folder made here is for you alone to write, whatever the umask: the app turns delivery on only for a command
  # no one else could replace. A folder that was there keeps its mode.
  (umask 022 && mkdir -p "$app_dir" "$bin_dir")
  new_app=$app_dir/.Jellymoon.app.new new_bin=$bin_dir/.jellymoon.new
  rm -rf "$new_app"
  cp -R "$tmp/unpacked/Jellymoon.app" "$new_app"
  cp "$tmp/unpacked/jellymoon" "$new_bin"
  chmod 755 "$new_bin"
  if [ -e "$app_dir/Jellymoon.app" ] || [ -L "$app_dir/Jellymoon.app" ]; then
    mv "$app_dir/Jellymoon.app" "$old_app"
  fi
  mv "$new_app" "$app_dir/Jellymoon.app"
  settle
  mv -f "$new_bin" "$bin_dir/jellymoon"
}

# Whether the folder the command is in can be written to by its group or by everyone, and is not sticky. Only a
# folder that was there before can be: the app does not offer to turn delivery on for a command in one.
open_bin() {
  mode=$(stat -L -f '%Mp%Lp' "$bin_dir" 2>/dev/null || true)
  case "$mode" in
    [1357]???) return 1 ;;
    *[2367]? | *[2367]) return 0 ;;
  esac
  return 1
}

# Refuses, before anything is downloaded, a Mac the app cannot start on and a target that is not ours to replace.
refuse_odd_targets() {
  macos=$(sw_vers -productVersion 2>/dev/null || true)
  case "${macos%%.*}" in
    '' | *[!0-9]*) die "could not read this Mac's macOS version; nothing was installed" ;;
  esac
  # Only the first number is compared: the minimum is a whole release.
  [ "${macos%%.*}" -ge "${MIN_MACOS%%.*}" ] ||
    die "Jellymoon needs macOS $MIN_MACOS or newer and this Mac has $macos; nothing was installed"
  # `mv` would put the new command inside a folder of that name, or inside the folder a link of that name points at.
  [ ! -d "$bin_dir/jellymoon" ] ||
    die "$bin_dir/jellymoon is a folder, not the jellymoon command: move it away and run this again; nothing was installed"
}

main() {
  [ $# -eq 0 ] || die "this script takes no arguments; its settings are the JELLYMOON_ variables named at its top"
  [ "$(uname -s)" = Darwin ] || die "this installer is for the Mac only for now; nothing was installed"
  [ -n "${HOME:-}" ] || die "HOME is not set; nothing was installed"
  base=${JELLYMOON_RELEASE_URL:-$RELEASE_URL}
  app_dir=${JELLYMOON_APP_DIR:-$HOME/Applications}
  bin_dir=${JELLYMOON_BIN_DIR:-$HOME/.local/bin}
  trap cleanup EXIT
  trap 'exit 1' HUP INT TERM
  # First of all, an old bundle that a killed run left moved aside goes back where it was.
  old_app=$app_dir/.Jellymoon.app.old
  settle
  did=installed
  [ ! -e "$app_dir/Jellymoon.app" ] || did=updated
  refuse_odd_targets
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
  if open_bin; then
    say "  $bin_dir can be written to by others, who could replace the command, so the app will not offer to turn" \
      "delivery on: run  jellymoon hook install  from a terminal, or  chmod go-w \"$bin_dir\"  first."
  fi
  say ""
  say "Next:"
  say "  open \"$app_dir/Jellymoon.app\"    shows today's page, read with one call through your own Claude Code"
  say "  \"$app_dir/Jellymoon.app/Contents/MacOS/jellymoon-app\"    the same from this terminal, if the opened app"
  say "                                     says Claude Code was not found"
  say "  jellymoon uninstall                takes Jellymoon off this Mac again"
}

# The whole script is read before any of it runs, so a download of this file that stops half way does nothing.
main "$@"
