# Outil de test : met la fenêtre principale d'un processus au premier plan, clique éventuellement
# à une position relative (en pixels physiques), puis la capture.
param([int]$Pid_, [string]$Sortie = "$env:TEMP\fenetre.png", [int]$ClicX = -1, [int]$ClicY = -1)
Add-Type -TypeDefinition @'
using System; using System.Runtime.InteropServices;
public static class CF {
  [StructLayout(LayoutKind.Sequential)] public struct RECT { public int L,T,R,B; }
  [DllImport("user32.dll")] public static extern bool SetProcessDPIAware();
  [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr h, out RECT r);
  [DllImport("user32.dll")] public static extern bool SetForegroundWindow(IntPtr h);
  [DllImport("user32.dll")] public static extern bool SetCursorPos(int x,int y);
  [DllImport("user32.dll")] public static extern void mouse_event(int f,int x,int y,int d,int e);
}
'@
[CF]::SetProcessDPIAware() | Out-Null
$p = Get-Process -Id $Pid_
$h = $p.MainWindowHandle
[CF]::SetForegroundWindow($h) | Out-Null
Start-Sleep -Milliseconds 400
$r = New-Object CF+RECT
[CF]::GetWindowRect($h, [ref]$r) | Out-Null
if ($ClicX -ge 0) {
  [CF]::SetCursorPos($r.L + $ClicX, $r.T + $ClicY) | Out-Null
  Start-Sleep -Milliseconds 100
  [CF]::mouse_event(2, 0, 0, 0, 0); Start-Sleep -Milliseconds 60; [CF]::mouse_event(4, 0, 0, 0, 0)
  Start-Sleep -Milliseconds 900
}
Add-Type -AssemblyName System.Drawing
$b = New-Object System.Drawing.Bitmap ($r.R - $r.L), ($r.B - $r.T)
[System.Drawing.Graphics]::FromImage($b).CopyFromScreen($r.L, $r.T, 0, 0, $b.Size)
$b.Save($Sortie)
"$($r.L),$($r.T) $($r.R - $r.L)x$($r.B - $r.T)"
