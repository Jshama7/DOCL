param(
  [string]$OutDir = "promo\shots",
  [int]$Width = 1920,
  [int]$Height = 1080,
  [string]$ProcessName = "mc-launcher",
  [string]$Only = "",
  [switch]$NoResize,
  [switch]$NoSwitch,
  [switch]$Themes
)

# Batch screenshot helper for the DOCL promo.
# Requires the dev window running (pnpm tauri dev) and dev-only hotkeys:
#   F1-F7 pages, F8 log panel, F9/F10 mask UUIDs, Ctrl+F3 community downloads,
#   Shift+F1..F5 / Shift+F8 theme + wallpaper looks, Shift+F7 restore user config.
# Captures the client area at native pixels.

Add-Type -AssemblyName System.Drawing
Add-Type -AssemblyName System.Windows.Forms

Add-Type @"
using System;
using System.Runtime.InteropServices;
public class DoclWin32 {
  [DllImport("user32.dll")] public static extern bool SetForegroundWindow(IntPtr hWnd);
  [DllImport("user32.dll")] public static extern IntPtr GetForegroundWindow();
  [DllImport("user32.dll")] public static extern uint GetWindowThreadProcessId(IntPtr hWnd, IntPtr processId);
  [DllImport("user32.dll")] public static extern bool AttachThreadInput(uint idAttach, uint idAttachTo, bool attach);
  [DllImport("kernel32.dll")] public static extern uint GetCurrentThreadId();
  [DllImport("user32.dll")] public static extern bool ShowWindow(IntPtr hWnd, int cmd);
  [DllImport("user32.dll")] public static extern bool SetWindowPos(IntPtr hWnd, IntPtr after, int x, int y, int cx, int cy, uint flags);
  [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr hWnd, out RECT rect);
  [DllImport("user32.dll")] public static extern bool GetClientRect(IntPtr hWnd, out RECT rect);
  [DllImport("user32.dll")] public static extern bool PrintWindow(IntPtr hWnd, IntPtr hdc, uint flags);
  [DllImport("user32.dll")] public static extern bool ClientToScreen(IntPtr hWnd, ref POINT point);
  [DllImport("user32.dll")] public static extern IntPtr GetDC(IntPtr hWnd);
  [DllImport("user32.dll")] public static extern int ReleaseDC(IntPtr hWnd, IntPtr hDC);
  [DllImport("gdi32.dll")] public static extern bool BitBlt(IntPtr dest, int x, int y, int w, int h, IntPtr src, int srcX, int srcY, uint rop);
  [DllImport("user32.dll")] public static extern bool BringWindowToTop(IntPtr hWnd);
  [DllImport("user32.dll")] public static extern bool RedrawWindow(IntPtr hWnd, IntPtr rect, IntPtr region, uint flags);
  [DllImport("dwmapi.dll")] public static extern int DwmFlush();
  [DllImport("user32.dll")] public static extern void keybd_event(byte vk, byte scan, uint flags, UIntPtr extra);
  [DllImport("user32.dll")] public static extern bool SetCursorPos(int x, int y);
  [DllImport("user32.dll")] public static extern bool GetCursorPos(out POINT point);
  [DllImport("user32.dll")] public static extern void mouse_event(uint flags, uint dx, uint dy, uint data, UIntPtr extra);
  [DllImport("user32.dll")] public static extern bool SetProcessDpiAwarenessContext(IntPtr value);
  [StructLayout(LayoutKind.Sequential)] public struct RECT { public int Left, Top, Right, Bottom; }
  [StructLayout(LayoutKind.Sequential)] public struct POINT { public int X, Y; }

  public static bool ForceForeground(IntPtr hwnd) {
    if (GetForegroundWindow() == hwnd) return true;
    IntPtr current = GetForegroundWindow();
    uint targetThread = GetWindowThreadProcessId(current, IntPtr.Zero);
    uint thisThread = GetCurrentThreadId();
    if (targetThread != thisThread) AttachThreadInput(thisThread, targetThread, true);
    // Alt tap releases the Windows foreground lock.
    keybd_event(0x12, 0, 0, UIntPtr.Zero);
    ShowWindow(hwnd, 9);
    BringWindowToTop(hwnd);
    bool ok = SetForegroundWindow(hwnd);
    keybd_event(0x12, 0, 2, UIntPtr.Zero);
    if (targetThread != thisThread) AttachThreadInput(thisThread, targetThread, false);
    return ok || GetForegroundWindow() == hwnd;
  }
}
"@

[void][DoclWin32]::SetProcessDpiAwarenessContext([IntPtr](-4))

$proc = Get-Process -Name $ProcessName -ErrorAction SilentlyContinue |
  Where-Object { $_.MainWindowHandle -ne 0 } |
  Select-Object -First 1

if (-not $proc) {
  Write-Host "DOCL window not found. Start it first: pnpm tauri dev" -ForegroundColor Red
  exit 1
}

$hwnd = $proc.MainWindowHandle
[void][DoclWin32]::ShowWindow($hwnd, 9)
Start-Sleep -Milliseconds 300
$outPath = Join-Path (Get-Location) $OutDir
if (-not (Test-Path $outPath)) { New-Item -ItemType Directory -Path $outPath | Out-Null }

function Get-RectInfo {
  $wr = New-Object DoclWin32+RECT
  $cr = New-Object DoclWin32+RECT
  [void][DoclWin32]::GetWindowRect($hwnd, [ref]$wr)
  [void][DoclWin32]::GetClientRect($hwnd, [ref]$cr)
  return @{
    Left = $wr.Left
    Top = $wr.Top
    FrameW = ($wr.Right - $wr.Left) - ($cr.Right - $cr.Left)
    FrameH = ($wr.Bottom - $wr.Top) - ($cr.Bottom - $cr.Top)
    ClientW = $cr.Right - $cr.Left
    ClientH = $cr.Bottom - $cr.Top
  }
}

if (-not $NoResize) {
  $info = Get-RectInfo
  $area = [System.Windows.Forms.Screen]::PrimaryScreen.WorkingArea
  $maxW = $area.Width - 40
  $maxH = $area.Height - 40
  $targetW = [Math]::Min($Width, $maxW)
  $targetH = [Math]::Min($Height, $maxH)
  if ($targetW -ne $Width -or $targetH -ne $Height) {
    Write-Host ("Screen limit reached: using {0}x{1} instead of {2}x{3}" -f $targetW, $targetH, $Width, $Height) -ForegroundColor Yellow
  }
  # Two passes: DWM frame math can differ from GetWindowRect, so correct against the real client size.
  for ($pass = 0; $pass -lt 3; $pass += 1) {
    $info = Get-RectInfo
    $dw = $targetW - $info.ClientW
    $dh = $targetH - $info.ClientH
    if ([Math]::Abs($dw) -le 1 -and [Math]::Abs($dh) -le 1) { break }
    $outerW = $info.ClientW + $info.FrameW + $dw
    $outerH = $info.ClientH + $info.FrameH + $dh
    [void][DoclWin32]::SetWindowPos($hwnd, [IntPtr]::Zero, 16, 16, $outerW, $outerH, 0x0014)
    Start-Sleep -Milliseconds 350
  }
  $info = Get-RectInfo
  Write-Host ("window client: {0}x{1}" -f $info.ClientW, $info.ClientH)
}

function Save-ClientShot([string]$name) {
  $info = Get-RectInfo
  $w = $info.ClientW
  $h = $info.ClientH

  # Refuse to touch the file unless the app is really in front; a covered WebView2 returns black frames.
  if ([DoclWin32]::GetForegroundWindow() -ne $hwnd) {
    Write-Host "ABORT: DOCL is not the foreground window, file not written" -ForegroundColor Red
    exit 1
  }

  # Park the cursor outside the window so caption buttons don't show hover highlights.
  $area = [System.Windows.Forms.Screen]::PrimaryScreen.WorkingArea
  [void][DoclWin32]::SetCursorPos($area.Right - 8, $area.Bottom - 8)
  [void][DoclWin32]::RedrawWindow($hwnd, [IntPtr]::Zero, [IntPtr]::Zero, 0x181)
  [void][DoclWin32]::DwmFlush()
  Start-Sleep -Milliseconds 400

  $bmp = New-Object System.Drawing.Bitmap($w, $h)
  $graphics = [System.Drawing.Graphics]::FromImage($bmp)
  $dest = $graphics.GetHdc()
  $ok = [DoclWin32]::PrintWindow($hwnd, $dest, 3)
  $graphics.ReleaseHdc($dest)
  $graphics.Dispose()

  # Blank-frame guard: sample the image, reject uniform output before overwriting the good file.
  $min = 255
  $max = 0
  $stepX = [Math]::Max(1, [int]($w / 40))
  $stepY = [Math]::Max(1, [int]($h / 24))
  for ($y = 0; $y -lt $h; $y += $stepY) {
    for ($x = 0; $x -lt $w; $x += $stepX) {
      $pixel = $bmp.GetPixel($x, $y)
      $lum = [int](($pixel.R + $pixel.G + $pixel.B) / 3)
      if ($lum -lt $min) { $min = $lum }
      if ($lum -gt $max) { $max = $lum }
    }
  }
  if (-not $ok -or ($max - $min) -lt 6) {
    $bmp.Dispose()
    Write-Host ("ABORT: blank/failed frame (ok={0}, range={1}), file not written" -f $ok, ($max - $min)) -ForegroundColor Red
    exit 1
  }

  $tmp = Join-Path $env:TEMP ("docl-shot-" + $name + ".png")
  $bmp.Save($tmp, [System.Drawing.Imaging.ImageFormat]::Png)
  $bmp.Dispose()
  $file = Join-Path $outPath ("{0}.png" -f $name)
  Move-Item -Path $tmp -Destination $file -Force
  Write-Host ("saved {0} ({1}x{2})" -f $file, $w, $h)
}

$shell = New-Object -ComObject WScript.Shell

function Focus-Window {
  [void]$shell.AppActivate($proc.Id)
  [void][DoclWin32]::ForceForeground($hwnd)
  Start-Sleep -Milliseconds 250

  if ([DoclWin32]::GetForegroundWindow() -ne $hwnd) {
    # Last resort: a real click on the title bar always activates the window.
    $rect = Get-RectInfo
    $saved = New-Object DoclWin32+POINT
    [void][DoclWin32]::GetCursorPos([ref]$saved)
    [void][DoclWin32]::SetCursorPos([int]($rect.Left + $rect.FrameW / 2 + 200), [int]($rect.Top + 14))
    [DoclWin32]::mouse_event(0x0002, 0, 0, 0, [UIntPtr]::Zero)
    [DoclWin32]::mouse_event(0x0004, 0, 0, 0, [UIntPtr]::Zero)
    Start-Sleep -Milliseconds 350
    [void][DoclWin32]::SetCursorPos($saved.X, $saved.Y)
  }
}

function Send-Key([string]$key) {
  Focus-Window
  [void]$shell.SendKeys($key)
  # Long enough for page transition plus any transient system overlay to fade.
  Start-Sleep -Milliseconds 2400
}

if ($NoSwitch) {
  Focus-Window
  Save-ClientShot "current"
  exit 0
}

# Theme looks: one home shot per built-in theme + wallpaper pair
if ($Themes) {
  Send-Key "{ESC}"
  Send-Key "{F1}"
  $looks = @(
    @{ key = "+{F1}"; name = "home-paper" },
    @{ key = "+{F2}"; name = "home-terracotta" },
    @{ key = "+{F3}"; name = "home-nocturne" },
    @{ key = "+{F4}"; name = "home-sakura" },
    @{ key = "+{F5}"; name = "home-ocean" },
    @{ key = "+{F8}"; name = "home-forge" }
  )
  foreach ($look in $looks) {
    if ($Only -and $look.name -ne $Only) { continue }
    Send-Key $look.key
    Save-ClientShot $look.name
  }
  # Restore the user's own theme so the app is not left in a capture-only look
  Send-Key "+{F7}"
  Write-Host ""
  Write-Host ("Done. Files in {0}" -f $outPath) -ForegroundColor Green
  exit 0
}

# Close any open modal first so shots are not covered by the log panel.
Send-Key "{ESC}"

# Mask account UUIDs for public screenshots (dev-only flag).
Send-Key "{F9}"

$shots = @(
  @{ key = "{F1}"; name = "01-home" },
  @{ key = "{F2}"; name = "02-versions" },
  @{ key = "^{F3}"; name = "03-download" },
  @{ key = "^{F4}"; name = "04-multiplayer" },
  @{ key = "{F5}"; name = "05-accounts" },
  @{ key = "{F6}"; name = "06-skins" },
  @{ key = "{F7}"; name = "07-settings" }
)

foreach ($shot in $shots) {
  if ($Only -and $shot.name -ne $Only) { continue }
  Send-Key $shot.key
  Save-ClientShot $shot.name
}

if (-not $Only) {
  # Log panel: seed demo lines, open, capture, close again.
  Send-Key "^{F8}"
  Save-ClientShot "08-log"
  [void]$shell.SendKeys("{ESC}")
  Start-Sleep -Milliseconds 400
  Send-Key "{F10}"
}

Write-Host ""
Write-Host ("Done. Files in {0}" -f $outPath) -ForegroundColor Green
