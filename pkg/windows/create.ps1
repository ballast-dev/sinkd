# Build sinkd and zip the release binary + configs from cfg/ (+ LICENSE, README) into artifacts/.
# Run on Windows with Rust and bump installed. PowerShell 5+.
$ErrorActionPreference = 'Stop'

$Root = (Resolve-Path (Join-Path $PSScriptRoot '..\..')).Path
Set-Location $Root

if (-not (Get-Command bump -ErrorAction SilentlyContinue)) {
  throw 'bump not found. Install bump (https://github.com/launchfirestorm/bump) and ensure it is on PATH.'
}

$Version = (& bump -b).Trim()
if (-not $Version) { throw 'bump -b returned empty version' }

$Artifacts = Join-Path $Root 'artifacts'
New-Item -ItemType Directory -Force -Path $Artifacts | Out-Null

$cargo = Get-Command cargo -ErrorAction SilentlyContinue
if (-not $cargo) { throw 'cargo not found in PATH' }

$cfgSystem = Join-Path $Root 'cfg\system\sinkd.conf'
$cfgUser = Join-Path $Root 'cfg\user\sinkd.conf'
foreach ($p in @($cfgSystem, $cfgUser)) { if (-not (Test-Path $p)) { throw "Missing $p" } }

cargo build -p sinkd --release --locked

$cargoTarget = $env:CARGO_TARGET_DIR
if (-not $cargoTarget) { $cargoTarget = Join-Path $Root 'target' }
$exe = Join-Path $cargoTarget 'release\sinkd.exe'
if (-not (Test-Path $exe)) { throw "Expected $exe after build" }

$arch = if ($env:PROCESSOR_ARCHITECTURE -eq 'ARM64') { 'arm64' } else { 'amd64' }
$staging = Join-Path $env:TEMP ("sinkd-win-{0}" -f [Guid]::NewGuid().ToString('N'))
New-Item -ItemType Directory -Force -Path $staging | Out-Null
try {
  Copy-Item $exe (Join-Path $staging 'sinkd.exe')
  Copy-Item $cfgSystem (Join-Path $staging 'sinkd.system.conf')
  Copy-Item $cfgUser (Join-Path $staging 'sinkd.user.conf')
  $license = Join-Path $Root 'LICENSE'
  if (Test-Path $license) { Copy-Item $license (Join-Path $staging 'LICENSE') }
  $readme = Join-Path $Root 'README.md'
  if (Test-Path $readme) { Copy-Item $readme (Join-Path $staging 'README.md') }

  $zipName = "sinkd-$Version-windows-$arch.zip"
  $zipPath = Join-Path $Artifacts $zipName
  if (Test-Path $zipPath) { Remove-Item -Force $zipPath }
  Compress-Archive -Path (Join-Path $staging '*') -DestinationPath $zipPath -CompressionLevel Optimal
  Write-Host "Wrote $zipPath"
}
finally {
  Remove-Item -Recurse -Force $staging -ErrorAction SilentlyContinue
}
