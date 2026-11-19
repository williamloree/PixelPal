# Outil de test : clic simple sur le personnage d'une instance (ouvre le menu de soins), capture,
# puis clic sur la ligne $Ligne du menu (0 = premier choix) et nouvelle capture.
param([int]$Pid_, [int]$Ligne = -1, [string]$Prefixe = "$env:TEMP\menu")
Add-Type -TypeDefinition @'
using System; using System.Runtime.InteropServices; using System.Collections.Generic;
public static class PM {
  [StructLayout(LayoutKind.Sequential)] public struct RECT { public int L,T,R,B; }
  [DllImport("user32.dll")] public static extern bool SetProcessDPIAware();
  [DllImport("user32.dll", CharSet=CharSet.Unicode)] public static extern IntPtr FindWindowEx(IntPtr p, IntPtr a, string c, string t);
  [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr h, out RECT r);
  [DllImport("user32.dll")] public static extern bool IsWindowVisible(IntPtr h);
  [DllImport("user32.dll")] public static extern uint GetWindowThreadProcessId(IntPtr h, out uint pid);
  [DllImport("user32.dll")] public static extern bool SetCursorPos(int x,int y);
  [DllImport("user32.dll")] public static extern void mouse_event(int f,int x,int y,int d,int e);
  public static List<int[]> Lister(uint pid) {
    var l = new List<int[]>(); IntPtr h = IntPtr.Zero;
    while ((h = FindWindowEx(IntPtr.Zero, h, "PixelPalCalque", null)) != IntPtr.Zero) {
      uint p; GetWindowThreadProcessId(h, out p); if (p != pid || !IsWindowVisible(h)) continue;
      RECT r; GetWindowRect(h, out r); l.Add(new int[]{r.L, r.T, r.R-r.L, r.B-r.T});
    }
    return l;
  }
}
'@
[PM]::SetProcessDPIAware() | Out-Null
Add-Type -AssemblyName System.Drawing
function Capture($n) { $b = New-Object System.Drawing.Bitmap 1920,1080; [System.Drawing.Graphics]::FromImage($b).CopyFromScreen(0,0,0,0,$b.Size); $b.Save("$Prefixe$n.png"); $b.Dispose() }
function Clic($x, $y) { [PM]::SetCursorPos($x, $y) | Out-Null; Start-Sleep -Milliseconds 80; [PM]::mouse_event(2, 0, 0, 0, 0); Start-Sleep -Milliseconds 60; [PM]::mouse_event(4, 0, 0, 0, 0) }
$u = 4
# le personnage : fenêtre de 60 × 80 points
$perso = [PM]::Lister([uint32]$Pid_) | Where-Object { $_[2] -eq 60 * $u -and $_[3] -eq 80 * $u } | Select-Object -First 1
if (-not $perso) { "personnage introuvable"; return }
Clic ($perso[0] + 30 * $u) ($perso[1] + [int]($perso[3] * 0.72))
Start-Sleep -Milliseconds 700
Capture 0
# le menu : fenêtre de 104 points de large, dessinée à la moitié de la taille du personnage
$menu = [PM]::Lister([uint32]$Pid_) | Where-Object { $_[2] -eq 104 * [math]::Max(2, [int]($u / 2)) } | Select-Object -First 1
if (-not $menu) { "menu introuvable"; return }
"menu en $($menu[0]),$($menu[1]) taille $($menu[2])x$($menu[3])"
if ($Ligne -ge 0) {
  # entête 15 points + 3 jauges de 10 points + 4 points, puis lignes de 12 points
  $k = [math]::Max(2, [int]($u / 2)); $y = $menu[1] + (15 + 30 + 4 + $Ligne * 12 + 6) * $k
  Clic ($menu[0] + 40 * $k) $y
  Start-Sleep -Milliseconds 1500
  Capture 1
}
