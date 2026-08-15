; Installer NSIS scritto a mano per TrackFlow — NON generato/derivato dal
; template di Tauri (quello lo abbiamo scartato per un tentativo simile
; in passato: troppo fragile da mantenere in sincrono coi nomi hash che
; Vite rigenera ad ogni build). Questo è deliberatamente piccolo e
; autonomo: impacchetta l'output già pronto di `npx tauri build`
; (app.exe, i 9 watcher, dist/) dentro una cartella versions\<VERSIONE>\,
; copia il launcher stabile alla radice, e scrive current.txt — vedi
; launcher/src/main.rs per come queste tre cose lavorano insieme
; nell'auto-aggiornamento in stile Squirrel.
;
; VERSIONE va passata da riga di comando: makensis /DVERSION=0.1.0 ...
; (fallback qui sotto solo per poter testare a mano senza specificarla).
!ifndef VERSION
  !define VERSION "0.0.0-dev"
!endif

!define PRODUCTNAME "TrackFlow"
!define MANUFACTURER "trackflow"
!define IDENTIFIER "it.trackflow.desktop"
!define UNINSTKEY "Software\Microsoft\Windows\CurrentVersion\Uninstall\${PRODUCTNAME}"
!define MANUKEY "Software\${MANUFACTURER}\${PRODUCTNAME}"

; Percorsi sorgente — relativi a questo file (installer/trackflow-installer.nsi)
!define SRC_ROOT "..\"
!define SRC_RELEASE "${SRC_ROOT}src-tauri\target\release\"
!define SRC_DIST "${SRC_ROOT}dist\"
!define SRC_ICONS "${SRC_ROOT}src-tauri\icons\"
!define SRC_LAUNCHER "${SRC_ROOT}launcher\target\release\launcher.exe"

Unicode true
ManifestDPIAware true
SetCompressor /SOLID lzma

!include "MUI2.nsh"
!include "Win\COM.nsh"
!include "Win\Propkey.nsh"

; Il collegamento (Menu Start/Desktop) punta a launcher.exe, non più
; direttamente ad app.exe come faceva l'installer generato da Tauri —
; launcher.exe non ha un manifest che dichiara un AppUserModelID suo, e
; senza intervenire esplicitamente Windows gli assegna un ID sintetico
; basato sul percorso del file (verificato con Get-StartApps). app.exe,
; quando poi manda un toast, dichiara SEMPRE "it.trackflow.desktop"
; (vedi notifications.rs) — se il collegamento non ha lo stesso ID, il
; toast risulterebbe orfano esattamente come il bug già risolto una
; volta in questa sessione (show() ritorna Ok ma l'icona/il toast non
; compare a dovere). Questa macro (stessa identica tecnica già usata
; dal template di Tauri, letta da src-tauri/target/release/nsis/x64/
; utils.nsh) imposta l'AppUserModelID sul file .lnk stesso via COM,
; indipendentemente da cosa dichiara l'eseguibile a cui punta.
!define BUNDLEID "${IDENTIFIER}"
!macro SetLnkAppUserModelId shortcut
  !insertmacro ComHlpr_CreateInProcInstance ${CLSID_ShellLink} ${IID_IShellLink} r0 ""
  ${If} $0 P<> 0
    ${IUnknown::QueryInterface} $0 '("${IID_IPersistFile}",.r1)'
    ${If} $1 P<> 0
      ${IPersistFile::Load} $1 '("${shortcut}", ${STGM_READWRITE})'
      ${IUnknown::QueryInterface} $0 '("${IID_IPropertyStore}",.r2)'
      ${If} $2 P<> 0
        System::Call 'Oleaut32::SysAllocString(w "${BUNDLEID}") i.r3'
        System::Call '*${SYSSTRUCT_PROPERTYKEY}(${PKEY_AppUserModel_ID})p.r4'
        System::Call '*${SYSSTRUCT_PROPVARIANT}(${VT_BSTR},,&i4 $3)p.r5'
        ${IPropertyStore::SetValue} $2 '($4,$5)'

        System::Call 'Oleaut32::SysFreeString($3)'
        System::Free $4
        System::Free $5

        ${IPropertyStore::Commit} $2 ""
        ${IUnknown::Release} $2 ""
        ${IPersistFile::Save} $1 '("${shortcut}",1)'
      ${EndIf}
      ${IUnknown::Release} $1 ""
    ${EndIf}
    ${IUnknown::Release} $0 ""
  ${EndIf}
!macroend

Name "${PRODUCTNAME}"
OutFile "trackflow-setup-${VERSION}.exe"
InstallDir "$LOCALAPPDATA\${PRODUCTNAME}"
InstallDirRegKey HKCU "${MANUKEY}" "InstallDir"
RequestExecutionLevel user

!define MUI_ICON "${SRC_ICONS}icon.ico"
!define MUI_UNICON "${SRC_ICONS}icon.ico"
!define MUI_ABORTWARNING

!insertmacro MUI_PAGE_WELCOME
!insertmacro MUI_PAGE_DIRECTORY
!insertmacro MUI_PAGE_INSTFILES
!define MUI_FINISHPAGE_RUN "$INSTDIR\launcher.exe"
!define MUI_FINISHPAGE_RUN_TEXT "Avvia TrackFlow"
!insertmacro MUI_PAGE_FINISH

!insertmacro MUI_UNPAGE_CONFIRM
!insertmacro MUI_UNPAGE_INSTFILES

!insertmacro MUI_LANGUAGE "English"

Section "Install"
  ; Watcher/app della versione corrente — MAI sovrascrive una versione
  ; già installata: ogni versione vive nella propria cartella, così
  ; un'installazione (= un aggiornamento, dal punto di vista
  ; dell'updater interno) non tocca mai i file che un'istanza in
  ; esecuzione ha aperti.
  SetOutPath "$INSTDIR\versions\${VERSION}"
  File "${SRC_RELEASE}app.exe"
  File "${SRC_RELEASE}aw-watcher-afk.exe"
  File "${SRC_RELEASE}aw-watcher-app-icons.exe"
  File "${SRC_RELEASE}aw-watcher-claude-code.exe"
  File "${SRC_RELEASE}aw-watcher-excel.exe"
  File "${SRC_RELEASE}aw-watcher-screenshot.exe"
  File "${SRC_RELEASE}aw-watcher-tray.exe"
  File "${SRC_RELEASE}aw-watcher-vpn.exe"
  File "${SRC_RELEASE}aw-watcher-vscode.exe"
  File "${SRC_RELEASE}aw-watcher-window.exe"

  SetOutPath "$INSTDIR\versions\${VERSION}\dist"
  File /r "${SRC_DIST}*.*"

  ; Solo l'icona usata per il toast di notifica (vedi notifications.rs) —
  ; non serve l'intero set di icone qui, quella vive nell'exe stesso.
  SetOutPath "$INSTDIR\versions\${VERSION}\icons"
  File /oname=notification-icon.png "${SRC_ICONS}128x128.png"

  ; Il launcher invece SEMPRE nella radice, MAI dentro versions\ — è
  ; l'unica cosa a cui puntano i collegamenti, e resta la stessa identica
  ; copia attraverso tutti gli aggiornamenti (vedi launcher/src/main.rs).
  SetOutPath "$INSTDIR"
  File /oname=launcher.exe "${SRC_LAUNCHER}"

  FileOpen $0 "$INSTDIR\current.txt" w
  FileWrite $0 "${VERSION}"
  FileClose $0

  CreateDirectory "$SMPROGRAMS\${PRODUCTNAME}"
  CreateShortcut "$SMPROGRAMS\${PRODUCTNAME}\${PRODUCTNAME}.lnk" "$INSTDIR\launcher.exe"
  !insertmacro SetLnkAppUserModelId "$SMPROGRAMS\${PRODUCTNAME}\${PRODUCTNAME}.lnk"
  CreateShortcut "$DESKTOP\${PRODUCTNAME}.lnk" "$INSTDIR\launcher.exe"
  !insertmacro SetLnkAppUserModelId "$DESKTOP\${PRODUCTNAME}.lnk"

  WriteUninstaller "$INSTDIR\uninstall.exe"
  WriteRegStr HKCU "${MANUKEY}" "InstallDir" "$INSTDIR"
  WriteRegStr HKCU "${MANUKEY}" "Identifier" "${IDENTIFIER}"

  WriteRegStr HKCU "${UNINSTKEY}" "DisplayName" "${PRODUCTNAME}"
  WriteRegStr HKCU "${UNINSTKEY}" "DisplayVersion" "${VERSION}"
  WriteRegStr HKCU "${UNINSTKEY}" "Publisher" "${MANUFACTURER}"
  WriteRegStr HKCU "${UNINSTKEY}" "InstallLocation" "$INSTDIR"
  WriteRegStr HKCU "${UNINSTKEY}" "DisplayIcon" "$INSTDIR\versions\${VERSION}\app.exe"
  WriteRegStr HKCU "${UNINSTKEY}" "UninstallString" '"$INSTDIR\uninstall.exe"'
  WriteRegDWORD HKCU "${UNINSTKEY}" "NoModify" 1
  WriteRegDWORD HKCU "${UNINSTKEY}" "NoRepair" 1
SectionEnd

Section "Uninstall"
  RMDir /r "$INSTDIR\versions"
  Delete "$INSTDIR\current.txt"
  Delete "$INSTDIR\launcher.exe"
  Delete "$INSTDIR\launcher-error.log"
  Delete "$INSTDIR\uninstall.exe"
  RMDir "$INSTDIR"

  Delete "$SMPROGRAMS\${PRODUCTNAME}\${PRODUCTNAME}.lnk"
  RMDir "$SMPROGRAMS\${PRODUCTNAME}"
  Delete "$DESKTOP\${PRODUCTNAME}.lnk"

  DeleteRegKey HKCU "${UNINSTKEY}"
  DeleteRegKey HKCU "${MANUKEY}"
SectionEnd
