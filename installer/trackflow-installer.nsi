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
!include "LogicLib.nsh"
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

; Se TrackFlow (o uno qualsiasi dei suoi sidecar watcher) è ancora in
; esecuzione, i suoi .exe restano bloccati da Windows e il "File"
; qui sotto fallisce con "Error opening file for writing" — bug reale
; riprodotto più volte durante i test di questa stessa sessione,
; installando/reinstallando mentre l'app girava ancora (anche solo in
; tray). Un utente reale può benissimo dimenticarsi di chiuderla prima
; di lanciare l'installer o un aggiornamento manuale — meglio chiudere
; tutto da soli invece di affidarsi a quella disciplina. `taskkill`
; ritorna un codice di errore se un processo non è in esecuzione, ma
; con ExecToLog non lo controlliamo: è normale e atteso che la maggior
; parte di questi processi non esista già.
!macro KillTrackFlowProcesses
  nsExec::ExecToLog 'taskkill /F /T /IM app.exe /IM launcher.exe /IM aw-watcher-afk.exe /IM aw-watcher-app-icons.exe /IM aw-watcher-claude-code.exe /IM aw-watcher-excel.exe /IM aw-watcher-screenshot.exe /IM aw-watcher-tray.exe /IM aw-watcher-vpn.exe /IM aw-watcher-vscode.exe /IM aw-watcher-window.exe'
  Pop $0
  ; 500ms non bastava sempre — bug reale trovato in questa sessione:
  ; anche con "taskkill" andato a buon fine, Windows non aveva ancora
  ; rilasciato l'handle di uno dei file un istante dopo, e le
  ; cancellazioni subito successive (current.txt, app-data\...)
  ; fallivano in silenzio (vedi le macro Robust* più sotto, che
  ; aggiungono comunque un secondo livello di sicurezza per questo
  ; stesso motivo).
  Sleep 1200
!macroend

; RMDir/Delete possono fallire in silenzio se anche un solo file al
; loro interno è ancora bloccato da un processo non del tutto
; terminato — successo esattamente questo in questa sessione: il log
; dell'uninstaller mostrava "Remove folder" come se fosse riuscito, ma
; la cartella (con tutto il contenuto, intatto) restava sul disco.
; Queste macro ritentano alcune volte con una breve pausa, e come
; ultima rete di sicurezza usano /REBOOTOK (MoveFileEx di Windows,
; MOVEFILE_DELAY_UNTIL_REBOOT) per garantire DAVVERO che il file
; sparisca, anche nel caso limite in cui resti bloccato in un modo che
; nemmeno taskkill risolve — al più tardi, al prossimo riavvio.
!macro RobustRMDirR path label
  Push $R0
  StrCpy $R0 0
  rmdir_retry_${label}:
    RMDir /r "${path}"
    IfFileExists "${path}\*.*" 0 rmdir_ok_${label}
      IntOp $R0 $R0 + 1
      IntCmp $R0 6 rmdir_giveup_${label}
      Sleep 500
      Goto rmdir_retry_${label}
  rmdir_giveup_${label}:
    RMDir /r /REBOOTOK "${path}"
    IfFileExists "${path}\*.*" 0 rmdir_ok_${label}
      MessageBox MB_ICONINFORMATION "Alcuni file in$\n${path}$\nerano ancora in uso e verranno rimossi automaticamente al prossimo riavvio del computer."
  rmdir_ok_${label}:
  Pop $R0
!macroend

!macro RobustDelete path label
  Push $R0
  StrCpy $R0 0
  delete_retry_${label}:
    Delete "${path}"
    IfFileExists "${path}" 0 delete_ok_${label}
      IntOp $R0 $R0 + 1
      IntCmp $R0 6 delete_giveup_${label}
      Sleep 500
      Goto delete_retry_${label}
  delete_giveup_${label}:
    Delete /REBOOTOK "${path}"
  delete_ok_${label}:
  Pop $R0
!macroend

; Windows tiene traccia di OGNI eseguibile mai lanciato sul PC in un
; pugno di chiavi di registro CONDIVISE con tutte le altre app
; (MuiCache: nome "amichevole" cache per Explorer; FeatureUsage\
; AppSwitched: cronologia Alt-Tab; AppCompatFlags\Compatibility
; Assistant: diagnostica compatibilità) — una voce per percorso
; completo di ogni .exe. Non si può cancellare la chiave (conterrebbe
; anche le voci di altre app), solo il singolo VALORE che corrisponde
; al NOSTRO percorso esatto. Usata sia in Uninstall che per ogni
; versione mai installata (vedi il ciclo su versions\* più sotto) —
; segnalato dall'utente dopo un controllo con Revo Uninstaller.
!macro CleanExeTraces exe
  DeleteRegValue HKCU "SOFTWARE\Classes\Local Settings\Software\Microsoft\Windows\Shell\MuiCache" "${exe}.ApplicationCompany"
  DeleteRegValue HKCU "SOFTWARE\Classes\Local Settings\Software\Microsoft\Windows\Shell\MuiCache" "${exe}.FriendlyAppName"
  DeleteRegValue HKCU "SOFTWARE\Microsoft\Windows\CurrentVersion\Explorer\FeatureUsage\AppSwitched" "${exe}"
  DeleteRegValue HKCU "SOFTWARE\Microsoft\Windows NT\CurrentVersion\AppCompatFlags\Compatibility Assistant\Store" "${exe}"
!macroend

Section "Install"
  !insertmacro KillTrackFlowProcesses

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

  ; Bug reale trovato in questa sessione, in due parti:
  ; 1) se current.txt risultava ancora bloccato da un processo non
  ;    del tutto terminato, la scrittura falliva e l'utente si
  ;    ritrovava a usare ancora la versione precedente dopo un
  ;    aggiornamento, senza nessun avviso.
  ; 2) il primo tentativo di fix (IfErrors dopo FileOpen) NON
  ;    funzionava: verificato con un'installazione silenziosa fatta
  ;    apposta per isolare il problema, current.txt restava comunque
  ;    con il valore vecchio — IfErrors non rileva in modo affidabile
  ;    un fallimento di FileOpen in NSIS, quindi il retry non
  ;    scattava mai, in silenzio, esattamente come il bug che doveva
  ;    correggere. Fix vero: si rilegge il file appena scritto e si
  ;    confronta il contenuto con quello atteso, invece di fidarsi del
  ;    flag di errore.
  Push $R0
  Push $R1
  StrCpy $R0 0
  write_currenttxt_retry:
    FileOpen $2 "$INSTDIR\current.txt" w
    FileWrite $2 "${VERSION}"
    FileClose $2

    FileOpen $2 "$INSTDIR\current.txt" r
    FileRead $2 $R1
    FileClose $2
    StrCmp $R1 "${VERSION}" write_currenttxt_ok
      IntOp $R0 $R0 + 1
      IntCmp $R0 6 write_currenttxt_failed
      Sleep 500
      Goto write_currenttxt_retry
  write_currenttxt_failed:
    MessageBox MB_ICONEXCLAMATION "Impossibile completare l'installazione: current.txt risulta ancora bloccato da un altro processo. Chiudi TrackFlow (anche dalla system tray, se presente) e riprova l'installazione."
  write_currenttxt_ok:
  Pop $R1
  Pop $R0

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
  !insertmacro KillTrackFlowProcesses

  ; Traccia ogni versione mai installata PRIMA di cancellare
  ; versions\ — dopo non sapremmo più quali cartelle/percorsi siano
  ; esistiti nel tempo (un aggiornamento automatico può aver
  ; accumulato più versioni oltre a quella corrente).
  FindFirst $0 $1 "$INSTDIR\versions\*"
  loop_versions:
    StrCmp $1 "" done_versions
    StrCmp $1 "." next_version
    StrCmp $1 ".." next_version
    IfFileExists "$INSTDIR\versions\$1\app.exe" 0 next_version
      !insertmacro CleanExeTraces "$INSTDIR\versions\$1\app.exe"
    next_version:
    FindNext $0 $1
    Goto loop_versions
  done_versions:
  FindClose $0

  !insertmacro CleanExeTraces "$INSTDIR\launcher.exe"
  !insertmacro CleanExeTraces "$INSTDIR\uninstall.exe"

  !insertmacro RobustRMDirR "$INSTDIR\versions" "versions"

  ; Dati utente veri (screenshot, cache icone app, stato watcher,
  ; moduli configurati) — a differenza dei file del programma sopra,
  ; qui chiediamo prima: potrebbero essere mesi di cronologia che
  ; l'utente vuole ancora consultare reinstallando in futuro, non va
  ; cancellata in silenzio solo perché "fa parte della pulizia".
  MessageBox MB_YESNO|MB_ICONQUESTION "Vuoi eliminare anche i dati raccolti finora (screenshot, cronologia app, progetti, categorie)?$\n$\nSe scegli No, resteranno sul disco e verranno ritrovati automaticamente in una futura reinstallazione." IDNO skip_app_data
    !insertmacro RobustRMDirR "$INSTDIR\app-data" "appdata"
    ; Il vero database degli eventi (quello che alimenta la Timeline)
    ; NON vive qui sotto $INSTDIR, ma nella cartella dati "Roaming" di
    ; Windows -- eredità del codice server vendorizzato da
    ; aw-server-rust (aw-server/src/dirs.rs, get_data_dir():
    ; dirs::data_dir().join("activitywatch").join("aw-server-rust") --
    ; su Windows $APPDATA, non $LOCALAPPDATA). Bug reale trovato in
    ; questa sessione: nessuna disinstallazione l'aveva mai toccato,
    ; quindi la cronologia della Timeline sopravviveva SEMPRE a
    ; qualunque disinstallazione, anche rispondendo "Sì" qui sopra.
    !insertmacro RobustRMDirR "$APPDATA\activitywatch\aw-server-rust" "eventdb"
  skip_app_data:

  !insertmacro RobustDelete "$INSTDIR\current.txt" "currenttxt"
  !insertmacro RobustDelete "$INSTDIR\launcher.exe" "launcherexe"
  !insertmacro RobustDelete "$INSTDIR\launcher-error.log" "launchererrorlog"
  !insertmacro RobustDelete "$INSTDIR\uninstall.exe" "uninstallexe"
  RMDir "$INSTDIR"

  Delete "$SMPROGRAMS\${PRODUCTNAME}\${PRODUCTNAME}.lnk"
  RMDir "$SMPROGRAMS\${PRODUCTNAME}"
  Delete "$DESKTOP\${PRODUCTNAME}.lnk"

  ; "Avvia con Windows" (Impostazioni → Generale in-app) scrive questo
  ; valore — mai stato ripulito da questo disinstallatore fino ad ora
  ; (bug reale, trovato in questa sessione): dopo un uninstall
  ; "pulito" restava comunque un tentativo di avvio automatico che
  ; punta a un launcher.exe ormai cancellato.
  DeleteRegValue HKCU "SOFTWARE\Microsoft\Windows\CurrentVersion\Run" "${PRODUCTNAME}"

  DeleteRegKey HKCU "${UNINSTKEY}"
  DeleteRegKey HKCU "${MANUKEY}"
  ; MANUKEY cancella solo la sotto-chiave "TrackFlow" — senza questa
  ; riga il genitore "trackflow" restava vuoto ma presente (bug reale
  ; trovato con Revo Uninstaller). /ifempty: la cancella solo se non
  ; contiene altro, non tocca nulla se per qualche motivo non è vuota.
  DeleteRegKey /ifempty HKCU "SOFTWARE\${MANUFACTURER}"
SectionEnd
