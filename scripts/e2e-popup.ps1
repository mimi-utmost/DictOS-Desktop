# E2E do popup do DictOS (Etapa 4).
# Uso: rode o daemon primeiro (`dictos run --dict data/dict/en-wordnet`), depois:
#   powershell -ExecutionPolicy Bypass -File scripts/e2e-popup.ps1 [-Word xyzzy]
#
# AVISO: este script faz captura de tela, envia teclas sintéticas e lê texto
# de outros apps via UI Automation — padrão comportamental que antivírus
# (ex.: Windows Defender) podem classificar como falso positivo
# (Trojan:BAT/ScreenShoter). É uma ferramenta de teste legítima; não contorne
# isso com evasão, configure exclusão no seu ambiente de dev se necessário.
#
# Verifica, com uma seleção real no Bloco de Notas:
#   1. popup aparece junto à seleção (distância <= tolerância);
#   2. o foco NÃO sai do Notepad quando o popup abre;
#   3. Esc fecha o popup;
#   4. screenshot final em %TEMP%\dictos-e2e.png.
param([string]$Word = "serendipity")

$ErrorActionPreference = "Stop"
Add-Type -AssemblyName System.Windows.Forms
Add-Type -AssemblyName System.Drawing
Add-Type -AssemblyName UIAutomationClient

Add-Type -TypeDefinition @"
using System;
using System.Text;
using System.Runtime.InteropServices;
public class Win32E2E {
    [DllImport("user32.dll")] public static extern IntPtr GetForegroundWindow();
    [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr h, out RECT r);
    [DllImport("user32.dll")] public static extern bool IsWindowVisible(IntPtr h);
    [DllImport("user32.dll")] public static extern bool EnumWindows(EnumProc cb, IntPtr l);
    [DllImport("user32.dll")] public static extern uint GetWindowThreadProcessId(IntPtr h, out uint pid);
    public delegate bool EnumProc(IntPtr h, IntPtr l);
    public struct RECT { public int Left, Top, Right, Bottom; }
    public static IntPtr FindWindowOfProcess(uint targetPid) {
        IntPtr found = IntPtr.Zero;
        EnumWindows((h, l) => {
            uint pid; GetWindowThreadProcessId(h, out pid);
            if (pid == targetPid && IsWindowVisible(h)) { found = h; return false; }
            return true;
        }, IntPtr.Zero);
        return found;
    }
}
"@

$failures = @()
function Check($name, $ok) {
    if ($ok) { Write-Output "PASS: $name" } else { Write-Output "FAIL: $name"; $script:failures += $name }
}

$dictos = Get-Process dictos -ErrorAction SilentlyContinue
if (-not $dictos) { Write-Output "ERRO: daemon dictos nao esta rodando"; exit 2 }

$textFile = Join-Path $env:TEMP "dictos-e2e.txt"
Set-Content -Path $textFile -Value $Word -Encoding ASCII
$notepad = Start-Process notepad.exe -ArgumentList $textFile -PassThru
Start-Sleep -Seconds 3

$wshell = New-Object -ComObject WScript.Shell
$wshell.AppActivate($notepad.Id) | Out-Null
Start-Sleep -Milliseconds 800

# Seleciona tudo
[System.Windows.Forms.SendKeys]::SendWait("^a")
Start-Sleep -Milliseconds 800

# Retângulo da seleção via UIA (fonte de verdade da âncora)
$selRect = $null
try {
    $focused = [System.Windows.Automation.AutomationElement]::FocusedElement
    $tp = $focused.GetCurrentPattern([System.Windows.Automation.TextPattern]::Pattern)
    $rects = $tp.GetSelection()[0].GetBoundingRectangles()
    if ($rects.Count -gt 0) { $selRect = $rects[0] }
} catch {}
if (-not $selRect) { Write-Output "ERRO: nao consegui ler o retangulo da selecao via UIA"; exit 2 }
Write-Output ("selecao: x={0} y={1} w={2} h={3}" -f $selRect.Left, $selRect.Top, $selRect.Width, $selRect.Height)

$foregroundBefore = [Win32E2E]::GetForegroundWindow()

# Dispara o hotkey do DictOS
[System.Windows.Forms.SendKeys]::SendWait("^+d")
Start-Sleep -Seconds 3

$popupHwnd = [Win32E2E]::FindWindowOfProcess($dictos.Id)
Check "popup abriu" ($popupHwnd -ne [IntPtr]::Zero)

if ($popupHwnd -ne [IntPtr]::Zero) {
    $r = New-Object Win32E2E+RECT
    [Win32E2E]::GetWindowRect($popupHwnd, [ref]$r) | Out-Null
    Write-Output ("popup: x={0} y={1} w={2} h={3}" -f $r.Left, $r.Top, ($r.Right - $r.Left), ($r.Bottom - $r.Top))

    $tol = 60
    $nearX = [Math]::Abs($r.Left - $selRect.Left) -le $tol
    $nearY = [Math]::Abs($r.Top - ($selRect.Top + $selRect.Height)) -le $tol -or `
             [Math]::Abs($r.Bottom - $selRect.Top) -le $tol
    Check "popup junto a selecao (x)" $nearX
    Check "popup junto a selecao (y)" $nearY

    Check "foco ficou no Notepad" ([Win32E2E]::GetForegroundWindow() -eq $foregroundBefore)

    # Screenshot antes de fechar
    $bounds = [System.Windows.Forms.Screen]::PrimaryScreen.Bounds
    $bitmap = New-Object System.Drawing.Bitmap($bounds.Width, $bounds.Height)
    $graphics = [System.Drawing.Graphics]::FromImage($bitmap)
    $graphics.CopyFromScreen($bounds.Location, [System.Drawing.Point]::Empty, $bounds.Size)
    $shot = Join-Path $env:TEMP "dictos-e2e.png"
    $bitmap.Save($shot, [System.Drawing.Imaging.ImageFormat]::Png)
    $graphics.Dispose(); $bitmap.Dispose()
    Write-Output "screenshot: $shot"

    # Esc fecha o popup
    [System.Windows.Forms.SendKeys]::SendWait("{ESC}")
    Start-Sleep -Milliseconds 800
    Check "Esc fechou o popup" (-not [Win32E2E]::IsWindowVisible($popupHwnd))
}

$notepad.CloseMainWindow() | Out-Null
Start-Sleep -Milliseconds 500
if (-not $notepad.HasExited) { $notepad.Kill() }

if ($failures.Count -gt 0) { Write-Output ("RESULTADO: FALHOU (" + ($failures -join ", ") + ")"); exit 1 }
Write-Output "RESULTADO: OK"
