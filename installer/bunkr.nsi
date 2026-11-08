Unicode true
!include "MUI2.nsh"
!include "FileFunc.nsh"
!include "WordFunc.nsh"
!include "WinMessages.nsh"
!ifndef VERSION
  !define VERSION "2.0.0"
!endif
!ifndef TAG
  !define TAG "en"
!endif
!define APPNAME "Bunkr Downloader"
!define REGKEY "Software\Microsoft\Windows\CurrentVersion\Uninstall\BunkrDownloader"
Name "${APPNAME} ${VERSION}"
OutFile "bunkr-setup-${VERSION}-windows-${TAG}-x64.exe"
InstallDir "$LOCALAPPDATA\Bunkr"
RequestExecutionLevel user
ShowInstDetails show

!define MUI_ICON "app.ico"
!define MUI_UNICON "app.ico"
!define MUI_HEADERIMAGE
!define MUI_HEADERIMAGE_BITMAP "header.bmp"
!define MUI_WELCOMEFINISHPAGE_BITMAP "welcome.bmp"
!define MUI_FINISHPAGE_RUN "$INSTDIR\bunkr-gui.exe"
!define MUI_FINISHPAGE_RUN_TEXT $(RUN_NOW)
!define MUI_FINISHPAGE_RUN_TIMEOUT 8000

!insertmacro MUI_PAGE_WELCOME
!insertmacro MUI_PAGE_COMPONENTS
!insertmacro MUI_PAGE_DIRECTORY
!insertmacro MUI_PAGE_INSTFILES
!insertmacro MUI_PAGE_FINISH
!insertmacro MUI_UNPAGE_CONFIRM
!insertmacro MUI_UNPAGE_INSTFILES

!insertmacro MUI_LANGUAGE "English"
!insertmacro MUI_LANGUAGE "PortugueseBR"
!ifndef APP_LANG
  !define APP_LANG "en"
!endif
!if "${APP_LANG}" == "pt"
  !define LANG_ID ${LANG_PORTUGUESEBR}
!else
  !define LANG_ID ${LANG_ENGLISH}
!endif

LangString RUN_NOW        ${LANG_ENGLISH}      "Start Bunkr Downloader now"
LangString RUN_NOW        ${LANG_PORTUGUESEBR} "Abrir o Bunkr Downloader agora"
LangString SEC_CLI        ${LANG_ENGLISH}      "Command-line (bunkr.exe)"
LangString SEC_CLI        ${LANG_PORTUGUESEBR} "Linha de comando (bunkr.exe)"
LangString SEC_GUI        ${LANG_ENGLISH}      "Desktop app (bunkr-gui.exe)"
LangString SEC_GUI        ${LANG_PORTUGUESEBR} "Aplicativo (bunkr-gui.exe)"
LangString SEC_PATH       ${LANG_ENGLISH}      "Add to PATH"
LangString SEC_PATH       ${LANG_PORTUGUESEBR} "Adicionar ao PATH"
LangString DESK           ${LANG_ENGLISH}      "Create a desktop shortcut"
LangString DESK           ${LANG_PORTUGUESEBR} "Criar atalho na área de trabalho"
LangString DESK_HDR       ${LANG_ENGLISH}      "Shortcuts"
LangString DESK_HDR       ${LANG_PORTUGUESEBR} "Atalhos"
LangString DESK_SUB       ${LANG_ENGLISH}      "Where shortcuts should be placed."
LangString DESK_SUB       ${LANG_PORTUGUESEBR} "Onde os atalhos serão criados."

Var DESKTOP_ON
Var DESK_CHECK

# Options page with the desktop-shortcut checkbox.
Page custom OptionsPage OptionsPageLeave
Function OptionsPage
  !insertmacro MUI_HEADER_TEXT $(DESK_HDR) $(DESK_SUB)
  nsDialogs::Create 1018
  Pop $0
  ${If} $0 == error
    Abort
  ${EndIf}
  ${NSD_CreateCheckbox} 0u 0u 100% 14u "$(DESK)"
  Pop $DESK_CHECK
  ${If} $DESKTOP_ON == "1"
    ${NSD_Check} $DESK_CHECK
  ${EndIf}
  nsDialogs::Show
FunctionEnd

Function OptionsPageLeave
  ${NSD_GetState} $DESK_CHECK $DESKTOP_ON
FunctionEnd

Function .onInit
  StrCpy $DESKTOP_ON "1"
  ; Force the language chosen at build time; NSIS would otherwise pick one from
  ; the OS locale, so the English installer would show Portuguese on a pt-BR
  ; machine.
  StrCpy $LANGUAGE ${LANG_ID}
FunctionEnd

Section "$(SEC_CLI)" SEC_CLI
  SectionIn RO
  SetOutPath "$INSTDIR"
  File "bunkr.exe"
  File "bunkr.toml.example"
  WriteRegStr HKCU "${REGKEY}" "DisplayName" "${APPNAME} ${VERSION}"
  WriteRegStr HKCU "${REGKEY}" "DisplayVersion" "${VERSION}"
  WriteRegStr HKCU "${REGKEY}" "Publisher" "jonperr"
  WriteRegStr HKCU "${REGKEY}" "URLInfoAbout" "https://github.com/jonperr/bunkr-rust-edition-downloader"
  WriteRegStr HKCU "${REGKEY}" "InstallLocation" "$INSTDIR"
  WriteRegStr HKCU "${REGKEY}" "DisplayIcon" "$INSTDIR\bunkr-gui.exe"
  WriteRegDword HKCU "${REGKEY}" "NoModify" 1
  WriteRegDword HKCU "${REGKEY}" "NoRepair" 1
SectionEnd

Section "$(SEC_GUI)" SEC_GUI
  SetOutPath "$INSTDIR"
  File "bunkr-gui.exe"
  CreateDirectory "$SMPROGRAMS\Bunkr Downloader"
  CreateShortcut "$SMPROGRAMS\Bunkr Downloader\Bunkr Downloader.lnk" "$INSTDIR\bunkr-gui.exe" "" "$INSTDIR\bunkr-gui.exe" 0
  ${If} $DESKTOP_ON == "1"
    CreateShortcut "$DESKTOP\Bunkr Downloader.lnk" "$INSTDIR\bunkr-gui.exe" "" "$INSTDIR\bunkr-gui.exe" 0
  ${EndIf}
SectionEnd

Section "$(SEC_PATH)" SEC_PATH
  ReadRegStr $0 HKCU "Environment" "PATH"
  ${WordFind} "$0" "$INSTDIR" "E+1{" $1
  ${If} $1 == ""
    ${If} $0 == ""
      WriteRegExpandStr HKCU "Environment" "PATH" "$INSTDIR"
    ${Else}
      WriteRegExpandStr HKCU "Environment" "PATH" "$0;$INSTDIR"
    ${EndIf}
    SendMessage ${HWND_BROADCAST} ${WM_WININICHANGE} 0 "STR:Environment" /TIMEOUT=50
  ${EndIf}
SectionEnd

Section -Post
  ; The GUI reads its language from this file, not from the OS locale.
  StrCpy $2 "en"
  ${If} "${TAG}" == "ptbr"
    StrCpy $2 "pt"
  ${EndIf}
  WriteINIStr "$INSTDIR\bunkr_gui_settings.ini" "General" "lang" "$2"
  WriteUninstaller "$INSTDIR\uninstall.exe"
SectionEnd

Section "Uninstall"
  Delete "$INSTDIR\bunkr.exe"
  Delete "$INSTDIR\bunkr-gui.exe"
  Delete "$INSTDIR\bunkr.toml.example"
  Delete "$INSTDIR\bunkr_gui_settings.ini"
  Delete "$INSTDIR\uninstall.exe"
  Delete "$DESKTOP\Bunkr Downloader.lnk"
  Delete "$SMPROGRAMS\Bunkr Downloader\Bunkr Downloader.lnk"
  RMDir "$SMPROGRAMS\Bunkr Downloader"
  ; Leave $INSTDIR first, otherwise Windows keeps the folder because it is the CWD.
  SetOutPath "$TEMP"
  RMDir "$INSTDIR"
  ReadRegStr $0 HKCU "Environment" "PATH"
  ${WordReplace} "$0" ";$INSTDIR" "" "+" $1
  ${WordReplace} "$1" "$INSTDIR;" "" "+" $2
  ${WordReplace} "$2" "$INSTDIR" "" "+" $3
  WriteRegExpandStr HKCU "Environment" "PATH" "$3"
  SendMessage ${HWND_BROADCAST} ${WM_WININICHANGE} 0 "STR:Environment" /TIMEOUT=50
  DeleteRegKey HKCU "${REGKEY}"
SectionEnd