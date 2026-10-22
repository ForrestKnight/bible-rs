#!/usr/bin/env bash
# Signs one Windows executable in place: the hook scripts/package-windows.sh calls for
# bible-rs.exe, and makensis for the uninstaller and the installer.
#
#   WINDOWS_SIGN_COMMAND='<signing command>' scripts/sign-windows.sh path/to/file.exe
#
# WINDOWS_SIGN_COMMAND is run with the file as its last argument and must sign that file in
# place. WINDOWS.md has the command for Azure Artifact Signing (formerly Trusted Signing:
# signtool with its dlib); any signtool or osslsigncode command line works the same way.
set -euo pipefail

file="${1:?usage: sign-windows.sh FILE}"
: "${WINDOWS_SIGN_COMMAND:?WINDOWS_SIGN_COMMAND is not set (WINDOWS.md, Signing)}"
if command -v cygpath >/dev/null; then
  # Git Bash: Windows programs get a Windows path, and their /switches are left alone rather
  # than taken for paths.
  file="$(cygpath -w "$file")"
  export MSYS_NO_PATHCONV=1
fi
echo "Signing $file"
eval "$WINDOWS_SIGN_COMMAND \"\$file\""

# signtool (SIGNTOOL, or on the PATH) checks that the signature chains to a trusted root and
# carries a timestamp; without it the signing command's own exit status has to do.
signtool="${SIGNTOOL:-$(command -v signtool || true)}"
if [[ -n "$signtool" ]]; then
  "$signtool" verify /pa /tw "$file"
fi
