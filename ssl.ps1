<#
.SYNOPSIS
  Set OpenSSL environment variables for Windows based on architecture.

.DESCRIPTION
  Checks the system's processor architecture. If x64, sets the Win64 paths;
  otherwise, sets the Win64-ARM paths. Variables are persisted at the User
  scope.
#>

$arch = $env:PROCESSOR_ARCHITECTURE
Write-Host "INFO: Ensure to download OpenSSL via winget"
Write-Host "INFO: winget install ShiningLight.OpenSSL.Dev"
Write-Hose ""
Write-Host "Detected architecture: $arch"

switch ($arch) {
  "AMD64" {
    [System.Environment]::SetEnvironmentVariable("OPENSSL_DIR", "C:\Program Files\OpenSSL-Win64", "User")
    [System.Environment]::SetEnvironmentVariable("OPENSSL_LIB_DIR", "C:\Program Files\OpenSSL-Win64\lib\VC\x64\MT", "User")
    [System.Environment]::SetEnvironmentVariable("OPENSSL_INCLUDE_DIR", "C:\Program Files\OpenSSL-Win64\include", "User")
    Write-Host "Set environment variables for x64."
  }
  "ARM64" {
    [System.Environment]::SetEnvironmentVariable("OPENSSL_DIR", "C:\Program Files\OpenSSL-Win64-ARM", "User")
    [System.Environment]::SetEnvironmentVariable("OPENSSL_LIB_DIR", "C:\Program Files\OpenSSL-Win64-ARM\lib\VC\arm64\MT", "User")
    [System.Environment]::SetEnvironmentVariable("OPENSSL_INCLUDE_DIR", "C:\Program Files\OpenSSL-Win64-ARM\include", "User")
    Write-Host "Set environment variables for ARM64."
  }
  default {
    Write-Host "Unknown or unsupported architecture: $arch"
  }
}

Write-Host "Done! You may need to open a new PowerShell session for the changes to take effect."