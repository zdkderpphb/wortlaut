# Wortlaut – Design

Datum: 2026-10-21 · Status: freigegeben

## Ziel

Eine Desktop-App für Windows und macOS, mit der absolute Laien Audio- und
Videodateien **lokal und offline** in Text umwandeln. Ein Installer, ein Klick,
keine Technik-Kenntnisse. Spracherkennung mit Mistrals **Voxtral** als lokales
Modell. Ausgabe als **.docx**, **.txt** und **.md**.

## Nicht-Ziele (v1)

- Mikrofon-Aufnahme, Live-Diktat
- Sprecher-Erkennung (wer spricht wann)
- Intel-Macs, Linux
- Cloud-Funktionen, Konten, Telemetrie
- Auswahl verschiedener Modelle in der Oberfläche

## Entscheidungen

| Thema | Entscheidung | Grund |
| --- | --- | --- |
| App-Rahmen | Tauri 2 (Rust + Web-Oberfläche) | kleiner Installer (~15–30 MB), nativ, kein Python |
| Oberfläche | Svelte 5 + TypeScript + Vite, eigenes CSS | schlank, wenig Boilerplate |
| Laufzeit | `transcribe-cpp` 0.4 (Rust-Binding von transcribe.cpp, MIT) | Voxtral nativ, Metal/Vulkan, Abbruch, Sprachhinweis |
| Modell | `Voxtral-Mini-3B-2507-Q4_K_M.gguf` von `handy-computer/Voxtral-Mini-3B-2507-GGUF` (2 984 721 056 Bytes, SHA256 `3a6717aa8f8989108d260cbd237584289eec43cc987e10133c33643515936205`) | ~2× schneller als 4B-Realtime, Sprachvorgabe möglich, WER ≈ Q8 |
| Modell-Auslieferung | Download beim ersten Start | GitHub-Release-Dateien sind auf 2 GB begrenzt |
| Audio-Dekodierung | `symphonia` (pure Rust) + Opus via libopus-Adapter | mp3, m4a/aac, wav, flac, ogg, opus, mp4, mov, mkv, webm ohne ffmpeg |
| macOS-Backend | statisch, Metal, eingebettete metallib | eine Binärdatei, nichts zu bündeln |
| Windows-Backend | `vulkan` + `dynamic-backends` | fehlt Vulkan, lädt die App trotzdem und nutzt die CPU |
| Lizenz | MIT | passt zu transcribe.cpp (MIT) und Voxtral (Apache 2.0) |
| Repo | `github.com/zdkderpphb/wortlaut`, öffentlich | direkte Downloads für Laien, freie CI-Minuten |

## Bedienung

Drei Bildschirme, deutsch/englisch nach Systemsprache, hell/dunkel nach System.

1. **Einrichtung** (nur beim ersten Start)
   - Kurze Erklärung: „Wortlaut arbeitet komplett auf Ihrem Computer. Dafür wird
     einmalig das Sprachmodell geladen (ca. 3 GB).“
   - Knopf „Jetzt einrichten“. Vorher Prüfung des freien Speicherplatzes.
   - Fortschrittsbalken mit MB, Geschwindigkeit und Restzeit. Abbruch und
     Fortsetzen möglich (HTTP-Range). Nach dem Download SHA256-Prüfung.
2. **Start**
   - Große Ablagefläche: „Datei hierher ziehen oder klicken“. Mehrere Dateien
     möglich → Warteliste.
   - Sprache: „Automatisch erkennen“ (Standard), Deutsch, Englisch, Französisch,
     Spanisch, Italienisch, Portugiesisch, Niederländisch, Hindi.
   - Schalter „Zeitmarken einfügen“ (aus).
3. **Ergebnis**
   - Während der Verarbeitung: Fortschrittsbalken, Restzeit, Text erscheint
     abschnittsweise, Knopf „Abbrechen“.
   - Danach: bearbeitbarer Text. Knöpfe „Word (.docx)“, „Text (.txt)“,
     „Markdown (.md)“, „Kopieren“. Speichern-Dialog schlägt Name und Ordner der
     Originaldatei vor. Bei mehreren Dateien zusätzlich „Alle speichern“.

## Architektur

```
Svelte-Oberfläche ──invoke──▶ Tauri-Befehle ──▶ Rust-Kern
        ▲                                         │
        └────────── Events (Fortschritt, Text) ───┘
```

Rust-Module unter `src-tauri/src/`, je eine Aufgabe, einzeln testbar:

- `audio.rs` – Datei → `Vec<f32>` 16 kHz mono. Symphonia dekodiert, Kanäle
  werden gemittelt, `rubato` resampelt.
- `chunker.rs` – teilt PCM in Abschnitte von ca. 30 s (min. 20 s, max. 30 s).
  Schnitt an der leisesten 100-ms-Stelle im Bereich 20–30 s. Liefert
  Start-/Endzeit je Abschnitt.
- `engine.rs` – lädt das Modell einmal (`Model::load`), Session mit begrenztem
  `n_ctx`, `CancelToken` für Abbruch. Transkribiert Abschnitt für Abschnitt und
  meldet jeden Teiltext über einen Callback.
- `model_manager.rs` – Pfad im App-Datenordner, Status (fehlt / teilweise /
  bereit), Download mit Fortsetzen, Fortschritt, SHA256, Speicherplatzprüfung.
- `export.rs` – Transkript (Abschnitte mit Zeiten) → txt, md, docx
  (`docx-rs`). Zeitmarken optional als `[hh:mm:ss]` je Absatz.
- `commands.rs` – Tauri-Befehle und Events; hält den App-Zustand
  (geladene Engine, laufender Auftrag).

Die Engine läuft in einem eigenen Thread; die Oberfläche blockiert nie.

## Datenfluss

1. Nutzer legt Datei ab → `transcribe_file(path, language, timestamps)`.
2. `audio` dekodiert → `chunker` teilt → `engine` transkribiert je Abschnitt.
3. Pro Abschnitt Event `transcript-progress { job_id, done_ms, total_ms, text }`.
4. Ende: Event `transcript-done { job_id, segments }` oder `transcript-error`.
5. Nutzer klickt Export → `export_transcript(segments, format, path)`.

## Fehlerbehandlung

Jeder Fehler hat eine verständliche Meldung und optional aufklappbare Details.

- Format nicht lesbar → „Diese Datei kann leider nicht gelesen werden.“
- Keine Sprache erkannt → leeres Ergebnis mit Hinweis statt Fehler.
- Zu wenig Speicherplatz → benötigter und freier Platz in GB.
- Download unterbrochen → „Fortsetzen“-Knopf; Teil-Datei bleibt erhalten.
- Prüfsumme falsch → Datei löschen, erneut laden.
- Modell lädt nicht (z. B. zu wenig RAM) → Hinweis auf Systemanforderungen.
- Log-Datei im App-Log-Ordner (`tauri-plugin-log`), Menüpunkt „Log öffnen“.

## Build und Release

- GitHub Actions, Auslöser: Tag `v*` und manuell.
- macOS (Apple Silicon, `macos-latest`): `.dmg`.
- Windows x64 (`windows-latest`): NSIS-`.exe`, Installation pro Benutzer ohne
  Admin-Rechte, WebView2 wird bei Bedarf nachgeladen. Vulkan-SDK im Build.
- Signierung: wird aktiv, sobald Secrets gesetzt sind
  (`APPLE_CERTIFICATE`, `APPLE_ID`, … bzw. Windows-Zertifikat). Ohne Secrets
  unsignierte Builds; README erklärt das Öffnen Schritt für Schritt.
- Ergebnis: Entwurfs-Release mit beiden Installern.

## Tests

- Rust-Unit-Tests: Chunker (Grenzen, Stille, kurze Dateien), Resampling,
  Export (txt/md/docx-Inhalt, Zeitmarken), Download-Fortsetzen gegen lokalen
  Testserver, SHA256-Prüfung.
- Integrationstest (lokal, mit echtem Modell, per Umgebungsvariable aktiviert):
  deutsche Test-WAV → erwarteter Text.
- CI: `cargo test` + Frontend-Typprüfung auf beiden Plattformen.
- Manuell: App starten, Einrichtung, echte Datei transkribieren, alle drei
  Exporte öffnen.

## README

Einfach und klar, für Laien zuerst: was die App macht, ein Download-Link je
System, drei Schritte zur Nutzung, Hinweis zur ersten Öffnung ohne Signatur,
Systemanforderungen. Entwickler-Teil kurz und am Ende.
