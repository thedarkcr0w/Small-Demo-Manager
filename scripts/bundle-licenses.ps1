param(
    [Parameter(Mandatory=$true)][string]$DependenciesDirectory,
    [Parameter(Mandatory=$true)][string]$Destination
)
$ErrorActionPreference = 'Stop'
$project = Split-Path $PSScriptRoot
$licenseRoot = Join-Path $project 'LICENSES'
$bundle = New-Object Text.StringBuilder
[void]$bundle.Append([IO.File]::ReadAllText((Join-Path $project 'THIRD-PARTY-NOTICES.md')))

function Add-Notice([string]$Label, [string]$File) {
    [void]$bundle.AppendLine()
    [void]$bundle.AppendLine()
    [void]$bundle.AppendLine(('----- BEGIN ' + $Label + ' -----'))
    # Keep the complete notice text, including HTML and upstream provenance.
    # Consolidation changes the container, not the third-party license terms.
    [void]$bundle.Append([IO.File]::ReadAllText($File))
    [void]$bundle.AppendLine()
    [void]$bundle.AppendLine(('----- END ' + $Label + ' -----'))
}

$packages = @(Get-ChildItem -LiteralPath $DependenciesDirectory -Directory | Sort-Object Name)
if (-not $packages.Count) { throw 'No collected dependency notices to bundle' }
foreach ($package in $packages) {
    foreach ($file in (Get-ChildItem -LiteralPath $package.FullName -Recurse -File | Sort-Object FullName)) {
        $relative = $file.FullName.Substring($DependenciesDirectory.Length).TrimStart([char[]]@('\','/')).Replace('\','/')
        Add-Notice -Label ('dependency/' + $relative) -File $file.FullName
    }
}
$toolchain = Join-Path $licenseRoot 'toolchain'
foreach ($file in (Get-ChildItem -LiteralPath $toolchain -Recurse -File | Sort-Object FullName)) {
    $relative = $file.FullName.Substring($toolchain.Length).TrimStart([char[]]@('\','/')).Replace('\','/')
    Add-Notice -Label ('toolchain/' + $relative) -File $file.FullName
}

New-Item -ItemType Directory -Path (Join-Path $Destination 'LICENSES/dependencies') -Force | Out-Null
Copy-Item -LiteralPath (Join-Path $project 'LICENSE') -Destination (Join-Path $Destination 'LICENSE')
Copy-Item -LiteralPath (Join-Path $licenseRoot 'Small-Demo-Manager-original-MIT.txt') -Destination (Join-Path $Destination 'LICENSES/Small-Demo-Manager-original-MIT.txt')
Copy-Item -LiteralPath (Join-Path $DependenciesDirectory 'INDEX.md') -Destination (Join-Path $Destination 'LICENSES/dependencies/INDEX.md')
[IO.File]::WriteAllText((Join-Path $Destination 'THIRD-PARTY-NOTICES.md'), $bundle.ToString(), (New-Object Text.UTF8Encoding $false))
Write-Output "Bundled notices for $($packages.Count) dependencies into one file."
