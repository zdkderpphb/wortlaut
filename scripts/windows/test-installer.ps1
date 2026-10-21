<#
  CI check of what users download: installs the setup silently, runs the app's
  self-test and starts it once. Afterwards it removes the app's data folder so a
  later test (test-msix.ps1) starts clean.
#>
param([Parameter(Mandatory)] [string] $Setup)

. "$PSScriptRoot\common.ps1"

Start-Process -FilePath $Setup -ArgumentList "/S" -Wait
$dir = "$env:LOCALAPPDATA\$($TauriConfig.productName)"
Get-ChildItem $dir | Format-Table Name, Length
$exe = Get-ChildItem "$dir\*.exe" | Where-Object { $_.Name -notlike "uninstall*" } | Select-Object -First 1

$report = "$env:RUNNER_TEMP\self-test-installer.txt"
$p = Start-Process -FilePath $exe.FullName -ArgumentList "--self-test", $report -Wait -PassThru
Get-Content $report
if ($p.ExitCode -ne 0) { throw "Self-test failed with exit code $($p.ExitCode)." }

$data = "$env:LOCALAPPDATA\$($TauriConfig.identifier)"
Test-AppStart -Exe $exe.FullName -LogDir "$data\logs"
Remove-Item $data -Recurse -Force
