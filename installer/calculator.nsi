; Windows installer for the calculator (NSIS 3).
; (This file is saved as UTF-8 with BOM so NSIS reads the French accents correctly.)
;
; Build it with:  python tools/build_installer.py
; (that script builds the release .exe first and passes VERSION / EXE / OUTFILE)
;
; Installs for the current user only (no admin prompt) into
; %LOCALAPPDATA%\Programs\Calculator, adds a Start menu shortcut, an optional
; desktop shortcut, and an uninstaller listed in Windows "Apps".

Unicode true
!include "MUI2.nsh"

!ifndef VERSION
  !define VERSION "0.0.0"
!endif
!ifndef EXE
  !define EXE "..\target\release\calculator.exe"
!endif
!ifndef OUTFILE
  !define OUTFILE "..\target\dist\calculator-windows-setup.exe"
!endif

!define APP_NAME "Calculator"
!define APP_EXE "calculator.exe"
!define PUBLISHER "ayoubjardak-stack"
!define UNINST_KEY "Software\Microsoft\Windows\CurrentVersion\Uninstall\IcedCalculator"

Name "${APP_NAME}"
OutFile "${OUTFILE}"
InstallDir "$LOCALAPPDATA\Programs\${APP_NAME}"
InstallDirRegKey HKCU "${UNINST_KEY}" "InstallLocation"
RequestExecutionLevel user
SetCompressor /SOLID lzma
BrandingText "${APP_NAME} ${VERSION}"

; Details shown in the setup file's Properties → Details tab.
VIProductVersion "${VERSION}.0"
VIAddVersionKey "ProductName" "${APP_NAME}"
VIAddVersionKey "ProductVersion" "${VERSION}"
VIAddVersionKey "FileVersion" "${VERSION}"
VIAddVersionKey "FileDescription" "${APP_NAME} installer"
VIAddVersionKey "CompanyName" "${PUBLISHER}"
VIAddVersionKey "LegalCopyright" "${PUBLISHER}"

; ── Pages ────────────────────────────────────────────────────────────────
!define MUI_ICON "..\assets\icon.ico"
!define MUI_UNICON "..\assets\icon.ico"
!define MUI_ABORTWARNING
!define MUI_COMPONENTSPAGE_NODESC

; Our own texts come from the LangStrings below, so they follow the
; installer language (English or French, picked from Windows' language).
!define MUI_WELCOMEPAGE_TEXT "$(WELCOME_TEXT)"
!define MUI_FINISHPAGE_RUN "$INSTDIR\${APP_EXE}"
!define MUI_FINISHPAGE_RUN_TEXT "$(RUN_TEXT)"

!insertmacro MUI_PAGE_WELCOME
!insertmacro MUI_PAGE_COMPONENTS
!insertmacro MUI_PAGE_DIRECTORY
!insertmacro MUI_PAGE_INSTFILES
!insertmacro MUI_PAGE_FINISH

!insertmacro MUI_UNPAGE_CONFIRM
!insertmacro MUI_UNPAGE_INSTFILES

!insertmacro MUI_LANGUAGE "English"
!insertmacro MUI_LANGUAGE "French"

LangString WELCOME_TEXT ${LANG_ENGLISH} "This will install ${APP_NAME} ${VERSION} on your computer.$\r$\n$\r$\nNo administrator rights are needed: it is installed for your Windows account only.$\r$\n$\r$\nClick Next to continue."
LangString WELCOME_TEXT ${LANG_FRENCH} "Ce programme va installer ${APP_NAME} ${VERSION} sur votre ordinateur.$\r$\n$\r$\nAucun droit administrateur n'est nécessaire : l'application est installée uniquement pour votre compte Windows.$\r$\n$\r$\nCliquez sur Suivant pour continuer."
LangString RUN_TEXT ${LANG_ENGLISH} "Open ${APP_NAME} now"
LangString RUN_TEXT ${LANG_FRENCH} "Ouvrir ${APP_NAME} maintenant"
LangString SEC_APP ${LANG_ENGLISH} "${APP_NAME} (required)"
LangString SEC_APP ${LANG_FRENCH} "${APP_NAME} (obligatoire)"
LangString SEC_DESKTOP ${LANG_ENGLISH} "Desktop shortcut"
LangString SEC_DESKTOP ${LANG_FRENCH} "Raccourci sur le Bureau"

; ── Install ──────────────────────────────────────────────────────────────
Section "$(SEC_APP)" SecApp
  SectionIn RO

  ; Close a running copy so its .exe can be replaced.
  nsExec::Exec 'taskkill /IM "${APP_EXE}" /F'

  SetOutPath "$INSTDIR"
  File "/oname=${APP_EXE}" "${EXE}"
  WriteUninstaller "$INSTDIR\uninstall.exe"

  CreateShortcut "$SMPROGRAMS\${APP_NAME}.lnk" "$INSTDIR\${APP_EXE}"

  ; Entry in Settings → Apps (and the old "Programs and Features").
  WriteRegStr HKCU "${UNINST_KEY}" "DisplayName" "${APP_NAME}"
  WriteRegStr HKCU "${UNINST_KEY}" "DisplayVersion" "${VERSION}"
  WriteRegStr HKCU "${UNINST_KEY}" "Publisher" "${PUBLISHER}"
  WriteRegStr HKCU "${UNINST_KEY}" "DisplayIcon" "$INSTDIR\${APP_EXE}"
  WriteRegStr HKCU "${UNINST_KEY}" "InstallLocation" "$INSTDIR"
  WriteRegStr HKCU "${UNINST_KEY}" "UninstallString" '"$INSTDIR\uninstall.exe"'
  WriteRegStr HKCU "${UNINST_KEY}" "QuietUninstallString" '"$INSTDIR\uninstall.exe" /S'
  WriteRegDWORD HKCU "${UNINST_KEY}" "NoModify" 1
  WriteRegDWORD HKCU "${UNINST_KEY}" "NoRepair" 1
  WriteRegDWORD HKCU "${UNINST_KEY}" "EstimatedSize" 12000
SectionEnd

Section "$(SEC_DESKTOP)" SecDesktop
  CreateShortcut "$DESKTOP\${APP_NAME}.lnk" "$INSTDIR\${APP_EXE}"
SectionEnd

; ── Uninstall ────────────────────────────────────────────────────────────
Section "Uninstall"
  nsExec::Exec 'taskkill /IM "${APP_EXE}" /F'

  Delete "$INSTDIR\${APP_EXE}"
  Delete "$INSTDIR\uninstall.exe"
  RMDir "$INSTDIR"

  Delete "$SMPROGRAMS\${APP_NAME}.lnk"
  Delete "$DESKTOP\${APP_NAME}.lnk"

  DeleteRegKey HKCU "${UNINST_KEY}"

  ; Your saved settings (%APPDATA%\calculator) are kept on purpose,
  ; so reinstalling brings your theme and size back.
SectionEnd
