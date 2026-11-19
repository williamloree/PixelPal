# Outil de test : envoie une commande « ctl » à une instance PixelPal lancée avec PIXELPAL_JETON.
# Exemple : .\tools\ctl.ps1 -Port 47800 -Cmd etat
#           .\tools\ctl.ps1 -Port 47800 -Cmd ajouter -Extra @{ code = '...' }
param([int]$Port = 47800, [string]$Cmd = 'etat', [hashtable]$Extra = @{}, [string]$Jeton = 'jeton-de-test')
$v = @{ type = 'ctl'; jeton = $Jeton; cmd = $Cmd } + $Extra
$c = New-Object System.Net.Sockets.TcpClient('127.0.0.1', $Port)
$s = $c.GetStream()
$s.ReadTimeout = 30000
$w = New-Object System.IO.StreamWriter($s, (New-Object System.Text.UTF8Encoding $false))
$w.WriteLine(($v | ConvertTo-Json -Compress -Depth 5))
$w.Flush()
$r = New-Object System.IO.StreamReader($s, [System.Text.Encoding]::UTF8)
$r.ReadLine()
$c.Close()
