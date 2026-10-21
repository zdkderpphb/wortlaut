<#
  Builds the Microsoft Store package from a finished release build
  (run `npm run tauri build` first). It stays unsigned: the Store signs it on upload.

  The identity comes from Partner Center → Product identity, passed as parameters
  or as MSSTORE_IDENTITY_NAME, MSSTORE_PUBLISHER and MSSTORE_PUBLISHER_NAME.
  Without it the package gets a test identity – fine for test-msix.ps1, not for the Store.
#>
param(
  [string] $OutFile,
  [string] $IdentityName = $env:MSSTORE_IDENTITY_NAME,
  [string] $Publisher = $env:MSSTORE_PUBLISHER,
  [string] $PublisherDisplayName = $env:MSSTORE_PUBLISHER_NAME
)

. "$PSScriptRoot\common.ps1"

if (-not $IdentityName) {
  $IdentityName = "Wortlaut.Test"
  $Publisher = "CN=Wortlaut Test"
  $PublisherDisplayName = "Wortlaut"
  Write-Warning "No Store identity given – building a test package."
}
if (-not $Publisher -or -not $PublisherDisplayName) { throw "Publisher and publisher display name are needed too." }
if ($TauriConfig.version -notmatch '^\d+\.\d+\.\d+$') { throw "Store versions must be plain x.y.z, not '$($TauriConfig.version)'." }
# The Store reserves the fourth number for itself.
$version = "$($TauriConfig.version).0"

$work = "$TauriDir\target\msix"
$layout = "$work\layout"
if (-not $OutFile) { $OutFile = "$work\Wortlaut-Store.msix" }
Remove-Item $layout -Recurse -Force -ErrorAction SilentlyContinue
New-Item -ItemType Directory "$layout\Assets" -Force | Out-Null

# Same files the installer puts on disk: the app and the DLLs next to it.
Copy-Item "$TauriDir\target\release\wortlaut.exe" $layout
Copy-Item "$TauriDir\runtime\*.dll" $layout
Copy-Item "$TauriDir\msix\Assets\*" "$layout\Assets"

$values = @{
  "{{IDENTITY_NAME}}"          = $IdentityName
  "{{PUBLISHER}}"              = $Publisher
  "{{PUBLISHER_DISPLAY_NAME}}" = $PublisherDisplayName
  "{{VERSION}}"                = $version
}
$manifest = Get-Content "$TauriDir\msix\AppxManifest.xml" -Raw
foreach ($key in $values.Keys) {
  $manifest = $manifest.Replace($key, [System.Security.SecurityElement]::Escape($values[$key]))
}
Set-Content "$layout\AppxManifest.xml" $manifest -Encoding utf8NoBOM

# resources.pri lets Windows pick the right logo size (taskbar, Start, Store).
$makepri = Find-SdkTool "makepri.exe"
& $makepri createconfig /cf "$work\priconfig.xml" /dq de-DE_en-US /pv 10.0.0 /o
& $makepri new /pr $layout /cf "$work\priconfig.xml" /mn "$layout\AppxManifest.xml" /of "$layout\resources.pri" /o

$makeappx = Find-SdkTool "makeappx.exe"
& $makeappx pack /d $layout /p $OutFile /o

Get-Item $OutFile | Format-Table Name, Length
"Store package $version for $IdentityName ($PublisherDisplayName): $OutFile"
