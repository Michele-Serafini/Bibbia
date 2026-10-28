Unicode true
!include "MUI2.nsh"
!include "LogicLib.nsh"
!include "x64.nsh"
!include "WinVer.nsh"
!include "FileFunc.nsh"
!define APP_ID "io.github.MicheleSerafini.Bibbia"
!define ARP_KEY "Software\Microsoft\Windows\CurrentVersion\Uninstall\BibbiaRust"

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
VIAddVersionKey /LANG=1040 "LegalCopyright" "Codice MIT · testi con licenze separate"

!define MUI_ABORTWARNING
!define MUI_ICON "${PROJECT_DIR}\app\Assets\Bibbia.ico"
!define MUI_UNICON "${PROJECT_DIR}\app\Assets\Bibbia.ico"
!define MUI_WELCOMEPAGE_TEXT "Leggi la Bibbia offline, evidenzia parole e frasi e conserva i tuoi appunti.$\r$\n$\r$\nSono incluse CEI 2008 e Martini cattoliche in italiano, Diodati, KJV e una raccolta estesa di 83 libri/sezioni in inglese. Il testo CEI è destinato a finalità formative e didattiche non commerciali. Non sono richiesti account o download aggiuntivi.$\r$\n$\r$\nL'app sarà installata solo per il tuo utente."
!insertmacro MUI_PAGE_WELCOME
!insertmacro MUI_PAGE_LICENSE "${PROJECT_DIR}\LICENSE"
!insertmacro MUI_PAGE_LICENSE "${PROJECT_DIR}\core\data\LICENSE.cei2008.txt"
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

; Give Start/Search the same stable identity as the running WinUI process.
; IPropertyStore is provided by Windows: no plugin, PowerShell or service.
Function SetShortcutIdentity
  Exch $0
  Push $1
  Push $2
  Push $3
  Push $4
  Push $5
  Push $6
  Push $7
  System::Call 'ole32::CoInitialize(p 0) i .r6'
  System::Call 'shell32::SHGetPropertyStoreFromParsingName(w r0, p 0, i 2, g "{886D8EEB-8CF2-4446-8D02-CDBA1DBDCF99}", *p .r1) i .r2'
  ${If} $2 == 0
    System::Call 'shlwapi::SHStrDupW(w "${APP_ID}", *p .r3) i .r2'
    ${If} $2 == 0
      System::Call '*(g "{9F4C2855-9F79-4B39-A8D0-E1D42DE1D5F3}", i 5) p .r4'
      ; PROPVARIANT: VT_LPWSTR, six reserved bytes, string pointer, union pad.
      System::Call '*(i 31, i 0, p r3, i 0) p .r5'
      System::Call '$1->6(p r4, p r5) i .r2'
      System::Free $4
      System::Free $5
      System::Call 'ole32::CoTaskMemFree(p r3)'
      ${If} $2 == 0
        ; PKEY_AppUserModel_UninstallCommand (PID 37) is the documented
        ; shortcut property used by Start to expose its Uninstall verb.
        StrCpy $7 '$\"$INSTDIR\Uninstall.exe$\"'
        System::Call 'shlwapi::SHStrDupW(w r7, *p .r3) i .r2'
        ${If} $2 == 0
          System::Call '*(g "{9F4C2855-9F79-4B39-A8D0-E1D42DE1D5F3}", i 37) p .r4'
          System::Call '*(i 31, i 0, p r3, i 0) p .r5'
          System::Call '$1->6(p r4, p r5) i .r2'
          System::Free $4
          System::Free $5
          System::Call 'ole32::CoTaskMemFree(p r3)'
          ${If} $2 == 0
            System::Call '$1->7() i .r2'
          ${EndIf}
        ${EndIf}
      ${EndIf}
    ${EndIf}
    System::Call '$1->2()'
  ${EndIf}
  ${If} $2 != 0
    DetailPrint "Impossibile impostare l'identità del collegamento: $2"
    SetErrors
  ${EndIf}
  ${If} $6 >= 0
    System::Call 'ole32::CoUninitialize()'
  ${EndIf}
  Pop $7
  Pop $6
  Pop $5
  Pop $4
  Pop $3
  Pop $2
  Pop $1
  Pop $0
FunctionEnd

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
  Push "$SMPROGRAMS\Bibbia\Bibbia.lnk"
  Call SetShortcutIdentity
  CreateShortCut "$SMPROGRAMS\Bibbia\Disinstalla Bibbia.lnk" "$INSTDIR\Uninstall.exe" "" "$INSTDIR\Uninstall.exe" 0
  WriteRegStr HKCU "Software\BibbiaRust" "InstallDir" "$INSTDIR"
  WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\BibbiaRust" "DisplayName" "Bibbia"
  WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\BibbiaRust" "DisplayVersion" "${APP_VERSION}"
  WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\BibbiaRust" "DisplayIcon" "$INSTDIR\Bibbia.Windows.exe"
  WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\BibbiaRust" "InstallLocation" "$INSTDIR"
  WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\BibbiaRust" "UninstallString" '$\"$INSTDIR\Uninstall.exe$\"'
  WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\BibbiaRust" "QuietUninstallString" '$\"$INSTDIR\Uninstall.exe$\" /S'
  WriteRegDWORD HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\BibbiaRust" "NoModify" 1
  WriteRegDWORD HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\BibbiaRust" "NoRepair" 1
  WriteRegStr HKCU "${ARP_KEY}" "Publisher" "Michele Serafini"
  WriteRegStr HKCU "${ARP_KEY}" "AppUserModelID" "${APP_ID}"
  WriteRegStr HKCU "${ARP_KEY}" "URLInfoAbout" "https://github.com/Michele-Serafini/Bibbia"
  WriteRegDWORD HKCU "${ARP_KEY}" "SystemComponent" 0
  WriteRegDWORD HKCU "${ARP_KEY}" "NoRemove" 0
  ${GetSize} "$INSTDIR" "/S=0K" $0 $1 $2
  WriteRegDWORD HKCU "${ARP_KEY}" "EstimatedSize" $0
  WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\App Paths\Bibbia.Windows.exe" "" "$INSTDIR\Bibbia.Windows.exe"
  WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\App Paths\Bibbia.Windows.exe" "Path" "$INSTDIR"
  System::Call 'shell32::SHChangeNotify(i 0x08000000, i 0, p 0, p 0)'
  SetErrorLevel 0
SectionEnd

Section /o "Collegamento sul desktop" DesktopSection
  CreateShortCut "$DESKTOP\Bibbia.lnk" "$INSTDIR\Bibbia.Windows.exe"
  Push "$DESKTOP\Bibbia.lnk"
  Call SetShortcutIdentity
SectionEnd

Section "Uninstall"
  Call un.CheckAppClosed
  !include "${MANIFEST_DIR}\uninstall-files.nsh"
  Delete "$INSTDIR\Uninstall.exe"
  RMDir "$INSTDIR"
  Delete "$SMPROGRAMS\Bibbia\Bibbia.lnk"
  Delete "$SMPROGRAMS\Bibbia\Disinstalla Bibbia.lnk"
  RMDir "$SMPROGRAMS\Bibbia"
  Delete "$DESKTOP\Bibbia.lnk"
  DeleteRegKey HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\BibbiaRust"
  DeleteRegKey HKCU "Software\BibbiaRust"
  DeleteRegKey HKCU "Software\Microsoft\Windows\CurrentVersion\App Paths\Bibbia.Windows.exe"
  System::Call 'shell32::SHChangeNotify(i 0x08000000, i 0, p 0, p 0)'
  SetErrorLevel 0
SectionEnd
