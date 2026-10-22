; Register a Win32 toast identity even when an upgrade reuses existing shortcuts.
!macro NSIS_HOOK_POSTINSTALL
  WriteRegStr HKCU "Software\Classes\AppUserModelId\com.contextdock.desktop" "DisplayName" "ContextDock"
  WriteRegStr HKCU "Software\Classes\AppUserModelId\com.contextdock.desktop" "IconUri" "$INSTDIR\ContextDock.ico"
  WriteRegStr HKCU "Software\Classes\AppUserModelId\com.contextdock.desktop" "CustomActivator" "{F2B6E09C-86C1-4A12-B15E-C8330596F08B}"
  WriteRegStr HKCU "Software\Classes\AppUserModelId\com.contextdock.desktop" "ContextDockOwner" "$INSTDIR\${MAINBINARYNAME}.exe"
!macroend

; Only a real uninstall removes this installation's startup registration.
; Updates retain opt-in and Windows' StartupApproved state.
!macro NSIS_HOOK_POSTUNINSTALL
  ${If} $UpdateMode <> 1
    ReadRegStr $R0 HKCU "Software\Classes\AppUserModelId\com.contextdock.desktop" "ContextDockOwner"
    ${If} $R0 == "$INSTDIR\${MAINBINARYNAME}.exe"
      DeleteRegKey HKCU "Software\Classes\AppUserModelId\com.contextdock.desktop"
    ${EndIf}
    ReadRegStr $R0 HKCU "Software\Microsoft\Windows\CurrentVersion\Run" "ContextDock"
    ${If} $R0 == '$\"$INSTDIR\${MAINBINARYNAME}.exe$\" --background'
      DeleteRegValue HKCU "Software\Microsoft\Windows\CurrentVersion\Run" "ContextDock"
      DeleteRegValue HKCU "Software\Microsoft\Windows\CurrentVersion\Explorer\StartupApproved\Run" "ContextDock"
    ${EndIf}
  ${EndIf}
!macroend
