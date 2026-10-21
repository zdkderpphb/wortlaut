<#
  CI check of the Store package: signs a copy with a throwaway certificate,
  installs it, runs the app's self-test inside the package and starts it once.
  It trusts that certificate machine-wide – don't run it on your own PC.
#>
param([string] $Package)

. "$PSScriptRoot\common.ps1"

if (-not $Package) { $Package = "$TauriDir\target\msix\Wortlaut-Store.msix" }
$tmp = New-Item -ItemType Directory (Join-Path ([IO.Path]::GetTempPath()) "wortlaut-msix-test") -Force

# The signing certificate has to name the package's publisher.
Add-Type -AssemblyName System.IO.Compression.FileSystem
$zip = [IO.Compression.ZipFile]::OpenRead((Resolve-Path $Package))
try {
  $reader = [IO.StreamReader]::new($zip.GetEntry("AppxManifest.xml").Open())
  [xml] $manifest = $reader.ReadToEnd()
  $reader.Dispose()
} finally {
  $zip.Dispose()
}
$identity = $manifest.Package.Identity

$cert = New-SelfSignedCertificate -Type Custom -Subject $identity.Publisher -KeyUsage DigitalSignature `
  -FriendlyName "Wortlaut CI test" -CertStoreLocation Cert:\CurrentUser\My `
  -TextExtension @("2.5.29.37={text}1.3.6.1.5.5.7.3.3", "2.5.29.19={text}")
Export-Certificate -Cert $cert -FilePath "$tmp\test.cer" | Out-Null
Import-Certificate -FilePath "$tmp\test.cer" -CertStoreLocation Cert:\LocalMachine\TrustedPeople | Out-Null
$password = ConvertTo-SecureString "test" -AsPlainText -Force
Export-PfxCertificate -Cert $cert -FilePath "$tmp\test.pfx" -Password $password | Out-Null

$signed = "$tmp\Wortlaut-test.msix"
Copy-Item $Package $signed
$signtool = Find-SdkTool "signtool.exe"
& $signtool sign /fd SHA256 /f "$tmp\test.pfx" /p test $signed

Add-AppxPackage -Path $signed
$installed = Get-AppxPackage -Name $identity.Name
"Installed $($installed.PackageFullName)"
try {
  # The execution alias runs the app inside its package.
  $exe = "$env:LOCALAPPDATA\Microsoft\WindowsApps\wortlaut.exe"
  $report = "$tmp\self-test.txt"
  $p = Start-Process -FilePath $exe -ArgumentList "--self-test", $report -Wait -PassThru
  Get-Content $report
  if ($p.ExitCode -ne 0) { throw "Self-test failed with exit code $($p.ExitCode)." }

  # Windows keeps the package's app data in its own folder (see src-tauri/src/packaged.rs).
  $data = "$env:LOCALAPPDATA\Packages\$($installed.PackageFamilyName)\LocalCache\Local\$($TauriConfig.identifier)"
  Test-AppStart -Exe $exe -LogDir "$data\logs"
} finally {
  Remove-AppxPackage -Package $installed.PackageFullName
}
