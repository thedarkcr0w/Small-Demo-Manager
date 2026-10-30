param([Parameter(Mandatory=$true)][string]$Destination, [switch]$Bundled)
$ErrorActionPreference = 'Stop'
$hostLine = rustc -vV | Select-String '^host: '
if (-not $hostLine) { throw 'Cannot determine the Rust build target' }
$buildTarget = if ($env:CARGO_BUILD_TARGET) { $env:CARGO_BUILD_TARGET } else { $hostLine.ToString().Substring(6).Trim() }
$metadata = cargo metadata --locked --format-version 1 --filter-platform $buildTarget | ConvertFrom-Json
if ($LASTEXITCODE -ne 0) { throw 'Cannot resolve dependency license metadata' }
$ids = New-Object 'System.Collections.Generic.HashSet[string]'
$queue = New-Object 'System.Collections.Generic.Queue[string]'
$nodes = @{}
foreach ($node in $metadata.resolve.nodes) { $nodes[$node.id] = $node }
$queue.Enqueue($metadata.resolve.root)
while ($queue.Count) {
    $id = $queue.Dequeue()
    if (-not $ids.Add($id)) { continue }
    foreach ($dependency in $nodes[$id].deps) { $queue.Enqueue($dependency.pkg) }
}
$licenseRoot = Join-Path $PSScriptRoot '../LICENSES'
$supplements = Get-Content -LiteralPath (Join-Path $licenseRoot 'dependency-sources.json') -Raw | ConvertFrom-Json
New-Item -ItemType Directory -Path $Destination -Force | Out-Null
$index = New-Object 'System.Collections.Generic.List[string]'
$index.Add("# Locked dependency licenses - $buildTarget")
$index.Add('')
$index.Add('These components retain their own licenses, not the darkcr0w personal-use terms. This list follows the locked dependency graph for the Windows build target, including its build-time dependencies. Unmodified source is available from the exact crates.io versions linked below; `cargo fetch --locked` also retrieves it. Symphonia source is provided under MPL-2.0 at those links. The modified parser source is in the project repository.')
$index.Add($(if ($Bundled) { 'Full dependency, font, compiler/runtime notices and supplemental provenance are consolidated in ../../THIRD-PARTY-NOTICES.md. This index is retained at its original path for compatibility with the 2.0.0 updater.' } else { 'Supplemental upstream license sources and revisions are recorded alongside their texts; compiler/runtime notices are in ../toolchain/.' }))
$index.Add('')
$index.Add('| Component | Declared license | Source | Included notices |')
$index.Add('|---|---|---|---|')
$missing = New-Object 'System.Collections.Generic.List[string]'
foreach ($package in ($metadata.packages | Where-Object { $ids.Contains($_.id) -and ($_.source -or $_.name -eq 'source2-demo') } | Sort-Object name,version)) {
    $root = Split-Path $package.manifest_path
    $id = $package.name + '-' + $package.version
    $notices = @(Get-ChildItem -LiteralPath $root -Recurse -File | Where-Object { $_.Name -match '^(LICEN[CS]E|COPYING|COPYRIGHT|NOTICE|OFL|UFL)' })
    if ($package.license_file -and (Test-Path -LiteralPath $package.license_file)) {
        $notices += Get-Item -LiteralPath $package.license_file
    }
    $notices = @($notices | Sort-Object FullName -Unique)
    foreach ($file in $notices) {
        $relative = $file.FullName.Substring($root.Length).TrimStart([char[]]@('\','/'))
        $noticeDestination = Join-Path (Join-Path $Destination $id) $relative
        New-Item -ItemType Directory -Path (Split-Path $noticeDestination) -Force | Out-Null
        Copy-Item -LiteralPath $file.FullName -Destination $noticeDestination
    }
    $extra = @()
    $supplement = $supplements.PSObject.Properties[$id]
    if ($supplement) {
        $sourceDirectory = Join-Path $licenseRoot $supplement.Value
        if (-not (Test-Path -LiteralPath $sourceDirectory -PathType Container)) { throw "Missing checked-in license supplement for $id" }
        $extra = @(Get-ChildItem -LiteralPath $sourceDirectory -File)
        foreach ($file in $extra) {
            $noticeDestination = Join-Path (Join-Path $Destination $id) ('upstream/' + $file.Name)
            New-Item -ItemType Directory -Path (Split-Path $noticeDestination) -Force | Out-Null
            Copy-Item -LiteralPath $file.FullName -Destination $noticeDestination
        }
    }
    $source = if ($package.name -eq 'source2-demo') { 'https://github.com/thedarkcr0w/Small-Demo-Manager/tree/main/vendor/source2-demo' } else { 'https://crates.io/crates/' + $package.name + '/' + $package.version }
    $license = if ($package.license) { $package.license } else { 'See upstream license file' }
    $count = $notices.Count + @($extra | Where-Object { $_.Name -match '^(LICEN[CS]E|COPYING|COPYRIGHT|NOTICE|OFL|UFL|MIT|Apache|CC0)' }).Count
    $included = if ($count) {
        if ($Bundled) { "[$count notices](../../THIRD-PARTY-NOTICES.md)" } else { "[$count files]($id/)" }
    } else { 'MISSING — package build must fail' }
    if (-not $count) { $missing.Add($id) }
    $index.Add("| $id | $license | [source]($source) | $included |")
}
[IO.File]::WriteAllLines((Join-Path $Destination 'INDEX.md'), $index)
if ($missing.Count) {
    throw ('Missing dependency license texts. Add a reviewed supplement before packaging: ' + ($missing -join ', '))
}
