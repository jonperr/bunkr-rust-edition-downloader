; Instalador do Bunkr Downloader para Windows (NSIS).
; Compile com: makensis /DVERSION=2.0.0 bunkr.nsi
; Requer bunkr.exe e (opcional) bunkr-gui.exe nesta pasta.

Unicode true
!include "MUI2.nsh"
!include "FileFunc.nsh"
!include "WordFunc.nsh"

!ifndef VERSION
  !define VERSION "2.0.0"
!endif

!define APPNAME "Bunkr Downloader"
!define PUBLISHER "Bunkr Downloader"
!define REGKEY "Software\Microsoft\Windows\CurrentVersion\Uninstall\BunkrDownloader"
!define INSTALLDIR "$LOCALAPPDATA\Bunkr"

Name "${APPNAME} ${VERSION}"
OutFile "bunkr-setup-${VERSION}-windows-x64.exe"
InstallDir "${INSTALLDIR}"
InstallDirRegKey HKCU "Software\BunkrDownloader" "InstallDir"
RequestExecutionLevel user
SetCompressor /SOLID lzma

; ---------------------------------------------------------------- visual
!define MUI_ABORTWARNING
!define MUI_ICON "app.ico"
!define MUI_UNICON "app.ico"
!define MUI_HEADERIMAGE
!define MUI_HEADERIMAGE_RIGHT
!define MUI_HEADERIMAGE_BITMAP "header.bmp"
!define MUI_WELCOMEFINISHPAGE_BITMAP "welcome.bmp"

!insertmacro MUI_PAGE_WELCOME
!insertmacro MUI_PAGE_COMPONENTS
!insertmacro MUI_PAGE_DIRECTORY
!insertmacro MUI_PAGE_INSTFILES
!insertmacro MUI_PAGE_FINISH

!insertmacro MUI_UNPAGE_CONFIRM
!insertmacro MUI_UNPAGE_INSTFILES

!insertmacro MUI_LANGUAGE "PortugueseBR"
!insertmacro MUI_LANGUAGE "English"

; ---------------------------------------------------------------- instalacao
Section "CLI (bunkr.exe)" SEC_CLI
  SectionIn RO
  SetOutPath "$INSTDIR"
  File "bunkr.exe"
  File "bunkr.toml.example"
SectionEnd

Section "Interface gráfica (bunkr-gui.exe)" SEC_GUI
  SetOutPath "$INSTDIR"
  File "bunkr-gui.exe"
  CreateShortcut "$DESKTOP\Bunkr Downloader.lnk" "$INSTDIR\bunkr-gui.exe" "" "$INSTDIR\bunkr-gui.exe" 0
  CreateDirectory "$SMPROGRAMS\Bunkr Downloader"
  CreateShortcut "$SMPROGRAMS\Bunkr Downloader\Bunkr Downloader.lnk" "$INSTDIR\bunkr-gui.exe" "" "$INSTDIR\bunkr-gui.exe" 0
SectionEnd

Section "Adicionar ao PATH" SEC_PATH
  ; Adiciona $INSTDIR ao PATH do usuario (sem duplicar).
  ReadRegStr $0 HKCU "Environment" "Path"
  ${If} $0 == ""
    WriteRegExpandStr HKCU "Environment" "Path" "$INSTDIR"
  ${Else}
    ${If} $0 != ""
      StrCpy $1 "$0"
      ; remove ocorrencias antigas de $INSTDIR
      ${WordReplace} "$0" ";$INSTDIR" "" "+" $2
      ${WordReplace} "$2" "$INSTDIR;" "" "+" $3
      ${WordReplace} "$3" "$INSTDIR" "" "+" $4
      WriteRegExpandStr HKCU "Environment" "Path" "$4;$INSTDIR"
    ${EndIf}
  ${EndIf}
  SendMessage ${HWND_BROADCAST} ${WM_WININICHANGE} 0 "STR:Environment" /TIMEOUT=5000
SectionEnd

; ---------------------------------------------------------------- registro
Section -Post
  WriteRegStr HKCU "Software\BunkrDownloader" "InstallDir" "$INSTDIR"
  WriteUninstaller "$INSTDIR\uninstall.exe"

  WriteRegStr HKCU "${REGKEY}" "DisplayName" "${APPNAME}"
  WriteRegStr HKCU "${REGKEY}" "DisplayVersion" "${VERSION}"
  WriteRegStr HKCU "${REGKEY}" "Publisher" "${PUBLISHER}"
  WriteRegStr HKCU "${REGKEY}" "DisplayIcon" "$INSTDIR\bunkr-gui.exe"
  WriteRegStr HKCU "${REGKEY}" "UninstallString" '"$INSTDIR\uninstall.exe"'
  WriteRegDWORD HKCU "${REGKEY}" "NoModify" 1
  WriteRegDWORD HKCU "${REGKEY}" "NoRepair" 1

  ${GetSize} "$INSTDIR" "/S=0K" $0 $1 $2
  IntFmt $0 "0x%08X" $0
  WriteRegDWORD HKCU "${REGKEY}" "EstimatedSize" "$0"
SectionEnd

!insertmacro MUI_FUNCTION_DESCRIPTION_BEGIN
  !insertmacro MUI_DESCRIPTION_TEXT ${SEC_CLI} "Ferramenta de linha de comando (obrigatória)."
  !insertmacro MUI_DESCRIPTION_TEXT ${SEC_GUI} "Janela gráfica com progresso, fila e modo escuro."
  !insertmacro MUI_DESCRIPTION_TEXT ${SEC_PATH} "Permite digitar 'bunkr' em qualquer terminal."
!insertmacro MUI_FUNCTION_DESCRIPTION_END

; ---------------------------------------------------------------- desinstalar
Section "Uninstall"
  Delete "$INSTDIR\bunkr.exe"
  Delete "$INSTDIR\bunkr-gui.exe"
  Delete "$INSTDIR\bunkr.toml.example"
  Delete "$INSTDIR\uninstall.exe"
  RMDir "$INSTDIR"

  Delete "$DESKTOP\Bunkr Downloader.lnk"
  Delete "$SMPROGRAMS\Bunkr Downloader\Bunkr Downloader.lnk"
  RMDir "$SMPROGRAMS\Bunkr Downloader"

  ReadRegStr $0 HKCU "Environment" "Path"
  ${WordReplace} "$0" ";$INSTDIR" "" "+" $1
  ${WordReplace} "$1" "$INSTDIR;" "" "+" $2
  ${WordReplace} "$2" "$INSTDIR" "" "+" $3
  WriteRegExpandStr HKCU "Environment" "Path" "$3"

  DeleteRegKey HKCU "${REGKEY}"
  DeleteRegKey HKCU "Software\BunkrDownloader"
SectionEnd
