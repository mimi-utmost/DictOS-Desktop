$ErrorActionPreference = "Stop"
Add-Type -AssemblyName System.Windows.Forms
Add-Type -AssemblyName System.Drawing

$textFile = Join-Path $env:TEMP "dictos-e2e.txt"
Set-Content -Path $textFile -Value "xyzzy" -Encoding ASCII

$notepad = Start-Process notepad.exe -ArgumentList $textFile -PassThru
Start-Sleep -Seconds 3

# Foca o Notepad
$wshell = New-Object -ComObject WScript.Shell
$wshell.AppActivate($notepad.Id) | Out-Null
Start-Sleep -Milliseconds 800

# Seleciona tudo e dispara o hotkey do DictOS
[System.Windows.Forms.SendKeys]::SendWait("^a")
Start-Sleep -Milliseconds 500
[System.Windows.Forms.SendKeys]::SendWait("^+d")
Start-Sleep -Seconds 3

# Captura a tela
$bounds = [System.Windows.Forms.Screen]::PrimaryScreen.Bounds
$bitmap = New-Object System.Drawing.Bitmap($bounds.Width, $bounds.Height)
$graphics = [System.Drawing.Graphics]::FromImage($bitmap)
$graphics.CopyFromScreen($bounds.Location, [System.Drawing.Point]::Empty, $bounds.Size)
$shot = Join-Path $env:TEMP "dictos-e2e.png"
$bitmap.Save($shot, [System.Drawing.Imaging.ImageFormat]::Png)
$graphics.Dispose()
$bitmap.Dispose()

# Limpa: Esc fecha popup, fecha Notepad sem salvar
[System.Windows.Forms.SendKeys]::SendWait("{ESC}")
Start-Sleep -Milliseconds 500
$notepad.CloseMainWindow() | Out-Null
Start-Sleep -Milliseconds 500
if (-not $notepad.HasExited) { $notepad.Kill() }

Write-Output "screenshot: $shot"
