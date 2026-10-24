Unicode true
!include "MUI2.nsh"
!include "LogicLib.nsh"
!include "x64.nsh"
!include "WinVer.nsh"

!ifndef APP_VERSION
  !error "Usa scripts/package.ps1 per generare l'installer."
!endif
!ifndef MANIFEST_DIR
  !error "Manifest dei file mancante."
!endif

Name "Bibbia"
OutFile "${OUTPUT_FILE}"
InstallDir "$LOCALAPPDATA\Programs\Bibbia"
InstallDirRegKey HKCU "Software\BibbiaRust" "InstallDir"
RequestExecutionLevel user
ManifestDPIAware true
SetCompressor /SOLID lzma
ShowInstDetails show
ShowUninstDetails show
VIProductVersion "${APP_VERSION}.0"
VIAddVersionKey /LANG=1040 "ProductName" "Bibbia"
VIAddVersionKey /LANG=1040 "ProductVersion" "${APP_VERSION}"
VIAddVersionKey /LANG=1040 "FileDescription" "Installazione di Bibbia per Windows"
VIAddVersionKey /LANG=1040 "FileVersion" "${APP_VERSION}.0"
VIAddVersionKey /LANG=1040 "LegalCopyright" "Progetto Bibbia · Licenza MIT"

!define MUI_ABORTWARNING
!define MUI_WELCOMEPAGE_TEXT "Leggi la Bibbia offline, evidenzia i versetti e conserva i tuoi appunti.$\r$\n$\r$\nLa Diodati italiana e la KJV inglese sono incluse. Non sono richiesti account o download aggiuntivi.$\r$\n$\r$\nL'app sarà installata solo per il tuo utente."
!insertmacro MUI_PAGE_WELCOME
!insertmacro MUI_PAGE_LICENSE "${PROJECT_DIR}\LICENSE"
!insertmacro MUI_PAGE_DIRECTORY
!insertmacro MUI_PAGE_COMPONENTS
!insertmacro MUI_PAGE_INSTFILES
!define MUI_FINISHPAGE_RUN "$INSTDIR\Bibbia.Windows.exe"
!define MUI_FINISHPAGE_RUN_TEXT "Apri Bibbia"
!define MUI_FINISHPAGE_TEXT "Bibbia è stata installata.$\r$\n$\r$\nLa trovi nel menu Start. I tuoi appunti restano sul dispositivo e saranno conservati anche se disinstalli l'app."
!insertmacro MUI_PAGE_FINISH
!insertmacro MUI_UNPAGE_CONFIRM
!insertmacro MUI_UNPAGE_INSTFILES
!insertmacro MUI_UNPAGE_FINISH
!insertmacro MUI_LANGUAGE "Italian"

Var PreviousDir

!macro CheckAppClosed PREFIX
Function ${PREFIX}CheckAppClosed
  retry:
  System::Call 'kernel32::OpenMutexW(i 0x100000, i 0, w "Local\BibbiaRust.App") p .r0'
  ${If} $0 != 0
    System::Call 'kernel32::CloseHandle(p r0)'
    IfSilent blocked
    MessageBox MB_RETRYCANCEL|MB_ICONEXCLAMATION "Chiudi Bibbia prima di continuare. I tuoi appunti sono al sicuro." IDRETRY retry
    blocked:
    SetErrorLevel 2
    Quit
  ${EndIf}
FunctionEnd
!macroend
!insertmacro CheckAppClosed ""
!insertmacro CheckAppClosed "un."

Function .onInit
  ${IfNot} ${RunningX64}
    MessageBox MB_OK|MB_ICONSTOP "Questa versione richiede Windows a 64 bit." /SD IDOK
    SetErrorLevel 3
    Quit
  ${EndIf}
  SetRegView 64
  SetShellVarContext current
  ReadRegStr $0 HKLM "SOFTWARE\Microsoft\Windows NT\CurrentVersion" "CurrentBuildNumber"
  ${If} $0 < 19041
    MessageBox MB_OK|MB_ICONSTOP "È necessario Windows 10 versione 2004 o successiva, oppure Windows 11." /SD IDOK
    SetErrorLevel 3
    Quit
  ${EndIf}
  ReadRegStr $PreviousDir HKCU "Software\BibbiaRust" "InstallDir"
  ${If} $PreviousDir != ""
    StrCpy $INSTDIR $PreviousDir
  ${EndIf}
  Call CheckAppClosed
FunctionEnd

Function un.onInit
  SetRegView 64
  SetShellVarContext current
  Call un.CheckAppClosed
FunctionEnd

Section "Bibbia (obbligatorio)" MainSection
  SectionIn RO
  Call CheckAppClosed
  ; Use the previous uninstaller's exact file list when upgrading. Never remove
  ; the application directory recursively or touch %LOCALAPPDATA%\BibbiaRust.
  ${If} $PreviousDir != ""
  ${AndIf} ${FileExists} "$PreviousDir\Uninstall.exe"
    ExecWait '"$PreviousDir\Uninstall.exe" /S _?=$PreviousDir' $0
    ${If} $0 != 0
      MessageBox MB_OK|MB_ICONSTOP "Impossibile aggiornare Bibbia. Chiudi l'app e riprova." /SD IDOK
      SetErrorLevel 4
      Abort
    ${EndIf}
    Delete "$PreviousDir\Uninstall.exe"
    RMDir "$PreviousDir"
  ${EndIf}
  ClearErrors
  SetOutPath "$INSTDIR"
  !include "${MANIFEST_DIR}\install-files.nsh"
  WriteUninstaller "$INSTDIR\Uninstall.exe"
  CreateDirectory "$SMPROGRAMS\Bibbia"
  CreateShortCut "$SMPROGRAMS\Bibbia\Bibbia.lnk" "$INSTDIR\Bibbia.Windows.exe"
  WriteRegStr HKCU "Software\BibbiaRust" "InstallDir" "$INSTDIR"
  WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\BibbiaRust" "DisplayName" "Bibbia"
  WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\BibbiaRust" "DisplayVersion" "${APP_VERSION}"
  WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\BibbiaRust" "DisplayIcon" "$INSTDIR\Bibbia.Windows.exe"
  WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\BibbiaRust" "InstallLocation" "$INSTDIR"
  WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\BibbiaRust" "UninstallString" '$\"$INSTDIR\Uninstall.exe$\"'
  WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\BibbiaRust" "QuietUninstallString" '$\"$INSTDIR\Uninstall.exe$\" /S'
  WriteRegDWORD HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\BibbiaRust" "NoModify" 1
  WriteRegDWORD HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\BibbiaRust" "NoRepair" 1
  SetErrorLevel 0
SectionEnd

Section /o "Collegamento sul desktop" DesktopSection
  CreateShortCut "$DESKTOP\Bibbia.lnk" "$INSTDIR\Bibbia.Windows.exe"
SectionEnd

Section "Uninstall"
  Call un.CheckAppClosed
  !include "${MANIFEST_DIR}\uninstall-files.nsh"
  Delete "$INSTDIR\Uninstall.exe"
  RMDir "$INSTDIR"
  Delete "$SMPROGRAMS\Bibbia\Bibbia.lnk"
  RMDir "$SMPROGRAMS\Bibbia"
  Delete "$DESKTOP\Bibbia.lnk"
  DeleteRegKey HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\BibbiaRust"
  DeleteRegKey HKCU "Software\BibbiaRust"
  SetErrorLevel 0
SectionEnd
