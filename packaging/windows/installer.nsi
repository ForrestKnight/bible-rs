; The Windows installer for Bible: per user (no administrator rights), into
; %LOCALAPPDATA%\Programs\Bible, with a Start menu shortcut and an entry in Settings > Apps.
; scripts/package-windows.sh builds it; WINDOWS.md says how to sign it.
;
; Defines (makensis -D...):
;   VERSION    the app's version, 0.2.0
;   SRC        the folder holding bible-rs.exe and the licence files
;   OUT        the installer to write
;   COPYRIGHT  the copyright line
;   SIGN       optional: a script that signs, in place, the file it is given (the uninstaller and
;              the installer are signed as they are written), run by the shell SIGN_SHELL

Unicode true
ManifestDPIAware true
ManifestSupportedOS Win10
SetCompressor /SOLID lzma
RequestExecutionLevel user

!include "MUI2.nsh"
!include "FileFunc.nsh"

!define NAME "Bible"
!define PUBLISHER "Fabrika LLC"
!define EXE "bible-rs.exe"
!define UNINSTALLER "Uninstall Bible.exe"
!define UNINSTALL_KEY "Software\Microsoft\Windows\CurrentVersion\Uninstall\bible-rs"
!define URL "https://github.com/michaelshimeles/bible-rs"

Name "${NAME}"
OutFile "${OUT}"
InstallDir "$LOCALAPPDATA\Programs\${NAME}"
InstallDirRegKey HKCU "${UNINSTALL_KEY}" "InstallLocation"
BrandingText "${NAME} ${VERSION}"

VIProductVersion "${VERSION}.0"
VIAddVersionKey /LANG=1033 "ProductName" "${NAME}"
VIAddVersionKey /LANG=1033 "CompanyName" "${PUBLISHER}"
VIAddVersionKey /LANG=1033 "FileDescription" "${NAME} Setup"
VIAddVersionKey /LANG=1033 "FileVersion" "${VERSION}"
VIAddVersionKey /LANG=1033 "ProductVersion" "${VERSION}"
VIAddVersionKey /LANG=1033 "LegalCopyright" "${COPYRIGHT}"

!ifdef SIGN
  !finalize '"${SIGN_SHELL}" "${SIGN}" "%1"' = 0
  !uninstfinalize '"${SIGN_SHELL}" "${SIGN}" "%1"' = 0
!endif

!define MUI_ICON "bible-rs.ico"
!define MUI_UNICON "bible-rs.ico"
!define MUI_ABORTWARNING
!define MUI_FINISHPAGE_RUN "$INSTDIR\${EXE}"
!define MUI_FINISHPAGE_RUN_TEXT "Open ${NAME}"

!insertmacro MUI_PAGE_DIRECTORY
!insertmacro MUI_PAGE_INSTFILES
!insertmacro MUI_PAGE_FINISH
!insertmacro MUI_UNPAGE_CONFIRM
!insertmacro MUI_UNPAGE_INSTFILES
!insertmacro MUI_LANGUAGE "English"

; Windows won't replace a running program, so ask for the app to be closed first.
!macro WAIT_FOR_APP_TO_CLOSE
  IfFileExists "$INSTDIR\${EXE}" 0 +7
    ClearErrors
    FileOpen $0 "$INSTDIR\${EXE}" a
    IfErrors 0 +3
      MessageBox MB_RETRYCANCEL|MB_ICONEXCLAMATION "${NAME} is open. Close it, then choose Retry." \
        /SD IDCANCEL IDRETRY -4
      Abort
    FileClose $0
!macroend

Section "Install"
  SetShellVarContext current
  SetOutPath "$INSTDIR"
  !insertmacro WAIT_FOR_APP_TO_CLOSE
  File "${SRC}\${EXE}"
  File "${SRC}\LICENSE.txt"
  File "${SRC}\OFL-Inter.txt"
  File "${SRC}\OFL-Literata.txt"
  WriteUninstaller "$INSTDIR\${UNINSTALLER}"

  CreateShortcut "$SMPROGRAMS\${NAME}.lnk" "$INSTDIR\${EXE}" "" "$INSTDIR\${EXE}" 0

  ; Settings > Apps > Installed apps.
  WriteRegStr HKCU "${UNINSTALL_KEY}" "DisplayName" "${NAME}"
  WriteRegStr HKCU "${UNINSTALL_KEY}" "DisplayVersion" "${VERSION}"
  WriteRegStr HKCU "${UNINSTALL_KEY}" "Publisher" "${PUBLISHER}"
  WriteRegStr HKCU "${UNINSTALL_KEY}" "DisplayIcon" "$INSTDIR\${EXE},0"
  WriteRegStr HKCU "${UNINSTALL_KEY}" "InstallLocation" "$INSTDIR"
  WriteRegStr HKCU "${UNINSTALL_KEY}" "UninstallString" '"$INSTDIR\${UNINSTALLER}"'
  WriteRegStr HKCU "${UNINSTALL_KEY}" "QuietUninstallString" '"$INSTDIR\${UNINSTALLER}" /S'
  WriteRegStr HKCU "${UNINSTALL_KEY}" "URLInfoAbout" "${URL}"
  WriteRegStr HKCU "${UNINSTALL_KEY}" "HelpLink" "${URL}/blob/main/WINDOWS.md"
  WriteRegDWORD HKCU "${UNINSTALL_KEY}" "NoModify" 1
  WriteRegDWORD HKCU "${UNINSTALL_KEY}" "NoRepair" 1
  ${GetSize} "$INSTDIR" "/S=0K" $0 $1 $2
  WriteRegDWORD HKCU "${UNINSTALL_KEY}" "EstimatedSize" $0
SectionEnd

; Notes, highlights and settings stay in %APPDATA%\bible-rs and API keys in the Credential
; Manager, as dragging the Mac app to the Trash keeps its data.
Section "Uninstall"
  SetShellVarContext current
  !insertmacro WAIT_FOR_APP_TO_CLOSE
  Delete "$SMPROGRAMS\${NAME}.lnk"
  Delete "$INSTDIR\${EXE}"
  Delete "$INSTDIR\LICENSE.txt"
  Delete "$INSTDIR\OFL-Inter.txt"
  Delete "$INSTDIR\OFL-Literata.txt"
  Delete "$INSTDIR\${UNINSTALLER}"
  RMDir "$INSTDIR"
  DeleteRegKey HKCU "${UNINSTALL_KEY}"
SectionEnd
