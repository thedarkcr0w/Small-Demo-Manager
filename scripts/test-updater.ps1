param(
    [string]$Package = (Join-Path $PSScriptRoot '../dist/SmallDemoManager-native-windows-x64.zip'),
    [string]$ArtifactsDirectory = (Join-Path ([IO.Path]::GetTempPath()) 'SmallDemoManager-tests'),
    [switch]$KeepReplacementOpen
)
$ErrorActionPreference = 'Stop'
$ArtifactsDirectory = $ExecutionContext.SessionState.Path.GetUnresolvedProviderPathFromPSPath($ArtifactsDirectory)
$root = Join-Path $ArtifactsDirectory ('updater-' + [guid]::NewGuid().ToString('N'))
$install = Join-Path $root 'app directory & space'
$stage = Join-Path $root 'update stage'
$payload = Join-Path $stage 'payload'
$data = Join-Path $root 'user data'
New-Item -ItemType Directory -Path $install,$stage,$data -Force | Out-Null
Expand-Archive -LiteralPath $Package -DestinationPath $payload
$exe = Join-Path $install 'SmallDemoManager.exe'
Copy-Item -LiteralPath (Join-Path $payload 'SmallDemoManager.exe') -Destination $exe
# A harmless PE overlay distinguishes the old test installation from the payload.
# Neither the source ZIP nor any user's installed executable is modified.
$file = [IO.File]::Open($exe, [IO.FileMode]::Append, [IO.FileAccess]::Write, [IO.FileShare]::None)
try { $marker = [Text.Encoding]::UTF8.GetBytes("`nIsolated old-installation fixture`n"); $file.Write($marker,0,$marker.Length) } finally { $file.Dispose() }
$oldHash = (Get-FileHash -LiteralPath $exe -Algorithm SHA256).Hash.ToLowerInvariant()
$helper = Join-Path $stage 'updater.exe'
Copy-Item -LiteralPath $exe -Destination $helper
[IO.File]::WriteAllText((Join-Path $install 'README.md'), 'Previous installation documentation')
[IO.File]::WriteAllText((Join-Path $install 'my-private-notes.txt'), 'Do not replace this unmanaged file')
New-Item -ItemType Directory -Path (Join-Path $data 'Audio.native') | Out-Null
[IO.File]::WriteAllBytes((Join-Path $data 'Audio.native/keep.bin'), [byte[]]@(1,2,3,4,5))
$library = [ordered]@{
    schema = 1
    settings = [ordered]@{cs2_path='';move_files=$false;backup_before_move=$true;background_parse=$false;check_for_updates=$false;layout='Hybrid'}
    folders = @()
    demos = @{ kept_annotation = @{id='kept_annotation';path=(Join-Path $root 'missing.dem');favorite=$true;tags=@('keep-this');note='Keep these original annotations'} }
}
[IO.File]::WriteAllText((Join-Path $data 'Library.native.json'), ($library | ConvertTo-Json -Depth 12))
$audioHash = (Get-FileHash -LiteralPath (Join-Path $data 'Audio.native/keep.bin') -Algorithm SHA256).Hash
$previousDataDirectory = $env:SDM_DATA_DIR
$env:SDM_DATA_DIR = $data
$parent = $null
$updater = $null
$replacement = $null
$successful = $false
try {
    $parent = Start-Process -FilePath $exe -PassThru -RedirectStandardError (Join-Path $root 'parent.stderr.txt') -RedirectStandardOutput (Join-Path $root 'parent.stdout.txt')
    $null = $parent.Handle
    [void]$parent.WaitForInputIdle(15000)
    # winit first creates an untitled bootstrap HWND. Wait for the real window.
    $windowTimer = [Diagnostics.Stopwatch]::StartNew()
    do {
        $parent.Refresh()
        if ($parent.HasExited -or $windowTimer.Elapsed.TotalSeconds -gt 15) { throw 'The isolated old app did not open its actual window' }
        if ($parent.MainWindowHandle -ne [IntPtr]::Zero -and $parent.MainWindowTitle -eq 'Small Demo Manager') { break }
        Start-Sleep -Milliseconds 50
    } while ($true)
    $manifest = Get-Content -LiteralPath (Join-Path $payload 'update-manifest.json') -Raw | ConvertFrom-Json
    $files = @(Get-ChildItem -LiteralPath $payload -Recurse -File | ForEach-Object {
        [ordered]@{path=$_.FullName.Substring($payload.Length+1).Replace('\','/');sha256=(Get-FileHash -LiteralPath $_.FullName -Algorithm SHA256).Hash.ToLowerInvariant();size=$_.Length}
    } | Sort-Object { $_.path -eq 'SmallDemoManager.exe' })
    $plan = [ordered]@{schema=1;parent_pid=$parent.Id;root=$stage;executable=$exe;data_dir=$data;startup_demo=$null;version=$manifest.version;previous_sha256=$oldHash;files=$files}
    $planPath = Join-Path $stage 'install.json'
    [IO.File]::WriteAllText($planPath, ($plan | ConvertTo-Json -Depth 12))
    $updater = Start-Process -FilePath $helper -ArgumentList @('--apply-update', ('"' + $planPath + '"')) -PassThru -RedirectStandardError (Join-Path $root 'helper.stderr.txt') -RedirectStandardOutput (Join-Path $root 'helper.stdout.txt')
    $null = $updater.Handle
    $timer = [Diagnostics.Stopwatch]::StartNew()
    while (-not (Test-Path -LiteralPath (Join-Path $stage 'helper-ready'))) {
        $updater.Refresh()
        if ($updater.HasExited -or $timer.Elapsed.TotalSeconds -gt 15) { throw 'The helper did not signal readiness' }
        Start-Sleep -Milliseconds 30
    }
    if ((Get-FileHash -LiteralPath $exe -Algorithm SHA256).Hash.ToLowerInvariant() -ne $oldHash -or (Test-Path -LiteralPath (Join-Path $stage 'backup'))) { throw 'The updater modified files before the parent closed' }
    if (-not $parent.CloseMainWindow()) { throw 'Could not close the isolated old app' }
    if (-not $parent.WaitForExit(15000)) { throw 'The isolated old app did not exit' }
    if (-not $updater.WaitForExit(60000)) {
        $errorPath = Join-Path $stage 'error.txt'
        $details = if (Test-Path -LiteralPath $errorPath) { Get-Content -LiteralPath $errorPath -Raw } else { 'No helper error log' }
        throw "Helper timeout: $details"
    }
    if ($null -ne $updater.ExitCode -and $updater.ExitCode -ne 0) { throw "Helper failed: $(Get-Content -LiteralPath (Join-Path $stage 'error.txt') -Raw -ErrorAction SilentlyContinue)" }
    $started = Get-Content -LiteralPath (Join-Path $stage 'started.json') -Raw | ConvertFrom-Json
    $replacement = Get-Process -Id $started.pid
    $replacement.Refresh()
    if ($replacement.Path -ne $exe -or $replacement.HasExited) { throw 'Replacement process did not stay running at the approved path' }
    $newHash = (Get-FileHash -LiteralPath $exe -Algorithm SHA256).Hash.ToLowerInvariant()
    if ($newHash -ne $manifest.executable_sha256 -or $newHash -eq $oldHash) { throw 'The actual executable bytes were not updated' }
    if ((Get-FileHash -LiteralPath (Join-Path $stage 'backup/SmallDemoManager.exe') -Algorithm SHA256).Hash.ToLowerInvariant() -ne $oldHash) { throw 'Old executable backup does not verify' }
    if ((Get-Content -LiteralPath (Join-Path $install 'my-private-notes.txt') -Raw) -ne 'Do not replace this unmanaged file') { throw 'Unmanaged install file was changed' }
    $saved = Get-Content -LiteralPath (Join-Path $data 'Library.native.json') -Raw | ConvertFrom-Json
    if ($saved.demos.kept_annotation.note -ne 'Keep these original annotations' -or -not $saved.demos.kept_annotation.favorite -or $saved.demos.kept_annotation.tags[0] -ne 'keep-this') { throw 'Library annotations were lost' }
    if ((Get-FileHash -LiteralPath (Join-Path $data 'Audio.native/keep.bin') -Algorithm SHA256).Hash -ne $audioHash) { throw 'Saved audio was changed' }
    if (-not (Test-Path -LiteralPath (Join-Path $stage 'installed.txt'))) { throw 'No installation completion marker' }
    $successful = $true
    [PSCustomObject]@{Fixture=$root;ManagedFiles=$files.Count;WaitedForParent=$true;ExecutableBytesChanged=$true;PreviousExecutableBackupVerified=$true;ReplacementStartupAcknowledged=$true;UserAnnotationsPreserved=$true;SavedAudioUnchanged=$true;UnmanagedInstallFileUnchanged=$true;HandledSpacesAndAmpersand=$true;ReplacementPID=$replacement.Id}
} finally {
    $env:SDM_DATA_DIR = $previousDataDirectory
    $ack = Join-Path $stage 'started.json'
    if ($null -eq $replacement -and (Test-Path -LiteralPath $ack)) {
        $id = (Get-Content -LiteralPath $ack -Raw | ConvertFrom-Json).pid
        $candidate = Get-Process -Id $id -ErrorAction SilentlyContinue
        if ($candidate -and $candidate.Path -eq $exe) { $replacement = $candidate }
    }
    # Only processes created by this fixture are closed. Artifacts stay for review.
    foreach ($process in @($replacement,$parent,$updater)) {
        if ($null -eq $process) { continue }
        if ($successful -and $KeepReplacementOpen -and $replacement -and $process.Id -eq $replacement.Id) { continue }
        $process.Refresh()
        if (-not $process.HasExited) {
            [void]$process.CloseMainWindow()
            if (-not $process.WaitForExit(5000)) { $process.Kill(); $process.WaitForExit() }
        }
    }
}
