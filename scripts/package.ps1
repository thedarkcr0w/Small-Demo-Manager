param([switch]$SkipTests)
$ErrorActionPreference = 'Stop'
Push-Location (Join-Path $PSScriptRoot '..')
$stage = $null
try {
    cargo fmt --all -- --check
    if ($LASTEXITCODE -ne 0) { throw 'Formatting check failed' }
    if (-not $SkipTests) {
        cargo test --locked
        if ($LASTEXITCODE -ne 0) { throw 'Tests failed' }
    }
    cargo build --release --locked --bin SmallDemoManager
    if ($LASTEXITCODE -ne 0) { throw 'Build failed' }
    $out = Join-Path (Get-Location) 'dist'
    $stage = Join-Path $out ('native-' + [Guid]::NewGuid().ToString('N'))
    New-Item -ItemType Directory -Path $stage -Force | Out-Null
    $target = if ($env:CARGO_TARGET_DIR) { $env:CARGO_TARGET_DIR } else { 'target' }
    Copy-Item (Join-Path $target 'release/SmallDemoManager.exe') $stage
    $metadata = cargo metadata --locked --no-deps --format-version 1 | ConvertFrom-Json
    if ($LASTEXITCODE -ne 0) { throw 'Cannot read package version for update manifest' }
    $package = $metadata.packages | Where-Object { $_.name -eq 'small-demo-manager' } | Select-Object -First 1
    if (-not $package) { throw 'Native package metadata is missing' }
    $manifest = [ordered]@{
        schema = 1
        application = 'small-demo-manager'
        target = 'windows-x64'
        version = $package.version
        executable_sha256 = (Get-FileHash -LiteralPath (Join-Path $stage 'SmallDemoManager.exe') -Algorithm SHA256).Hash.ToLowerInvariant()
    }
    [IO.File]::WriteAllText((Join-Path $stage 'update-manifest.json'), ($manifest | ConvertTo-Json))
    Copy-Item 'README.md','PATCHNOTES.md','showcase.png','LICENSE','THIRD-PARTY-NOTICES.md' $stage
    Copy-Item 'LICENSES' (Join-Path $stage 'LICENSES') -Recurse
    Copy-Item 'docs' (Join-Path $stage 'docs') -Recurse
    & (Join-Path $PSScriptRoot 'licenses.ps1') -Destination (Join-Path $stage 'LICENSES/dependencies')
    # Registry archives can use pre-1980 timestamps, which ZIP cannot encode.
    # Normalize only our staged copies; upstream source files stay untouched.
    foreach ($item in (Get-ChildItem -LiteralPath $stage -Recurse -Force)) {
        if ($item.LastWriteTime.Year -lt 1980 -or $item.LastWriteTime.Year -gt 2107) {
            $item.LastWriteTime = [datetime]::new(2000, 1, 1)
        }
    }
    $archive = Join-Path $out 'SmallDemoManager-native-windows-x64.zip'
    Compress-Archive -Path (Join-Path $stage '*') -DestinationPath $archive -Force
    $hash = (Get-FileHash -LiteralPath $archive -Algorithm SHA256).Hash.ToLowerInvariant()
    [System.IO.File]::WriteAllText(($archive + '.sha256'), ($hash + '  ' + [System.IO.Path]::GetFileName($archive) + "`n"))
    Write-Output $archive
    Write-Output "SHA256 $hash"
} finally {
    # Only this invocation's uniquely named staging directory is removed.
    if ($stage -and (Test-Path -LiteralPath $stage)) {
        Remove-Item -LiteralPath $stage -Recurse -Force
    }
    Pop-Location
}
