param(
    [Parameter(Mandatory=$true)][string]$Executable,
    [Parameter(Mandatory=$true)][string]$FixtureDirectory,
    [ValidateRange(2,120)][int]$Seconds = 12,
    [switch]$AllowBlocked
)
$ErrorActionPreference = 'Stop'
$Executable = (Resolve-Path -LiteralPath $Executable).Path
$FixtureDirectory = (Resolve-Path -LiteralPath $FixtureDirectory).Path
$marker = Join-Path $FixtureDirectory 'STARTUP-BENCHMARK.txt'
if (-not (Test-Path -LiteralPath $marker) -or (Get-Content -LiteralPath $marker -Raw).Trim() -cne 'Small Demo Manager synthetic startup fixture') {
    throw 'Only a retained fixture from tests/startup_performance.rs may be used. Normal user data is not a test fixture.'
}
$data = Join-Path $FixtureDirectory 'benchmark-only-data'
if (-not (Test-Path -LiteralPath (Join-Path $data 'Library.native.json'))) { throw 'The synthetic library is missing' }
if (-not ('SdmStartupProbe' -as [type])) {
    Add-Type @'
using System;
using System.Runtime.InteropServices;
public static class SdmStartupProbe {
    [DllImport("user32.dll", SetLastError=true)]
    public static extern IntPtr SendMessageTimeout(IntPtr window, uint message, UIntPtr w, IntPtr l, uint flags, uint timeout, out UIntPtr result);
}
'@
}
$previousDataDirectory = $env:SDM_DATA_DIR
$env:SDM_DATA_DIR = $data
$log = Join-Path $FixtureDirectory ('window-' + [guid]::NewGuid().ToString('N'))
$process = $null
try {
    $process = Start-Process -FilePath $Executable -PassThru -RedirectStandardOutput ($log + '.stdout.txt') -RedirectStandardError ($log + '.stderr.txt')
    $null = $process.Handle
    $timer = [Diagnostics.Stopwatch]::StartNew()
    $samples = 0
    $blocked = 0
    $maxDelay = 0.0
    $firstWindowMs = $null
    while ($timer.Elapsed.TotalSeconds -lt $Seconds) {
        $process.Refresh()
        if ($process.HasExited) { throw "App exited during startup: $($process.ExitCode)" }
        $handle = $process.MainWindowHandle
        if ($handle -ne [IntPtr]::Zero -and $process.MainWindowTitle -eq 'Small Demo Manager') {
            if ($null -eq $firstWindowMs) { $firstWindowMs = $timer.Elapsed.TotalMilliseconds }
            $response = [UIntPtr]::Zero
            $call = [Diagnostics.Stopwatch]::StartNew()
            $ok = [SdmStartupProbe]::SendMessageTimeout($handle, 0, [UIntPtr]::Zero, [IntPtr]::Zero, 3, 150, [ref]$response)
            $maxDelay = [Math]::Max($maxDelay, $call.Elapsed.TotalMilliseconds)
            $samples++
            if ($ok -eq [IntPtr]::Zero) { $blocked++ }
        }
        Start-Sleep -Milliseconds 100
    }
    $result = [PSCustomObject]@{Executable=$Executable;FixtureDirectory=$FixtureDirectory;FirstWindowMs=$firstWindowMs;Samples=$samples;BlockedSamples=$blocked;MaximumHeartbeatMs=$maxDelay;Responding=$process.Responding}
    [IO.File]::WriteAllText(($log + '.json'), ($result | ConvertTo-Json))
    $result
    if ($samples -eq 0) { throw 'No app window became available during the startup test' }
    if (-not $AllowBlocked -and ($blocked -gt 0 -or -not $result.Responding)) { throw 'The app window blocked during startup; see the retained timing report' }
} finally {
    $env:SDM_DATA_DIR = $previousDataDirectory
    # Only the process launched for this synthetic fixture may be closed.
    if ($process) {
        $process.Refresh()
        if (-not $process.HasExited) {
            [void]$process.CloseMainWindow()
            if (-not $process.WaitForExit(5000)) { $process.Kill(); $process.WaitForExit() }
        }
    }
}
