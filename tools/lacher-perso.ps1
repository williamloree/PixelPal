# Outil de test : attrape le personnage d'une instance (par numéro de processus), le monte à la hauteur
# $Y, le lâche, puis prend des captures pendant la chute.
param([int]$Pid_, [int]$Y = 150, [string]$Prefixe = "$env:TEMP\chute")
Add-Type -TypeDefinition @'
using System; using System.Runtime.InteropServices; using System.Collections.Generic;
public static class PL {
  [StructLayout(LayoutKind.Sequential)] public struct RECT { public int L,T,R,B; }
  [DllImport("user32.dll")] public static extern bool SetProcessDPIAware();
  [DllImport("user32.dll", CharSet=CharSet.Unicode)] public static extern IntPtr FindWindowEx(IntPtr p, IntPtr a, string c, string t);
  [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr h, out RECT r);
  [DllImport("user32.dll")] public static extern uint GetWindowThreadProcessId(IntPtr h, out uint pid);
  [DllImport("user32.dll")] public static extern bool SetCursorPos(int x,int y);
  [DllImport("user32.dll")] public static extern void mouse_event(int f,int x,int y,int d,int e);
  public static List<int[]> Lister(uint pid) {
    var l = new List<int[]>(); IntPtr h = IntPtr.Zero;
    while ((h = FindWindowEx(IntPtr.Zero, h, "PixelPalCalque", null)) != IntPtr.Zero) {
      uint p; GetWindowThreadProcessId(h, out p); if (p != pid) continue;
      RECT r; GetWindowRect(h, out r); l.Add(new int[]{r.L, r.T, r.R-r.L, r.B-r.T});
    }
    return l;
  }
}
'@
[PL]::SetProcessDPIAware() | Out-Null
Add-Type -AssemblyName System.Drawing
function Capture($n) { $b = New-Object System.Drawing.Bitmap 1920,1080; [System.Drawing.Graphics]::FromImage($b).CopyFromScreen(0,0,0,0,$b.Size); $b.Save("$Prefixe$n.png"); $b.Dispose() }
# la fenêtre du personnage est la plus grande (60 × 80 points)
$v = [PL]::Lister([uint32]$Pid_) | Sort-Object { $_[2] * $_[3] } -Descending | Select-Object -First 1
if (-not $v) { "personnage introuvable"; return }
# on le prend par le torse : centre horizontal, à 72 % de la hauteur de sa fenêtre
$x = $v[0] + [int]($v[2] / 2); $y0 = $v[1] + [int]($v[3] * 0.72)
"prise en $x,$y0"
[PL]::SetCursorPos($x, $y0) | Out-Null; Start-Sleep -Milliseconds 100
[PL]::mouse_event(2, 0, 0, 0, 0)
for ($yy = $y0; $yy -gt $Y; $yy -= 25) { [PL]::SetCursorPos($x, $yy) | Out-Null; Start-Sleep -Milliseconds 30 }
Start-Sleep -Milliseconds 600
Capture 0
[PL]::mouse_event(4, 0, 0, 0, 0)
for ($i = 1; $i -le 4; $i++) { Start-Sleep -Milliseconds 1200; Capture $i }
