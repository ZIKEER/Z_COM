param([int]$ProcessId, [int]$X = 0, [int]$Y = 0, [int]$Width = 0, [int]$Height = 0)

Add-Type @'
using System;
using System.Runtime.InteropServices;
public class NativeInteraction {
    [DllImport("user32.dll")] public static extern bool SetProcessDPIAware();
    [DllImport("user32.dll")] public static extern bool SetForegroundWindow(IntPtr handle);
    [DllImport("user32.dll")] public static extern bool SetWindowPos(IntPtr handle, IntPtr after, int x, int y, int width, int height, uint flags);
    [DllImport("user32.dll")] public static extern bool PostMessage(IntPtr handle, uint message, IntPtr wparam, IntPtr lparam);
    [DllImport("user32.dll")] public static extern bool SetCursorPos(int x, int y);
    [DllImport("user32.dll")] public static extern bool ClientToScreen(IntPtr handle, ref Point point);
    [DllImport("user32.dll")] public static extern void mouse_event(uint flags, uint x, uint y, uint data, UIntPtr extra);
    public struct Point { public int X, Y; }
}
'@
$process = Get-Process -Id $ProcessId
[NativeInteraction]::SetProcessDPIAware() | Out-Null
if ($process.MainWindowHandle -eq 0) { throw "No window for process $ProcessId" }
[NativeInteraction]::SetForegroundWindow($process.MainWindowHandle) | Out-Null
if ($Width -gt 0 -and $Height -gt 0) {
    [NativeInteraction]::SetWindowPos($process.MainWindowHandle, [IntPtr]::Zero, 0, 0, $Width, $Height, 6) | Out-Null
} else {
    $point = New-Object NativeInteraction+Point
    $point.X = $X
    $point.Y = $Y
    [NativeInteraction]::ClientToScreen($process.MainWindowHandle, [ref]$point) | Out-Null
    [NativeInteraction]::SetCursorPos($point.X, $point.Y) | Out-Null
    Start-Sleep -Milliseconds 100
    $position = [IntPtr](($Y -shl 16) -bor ($X -band 65535))
    [NativeInteraction]::PostMessage($process.MainWindowHandle, 0x0200, [IntPtr]::Zero, $position) | Out-Null
    [NativeInteraction]::PostMessage($process.MainWindowHandle, 0x0201, [IntPtr]1, $position) | Out-Null
    Start-Sleep -Milliseconds 100
    [NativeInteraction]::PostMessage($process.MainWindowHandle, 0x0202, [IntPtr]::Zero, $position) | Out-Null
}
