<#
.SYNOPSIS
  Set OpenSSL environment variables for Windows based on architecture.

.DESCRIPTION
  Checks the system's processor architecture. If x64, sets the Win64 paths;
  otherwise, sets the Win64-ARM paths. Variables are persisted at the User
  scope.
#>

# Microsoft security is annoying, run this to set env vars. 
# This is needed for rust-analyzer to build and work again sinkd
#
#     powershell.exe -ExecutionPolicy Bypass -File .\windows_env.ps1
#

# if ((Get-ExecutionPolicy) -eq "Restricted") {
#     Write-Host "Execution Policy is Restricted. Bypassing for this session."
#     powershell.exe -ExecutionPolicy Bypass -File $MyInvocation.MyCommand.Path
#     exit
# }

# Grab the architecture from PROCESSOR_ARCHITECTURE (e.g. "AMD64", "ARM64", "x86")
$arch = $env:PROCESSOR_ARCHITECTURE

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
