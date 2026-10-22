; Internet Download Center NSIS kancaları.
; Kurulum bitince uygulama ilk açılışta tarayıcı ve sistem entegrasyonunu sihirbazla sorar.
; Kaldırmada köprü kayıtları ve idc:// işleyicisi temizlenir.

!macro NSIS_HOOK_PREINSTALL
  ; Çalışan örneği kapat (dosyalar kilitli kalmasın).
  nsExec::Exec 'taskkill /IM internet-download-center.exe /F'
  nsExec::Exec 'taskkill /IM idc-host.exe /F'
!macroend

!macro NSIS_HOOK_POSTINSTALL
!macroend

!macro NSIS_HOOK_PREUNINSTALL
  nsExec::Exec 'taskkill /IM internet-download-center.exe /F'
  nsExec::Exec 'taskkill /IM idc-host.exe /F'
!macroend

!macro NSIS_HOOK_POSTUNINSTALL
  DeleteRegKey HKCU "Software\Google\Chrome\NativeMessagingHosts\com.furkanyildirim.idc"
  DeleteRegKey HKCU "Software\Chromium\NativeMessagingHosts\com.furkanyildirim.idc"
  DeleteRegKey HKCU "Software\Microsoft\Edge\NativeMessagingHosts\com.furkanyildirim.idc"
  DeleteRegKey HKCU "Software\BraveSoftware\Brave-Browser\NativeMessagingHosts\com.furkanyildirim.idc"
  DeleteRegKey HKCU "Software\Vivaldi\NativeMessagingHosts\com.furkanyildirim.idc"
  DeleteRegKey HKCU "Software\Opera Software\NativeMessagingHosts\com.furkanyildirim.idc"
  DeleteRegKey HKCU "Software\Mozilla\NativeMessagingHosts\com.furkanyildirim.idc"
  DeleteRegKey HKCU "Software\LibreWolf\NativeMessagingHosts\com.furkanyildirim.idc"
  DeleteRegKey HKCU "Software\Waterfox\NativeMessagingHosts\com.furkanyildirim.idc"
  DeleteRegKey HKCU "Software\Classes\idc"
  DeleteRegValue HKCU "Software\Microsoft\Windows\CurrentVersion\Run" "Internet Download Center"
!macroend
