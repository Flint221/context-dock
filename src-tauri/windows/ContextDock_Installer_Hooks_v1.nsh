; Only a real uninstall removes this installation's startup registration.
; Updates retain opt-in and Windows' StartupApproved state.
!macro NSIS_HOOK_POSTUNINSTALL
  ${If} $UpdateMode <> 1
    ReadRegStr $R0 HKCU "Software\Microsoft\Windows\CurrentVersion\Run" "ContextDock"
    ${If} $R0 == '$\"$INSTDIR\${MAINBINARYNAME}.exe$\" --background'
      DeleteRegValue HKCU "Software\Microsoft\Windows\CurrentVersion\Run" "ContextDock"
      DeleteRegValue HKCU "Software\Microsoft\Windows\CurrentVersion\Explorer\StartupApproved\Run" "ContextDock"
    ${EndIf}
  ${EndIf}
!macroend
