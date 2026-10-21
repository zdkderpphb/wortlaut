# Signing the installers

Unsigned installers work, but macOS and Windows warn on first launch (the README
explains how users get past that). Signing removes the warning.

## macOS (Developer ID + notarization)

Needs an [Apple Developer Program](https://developer.apple.com/programs/) membership.

1. In Xcode or on developer.apple.com create a **Developer ID Application** certificate
   and export it with its private key as a `.p12` file (with a password).
2. Create an [app-specific password](https://support.apple.com/102654) for your Apple ID.
3. Add these repository secrets (*Settings → Secrets and variables → Actions*):

   | Secret | Value |
   | --- | --- |
   | `APPLE_CERTIFICATE` | `base64 -i certificate.p12 \| pbcopy`, then paste |
   | `APPLE_CERTIFICATE_PASSWORD` | the `.p12` password |
   | `APPLE_SIGNING_IDENTITY` | e.g. `Developer ID Application: Your Name (TEAMID)` |
   | `APPLE_ID` | your Apple ID e-mail |
   | `APPLE_PASSWORD` | the app-specific password |
   | `APPLE_TEAM_ID` | your 10-character team ID |

The release workflow detects `APPLE_CERTIFICATE` and then signs and notarizes the app and DMG.

## Windows

The Microsoft Store version is signed by Microsoft on upload – see [STORE.md](STORE.md).

The installer on the release page is unsigned for now. Windows trusts installers signed with a
code-signing certificate, for example from
[Azure Artifact Signing](https://learn.microsoft.com/azure/artifact-signing/) (organisations) or,
for open-source projects, the [SignPath Foundation](https://signpath.org/). Tauri calls any
signing tool through `bundle.windows.signCommand`; see the
[Tauri guide on Windows code signing](https://v2.tauri.app/distribute/sign/windows/).
