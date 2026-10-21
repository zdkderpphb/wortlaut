<div align="center">

<img src="assets/icon.png" width="112" alt="Wortlaut-Logo">

# Wortlaut

**Audio und Video in Text umwandeln – kostenlos, offline, auf Ihrem Computer.**

[**⬇ Für Mac herunterladen**](https://github.com/zdkderpphb/wortlaut/releases/latest/download/Wortlaut-Mac.dmg)
&nbsp;&nbsp;·&nbsp;&nbsp;
[**⬇ Für Windows herunterladen**](https://github.com/zdkderpphb/wortlaut/releases/latest/download/Wortlaut-Windows-setup.exe)

<img src="assets/screenshot-result.png" width="760" alt="Wortlaut zeigt ein fertiges Transkript mit Knöpfen zum Speichern als Word, Text oder Markdown">

</div>

---

## So einfach geht's

1. **Installieren** – Laden Sie die Datei für Ihren Computer herunter und öffnen Sie sie.
2. **Einmal einrichten** – Beim ersten Start lädt Wortlaut sein Sprachmodell (ca. 3 GB).
   Das dauert je nach Internet 5 bis 15 Minuten und passiert nur ein einziges Mal.
3. **Datei hineinziehen** – Ziehen Sie eine Aufnahme ins Fenster. Der Text erscheint,
   und Sie speichern ihn mit einem Klick als **Word**, **Text** oder **Markdown**.

Das war's. Ab jetzt funktioniert Wortlaut auch ohne Internet.

<div align="center">
<img src="assets/screenshot-home.png" width="560" alt="Startseite von Wortlaut mit großer Fläche zum Hineinziehen von Dateien">
</div>

## Was Wortlaut kann

- **Privat:** Ihre Aufnahmen verlassen den Computer nie. Keine Cloud, kein Konto, keine Kosten.
- **Fast jede Datei:** MP3, M4A, WAV, FLAC, OGG, WhatsApp-Sprachnachrichten (Opus),
  und Videos wie MP4, MOV, MKV oder WebM.
- **8 Sprachen:** Deutsch, Englisch, Französisch, Spanisch, Italienisch, Portugiesisch,
  Niederländisch und Hindi – auf Wunsch automatisch erkannt.
- **Zeitmarken:** Auf Wunsch steht vor jedem Absatz, wann er gesagt wurde, z. B. `[00:12:30]`.
- **Mehrere Dateien:** Einfach alle auf einmal hineinziehen – sie werden der Reihe nach umgewandelt.
- **Text bearbeiten:** Tippfehler direkt in Wortlaut korrigieren, dann speichern.

## Beim ersten Öffnen

Wortlaut ist noch nicht bei Apple und Microsoft registriert. Ihr Computer fragt deshalb
beim ersten Öffnen einmal nach. So geht es weiter:

<details>
<summary><b>Mac</b></summary>

1. Öffnen Sie die heruntergeladene Datei `Wortlaut-Mac.dmg` und ziehen Sie **Wortlaut**
   in den Ordner **Programme**.
2. Starten Sie Wortlaut. Es erscheint eine Meldung, dass die App nicht geöffnet wurde –
   klicken Sie auf **Fertig**.
3. Öffnen Sie **Systemeinstellungen → Datenschutz & Sicherheit**, scrollen Sie nach unten
   und klicken Sie bei „Wortlaut“ auf **Dennoch öffnen**.
4. Bestätigen Sie mit Ihrem Passwort. Ab jetzt startet Wortlaut ganz normal.

</details>

<details>
<summary><b>Windows</b></summary>

1. Öffnen Sie die heruntergeladene Datei `Wortlaut-Windows-setup.exe`.
2. Erscheint „Der Computer wurde durch Windows geschützt“, klicken Sie auf
   **Weitere Informationen** und dann auf **Trotzdem ausführen**.
3. Die Installation braucht keine Administrator-Rechte. Wortlaut finden Sie danach im Startmenü.

</details>

## Was Ihr Computer braucht

|                | Mac                                 | Windows                          |
| -------------- | ----------------------------------- | -------------------------------- |
| System         | macOS 13 oder neuer                 | Windows 10 oder 11 (64 Bit)      |
| Prozessor      | Apple-Chip (M1 oder neuer)          | jeder aktuelle Intel- oder AMD-Prozessor |
| Arbeitsspeicher| 8 GB, besser 16 GB                  | 8 GB, besser 16 GB               |
| Speicherplatz  | 4 GB frei                           | 4 GB frei                        |

**Wie schnell?** Auf einem Mac mit Apple-Chip ist eine Stunde Aufnahme meist in 5 bis 15 Minuten
fertig. Unter Windows nutzt Wortlaut die Grafikkarte, wenn möglich. Ohne passende Grafikkarte
dauert die Umwandlung etwa so lange wie die Aufnahme selbst.

## Häufige Fragen

<details>
<summary><b>Werden meine Aufnahmen irgendwohin geschickt?</b></summary>

Nein. Wortlaut verbindet sich nur ein einziges Mal mit dem Internet, um das Sprachmodell
herunterzuladen. Die Umwandlung passiert vollständig auf Ihrem Computer.
</details>

<details>
<summary><b>Wie genau ist der Text?</b></summary>

Bei deutlicher Sprache sehr genau. Namen, Fachbegriffe und Gespräche, in denen viele
durcheinander reden, sollten Sie kurz gegenlesen. Wer gerade spricht, erkennt Wortlaut nicht.
</details>

<details>
<summary><b>Der Download ist abgebrochen. Muss ich von vorne anfangen?</b></summary>

Nein. Starten Sie Wortlaut neu und klicken Sie auf **Einrichtung fortsetzen** –
der Download macht dort weiter, wo er aufgehört hat.
</details>

<details>
<summary><b>Etwas funktioniert nicht – was tun?</b></summary>

Bei einer Fehlermeldung finden Sie unter **Technische Details** einen Link
**Protokoll öffnen**. Schicken Sie uns diese Datei gerne mit einer kurzen Beschreibung als
[Fehlermeldung auf GitHub](https://github.com/zdkderpphb/wortlaut/issues/new).
</details>

<details>
<summary><b>Wie entferne ich Wortlaut wieder?</b></summary>

- **Mac:** Wortlaut aus dem Ordner *Programme* in den Papierkorb ziehen. Das Sprachmodell liegt unter
  `~/Library/Application Support/io.github.zdkderpphb.wortlaut` und kann ebenfalls gelöscht werden.
- **Windows:** *Einstellungen → Apps → Wortlaut → Deinstallieren*. Das Sprachmodell liegt unter
  `%LOCALAPPDATA%\io.github.zdkderpphb.wortlaut`.
</details>

---

## Für Entwickler

Wortlaut ist eine [Tauri 2](https://tauri.app)-App: Oberfläche in Svelte 5, Kern in Rust.
Die Spracherkennung läuft über [transcribe.cpp](https://github.com/handy-computer/transcribe.cpp)
(Metal auf dem Mac, Vulkan oder CPU unter Windows) mit
[Voxtral Mini 3B](https://huggingface.co/mistralai/Voxtral-Mini-3B-2507) von Mistral AI
in der 4-Bit-Fassung (`Q4_K_M`).

```bash
npm install
npm run tauri dev       # App im Entwicklungsmodus starten
npm run tauri build     # Installer bauen
```

Voraussetzungen: Node 24, Rust (stable), CMake. Unter Windows zusätzlich Visual Studio Build Tools
und das [Vulkan SDK](https://vulkan.lunarg.com/sdk/home).

**Tests**

```bash
npm run build && cargo test --manifest-path src-tauri/Cargo.toml --lib
# mit echtem Modell:
WORTLAUT_MODEL=/pfad/zu/Voxtral-Mini-3B-2507-Q4_K_M.gguf cargo test --manifest-path src-tauri/Cargo.toml --lib
```

**Aufbau**

| Ordner / Datei                  | Aufgabe                                                   |
| ------------------------------- | --------------------------------------------------------- |
| `src/lib/`                      | Oberfläche: Einrichtung, Warteliste, Ergebnis, Texte (de/en) |
| `src-tauri/src/audio.rs`        | Audio und Video lesen, auf 16 kHz mono umrechnen          |
| `src-tauri/src/chunker.rs`      | Aufnahmen in 20–30-s-Abschnitte teilen, in Sprechpausen   |
| `src-tauri/src/engine.rs`       | Voxtral laden und Abschnitt für Abschnitt umwandeln       |
| `src-tauri/src/model_manager.rs`| Modell herunterladen (fortsetzbar, mit Prüfsumme)         |
| `src-tauri/src/export.rs`       | Speichern als Word, Text, Markdown                        |
| `src-tauri/src/commands.rs`     | Verbindung zwischen Oberfläche und Rust                   |

**Neue Version veröffentlichen:** Versionsnummer in `package.json`, `src-tauri/Cargo.toml` und
`src-tauri/tauri.conf.json` erhöhen, dann `git tag v0.2.0 && git push --tags`. GitHub Actions baut
beide Installer, prüft sie mit einem Selbsttest und legt einen Release-Entwurf an. Nach dem
Veröffentlichen zeigen die Download-Links oben automatisch auf die neue Version.
Signierung für Mac und Windows: siehe [docs/SIGNING.md](docs/SIGNING.md).

## Lizenz und Dank

Wortlaut steht unter der [MIT-Lizenz](LICENSE).
Es baut auf großartiger offener Arbeit auf: dem Sprachmodell **Voxtral** von
[Mistral AI](https://mistral.ai) (Apache 2.0), **transcribe.cpp** und **ggml** (MIT),
**Tauri**, **Symphonia** und **libopus**. Details in [THIRD-PARTY-NOTICES.md](THIRD-PARTY-NOTICES.md).
„Voxtral“ ist eine Marke von Mistral AI; Wortlaut ist kein Produkt von Mistral AI.
