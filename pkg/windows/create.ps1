# Zip release binaries + configs from cfg/ (+ LICENSE, README) into artifacts/.
# Set SINKD_EXE / SINKD_SRV_EXE to prebuilt paths to skip cargo; otherwise Rust + bump on PATH. PowerShell 5+.
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

$cfgSystem = Join-Path $Root 'cfg\system\sinkd.conf'
$cfgUser = Join-Path $Root 'cfg\user\sinkd.conf'
foreach ($p in @($cfgSystem, $cfgUser)) { if (-not (Test-Path $p)) { throw "Missing $p" } }

function Resolve-BuiltExe {
  param([string]$Name)
  $cargo = Get-Command cargo -ErrorAction SilentlyContinue
  if (-not $cargo) { throw 'cargo not found in PATH (set SINKD_EXE / SINKD_SRV_EXE to skip build)' }
  $Triple = $env:CARGO_BUILD_TARGET
  if ($Triple) {
    & cargo build -p sinkd -p sinkd-srv --release --locked --target $Triple
  } else {
    & cargo build -p sinkd -p sinkd-srv --release --locked
  }
  $cargoTarget = $env:CARGO_TARGET_DIR
  if (-not $cargoTarget) { $cargoTarget = Join-Path $Root 'target' }
  if ($Triple) {
    [IO.Path]::Combine($cargoTarget, $Triple, 'release', $Name)
  } else {
    Join-Path $cargoTarget "release\$Name"
  }
}

$exe = $env:SINKD_EXE
if ($exe -and (Test-Path $exe)) {
  $exe = (Resolve-Path $exe).Path
} else {
  $exe = Resolve-BuiltExe -Name 'sinkd.exe'
  if (-not (Test-Path $exe)) { throw "Expected $exe after build" }
}

$srvExe = $env:SINKD_SRV_EXE
if ($srvExe -and (Test-Path $srvExe)) {
  $srvExe = (Resolve-Path $srvExe).Path
} else {
  $exeDir = Split-Path -Parent $exe
  $srvExe = Join-Path $exeDir 'sinkd-srv.exe'
  if (-not (Test-Path $srvExe)) {
    throw "Expected $srvExe next to sinkd.exe (same directory) or set SINKD_SRV_EXE"
  }
}

$arch = if ($env:PROCESSOR_ARCHITECTURE -eq 'ARM64') { 'arm64' } else { 'amd64' }
$staging = Join-Path $env:TEMP ("sinkd-win-{0}" -f [Guid]::NewGuid().ToString('N'))
New-Item -ItemType Directory -Force -Path $staging | Out-Null
try {
  Copy-Item $exe (Join-Path $staging 'sinkd.exe')
  Copy-Item $srvExe (Join-Path $staging 'sinkd-srv.exe')
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
