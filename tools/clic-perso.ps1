# Outil de test : clique sur le personnage d'une instance PixelPal (par numéro de processus) et capture l'écran.
param([int]$Pid_ = 0, [string]$Capture = "$env:TEMP\pixelpal-capture.png")
Add-Type -TypeDefinition @'
using System; using System.Runtime.InteropServices; using System.Collections.Generic;
public static class PP {
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
      uint p; GetWindowThreadProcessId(h, out p); if (pid != 0 && p != pid) continue;
      RECT r; GetWindowRect(h, out r); l.Add(new int[]{r.L, r.T, r.R-r.L, r.B-r.T});
    }
    return l;
  }
}
'@
[PP]::SetProcessDPIAware() | Out-Null
$persos = @([PP]::Lister([uint32]$Pid_) | Where-Object { $_[2] -eq $_[3] -and $_[2] -ge 80 })
"Personnages : $($persos.Count)"
if ($persos.Count -gt 0) {
  $v = $persos[0]
  [PP]::SetCursorPos($v[0] + $v[2] / 2, $v[1] + [int]($v[3] * 0.7)) | Out-Null
  [PP]::mouse_event(2, 0, 0, 0, 0); [PP]::mouse_event(4, 0, 0, 0, 0)
  Start-Sleep 4
}
Add-Type -AssemblyName System.Windows.Forms, System.Drawing
$e = [System.Windows.Forms.SystemInformation]::VirtualScreen
$b = New-Object System.Drawing.Bitmap $e.Width, $e.Height
[System.Drawing.Graphics]::FromImage($b).CopyFromScreen($e.X, $e.Y, 0, 0, $b.Size)
$b.Save($Capture)
