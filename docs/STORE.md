# Microsoft Store

The Store version needs no code-signing certificate: Microsoft signs the package when you
upload it, so Windows shows no warning. The release workflow builds the package
(`scripts/windows/make-msix.ps1`), installs and tests it on a Windows runner
(`scripts/windows/test-msix.ps1`), and keeps it as a download on the workflow run.

## One-time setup

1. Register for free at [storedeveloper.microsoft.com](https://storedeveloper.microsoft.com).
   A company account shows the company as publisher; verification is fastest with a D-U-N-S
   number, otherwise upload a commercial register extract (a few business days). Use an
   e-mail address on the company's domain.
2. In [Partner Center](https://partner.microsoft.com/dashboard/apps-and-games/overview) create a
   new app and reserve the name **Wortlaut**.
3. Open *Product management → Product identity* and store three values as repository
   **variables** (*Settings → Secrets and variables → Actions → Variables*). They are public,
   not secrets:

   | Variable | Partner Center value |
   | --- | --- |
   | `MSSTORE_IDENTITY_NAME` | `Package/Identity/Name` |
   | `MSSTORE_PUBLISHER` | `Package/Identity/Publisher` (starts with `CN=`) |
   | `MSSTORE_PUBLISHER_NAME` | `Package/Properties/PublisherDisplayName` |

   The identity cannot change after the first accepted submission.

Without these variables the workflow still builds and tests a package with a test identity,
but doesn't keep it.

## Each release

1. Release as usual (see the README). When the workflow has finished, open the run under
   *Actions → Release* and download **Wortlaut-Store-x.y.z** (a zip with `Wortlaut-Store.msix`).
2. In Partner Center open the app → *Start update* (or the first submission) → *Packages*, upload
   `Wortlaut-Store.msix`, add notes under *What's new*, and submit.

Certification usually takes one to three business days. The Store package version is the
app version with `.0` appended (`0.3.0` → `0.3.0.0`); it must grow with every submission.

## First submission

| Section | What to enter |
| --- | --- |
| Pricing and availability | Free, all markets |
| Properties | Category *Productivity*; privacy policy: `https://github.com/zdkderpphb/wortlaut/blob/main/docs/PRIVACY.md` |
| Age ratings | Questionnaire: no user interaction, no shared content, no purchases |
| Store listings | German and English, texts below; at least one screenshot taken on Windows, 1366×768 or larger |
| Packages | `Wortlaut-Store.msix` |
| Submission options → Restricted capabilities | Justification below |

**Why the app needs `runFullTrust`** (Partner Center asks for this):

> Wortlaut is a desktop app built with Tauri and Rust. It transcribes audio and video files
> locally on the user's computer with native speech recognition libraries (CPU and Vulkan GPU),
> reads the files the user picks and saves the transcripts where the user chooses.

**Notes for certification:**

> On first start, Wortlaut asks to download its speech recognition models (about 3 GB, from
> huggingface.co). Click "Herunterladen" / "Download" and wait until it finishes. Then drop any
> audio or video file into the window; the transcript appears while it is being created.
> No account is needed. After the download the app works fully offline.

## Store listing texts

### Deutsch

**Kurzbeschreibung**

Audio und Video in Text umwandeln – kostenlos, offline und privat auf Ihrem Computer.

**Beschreibung**

Wortlaut macht aus Aufnahmen Text. Ziehen Sie eine Audio- oder Videodatei ins Fenster, und
Wortlaut schreibt auf, was gesagt wurde – mit dem Sprachmodell Voxtral, vollständig auf Ihrem
Computer. Ihre Aufnahmen verlassen den Computer nie: keine Cloud, kein Konto, keine Kosten.

Beim ersten Start lädt Wortlaut einmalig sein Sprachmodell (ca. 3 GB). Danach funktioniert alles
ohne Internet.

**Funktionen**

- Fast jede Datei: MP3, M4A, WAV, FLAC, OGG, Sprachnachrichten und Videos wie MP4, MOV oder MKV
- 8 Sprachen: Deutsch, Englisch, Französisch, Spanisch, Italienisch, Portugiesisch, Niederländisch, Hindi
- Sprecher unterscheiden – für Interviews und Gesprächsrunden, beliebig viele Personen
- Speichern als Word, Text oder Markdown
- Export für MAXQDA, ATLAS.ti und NVivo, mit Zeitmarken und Sprechern
- Untertitel als SRT-Datei
- Zusammenfassung auf Knopfdruck
- Text bearbeiten, Stellen anhören, Suchen & Ersetzen
- Verlauf mit allen Transkripten, Änderungen werden automatisch gespeichert

**Suchbegriffe:** Transkription, Diktat, Interview, Sprache zu Text, Untertitel, offline, Voxtral

### English

**Short description**

Turn audio and video into text – free, offline and private on your computer.

**Description**

Wortlaut turns recordings into text. Drop an audio or video file into the window and Wortlaut
writes down what was said – with the Voxtral speech model, entirely on your computer. Your
recordings never leave it: no cloud, no account, no cost.

On first start Wortlaut downloads its speech model once (about 3 GB). After that everything works
without internet.

**Features**

- Almost any file: MP3, M4A, WAV, FLAC, OGG, voice messages and videos such as MP4, MOV or MKV
- 8 languages: English, German, French, Spanish, Italian, Portuguese, Dutch, Hindi
- Tells speakers apart – for interviews and group conversations, any number of people
- Save as Word, text or Markdown
- Export for MAXQDA, ATLAS.ti and NVivo, with timestamps and speakers
- Subtitles as SRT files
- Summary at the click of a button
- Edit the text, play back any passage, find & replace
- History of all transcripts, changes are saved automatically

**Search terms:** transcription, dictation, interview, speech to text, subtitles, offline, Voxtral

## How the package differs from the installer

- Windows keeps the app's data (models, history, logs) in the package's own folder,
  `%LOCALAPPDATA%\Packages\<package family>\LocalCache\Local\io.github.zdkderpphb.wortlaut`.
  Uninstalling removes it, the 3 GB model included. `src-tauri/src/packaged.rs` makes
  "open log folder" show that folder.
- The installer brings the WebView2 runtime along if it is missing; the package relies on it
  being there, as it is on current Windows 10 and 11.
- `wortlaut.exe` also works on the command line (an app execution alias); CI uses it to run the
  self-test inside the package.
