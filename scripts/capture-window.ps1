param(
    [string]$Title = "Z_COM",
    [int]$ProcessId = 0,
    [string]$Output = "artifacts/ui/original.png"
)

Add-Type -AssemblyName System.Drawing
Add-Type @'
using System;
using System.Runtime.InteropServices;
using System.Text;
public class WindowCapture {
    public delegate bool EnumCallback(IntPtr handle, IntPtr parameter);
    [DllImport("user32.dll")] public static extern bool EnumWindows(EnumCallback callback, IntPtr parameter);
    [DllImport("user32.dll")] public static extern uint GetWindowThreadProcessId(IntPtr handle, out uint processId);
    [DllImport("user32.dll", CharSet=CharSet.Unicode)] public static extern int GetWindowText(IntPtr handle, StringBuilder text, int capacity);
    public static IntPtr Find(int processId, string title) {
        IntPtr found = IntPtr.Zero;
        EnumWindows((handle, parameter) => {
            uint owner; GetWindowThreadProcessId(handle, out owner);
            var text = new StringBuilder(256); GetWindowText(handle, text, 256);
            if (owner == processId && text.ToString() == title) { found = handle; return false; }
            return true;
        }, IntPtr.Zero);
        return found;
    }
    [StructLayout(LayoutKind.Sequential)]
    public struct Rect { public int Left, Top, Right, Bottom; }
    [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr handle, out Rect rect);
    [DllImport("user32.dll")] public static extern bool PrintWindow(IntPtr handle, IntPtr destination, uint flags);
    [DllImport("user32.dll")] public static extern bool ShowWindow(IntPtr handle, int command);
}
'@
$process = if ($ProcessId) { Get-Process -Id $ProcessId } else { Get-Process -Name 'z-com' | Where-Object { $_.MainWindowHandle -ne 0 } | Select-Object -First 1 }
if (-not $process) { throw "Window not found: $Title" }
$windowHandle = [WindowCapture]::Find($process.Id, $Title)
if ($windowHandle -eq [IntPtr]::Zero) { throw "Window not found: $Title in process $($process.Id)" }
[WindowCapture]::ShowWindow($windowHandle, 9) | Out-Null
Start-Sleep -Milliseconds 250
$rect = New-Object WindowCapture+Rect
[WindowCapture]::GetWindowRect($windowHandle, [ref]$rect) | Out-Null
$bitmap = New-Object System.Drawing.Bitmap(($rect.Right - $rect.Left), ($rect.Bottom - $rect.Top))
$graphics = [System.Drawing.Graphics]::FromImage($bitmap)
$device = $graphics.GetHdc()
try {
    if (-not [WindowCapture]::PrintWindow($windowHandle, $device, 2)) { throw "PrintWindow failed" }
} finally {
    $graphics.ReleaseHdc($device)
    $graphics.Dispose()
}
$absoluteOutput = [System.IO.Path]::GetFullPath((Join-Path (Get-Location) $Output))
[System.IO.Directory]::CreateDirectory([System.IO.Path]::GetDirectoryName($absoluteOutput)) | Out-Null
$bitmap.Save($absoluteOutput, [System.Drawing.Imaging.ImageFormat]::Png)
$bitmap.Dispose()
Write-Output $absoluteOutput
