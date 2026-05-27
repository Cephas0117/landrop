; LANDrop NSIS installer hooks
; Adds Windows Firewall inbound rules so discovery and transfer work correctly.

!macro NSIS_HOOK_POSTINSTALL
  ; UDP 7777 — device discovery (mDNS broadcast fallback)
  nsExec::ExecToLog 'netsh advfirewall firewall add rule name="LANDrop Discovery (UDP)" \
    dir=in action=allow protocol=UDP localport=7777 program="$INSTDIR\LANDrop.exe" enable=yes profile=any'

  ; TCP 7878 — file transfer
  nsExec::ExecToLog 'netsh advfirewall firewall add rule name="LANDrop Transfer (TCP)" \
    dir=in action=allow protocol=TCP localport=7878 program="$INSTDIR\LANDrop.exe" enable=yes profile=any'
!macroend

!macro NSIS_HOOK_PREUNINSTALL
  ; Remove firewall rules on uninstall
  nsExec::ExecToLog 'netsh advfirewall firewall delete rule name="LANDrop Discovery (UDP)"'
  nsExec::ExecToLog 'netsh advfirewall firewall delete rule name="LANDrop Transfer (TCP)"'
!macroend
