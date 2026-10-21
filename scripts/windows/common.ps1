# Shared helpers for the Windows packaging scripts (dot-source this file).

$ErrorActionPreference = "Stop"
$PSNativeCommandUseErrorActionPreference = $true

$RepoRoot = (Resolve-Path "$PSScriptRoot\..\..").Path
$TauriDir = "$RepoRoot\src-tauri"
$TauriConfig = Get-Content "$TauriDir\tauri.conf.json" -Raw | ConvertFrom-Json

# Newest copy of a Windows SDK tool such as makeappx.exe.
function Find-SdkTool([string] $Name) {
  $tool = Get-ChildItem "${env:ProgramFiles(x86)}\Windows Kits\10\bin\*\x64\$Name" -ErrorAction SilentlyContinue |
    Where-Object { $_.Directory.Parent.Name -match '^\d+(\.\d+)+$' } |
    Sort-Object { [version]$_.Directory.Parent.Name } -Descending |
    Select-Object -First 1
  if (-not $tool) { throw "$Name not found – install the Windows SDK." }
  $tool.FullName
}

# Starts Wortlaut, waits for its log file and checks it is still running
# afterwards – a missing DLL or a broken WebView ends the process early.
function Test-AppStart([string] $Exe, [string] $LogDir) {
  $app = Start-Process -FilePath $Exe -PassThru
  $null = $app.Handle # keeps the exit code readable after the process ends
  try {
    $deadline = (Get-Date).AddSeconds(90)
    while (-not (Test-Path "$LogDir\*.log") -and -not $app.HasExited -and (Get-Date) -lt $deadline) {
      Start-Sleep -Seconds 2
    }
    Start-Sleep -Seconds 10
    if ($app.HasExited) { throw "Wortlaut quit right after starting (exit code $($app.ExitCode))." }
    if (-not (Test-Path "$LogDir\*.log")) { throw "No log file in $LogDir." }
    Get-Content "$LogDir\*.log"
    $app.Refresh()
    "Running, window title: '$($app.MainWindowTitle)'"
  } finally {
    if (-not $app.HasExited) { Stop-Process -Id $app.Id -Force }
  }
}
