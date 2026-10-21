// All user-facing text, German first. Pick by system language.

const de = {
  appTagline: "Audio und Video in Text umwandeln",
  offlineBadge: "Offline · auf Ihrem Computer",

  setupTitle: "Willkommen bei Wortlaut",
  setupIntro:
    "Wortlaut wandelt Aufnahmen in Text um – komplett auf Ihrem Computer. Ihre Dateien verlassen das Gerät nie.",
  setupPointPrivate: "Privat und offline",
  setupPointFormats: "Speichern als Word, Text oder Markdown",
  setupPointLanguages: "Deutsch, Englisch und 6 weitere Sprachen",
  setupCardTitle: "Einmalige Einrichtung",
  setupCardText:
    "Dafür wird einmal das Sprachmodell geladen ({size}). Danach funktioniert alles ohne Internet.",
  setupFreeSpace: "Freier Speicherplatz: {free}",
  setupStart: "Jetzt einrichten",
  setupResume: "Einrichtung fortsetzen",
  setupResumeHint: "{done} von {total} sind schon geladen.",
  setupDownloading: "Sprachmodell wird geladen …",
  setupProgress: "{done} von {total} · {speed}",
  setupRemaining: "noch ca. {time}",
  setupVerifying: "Download wird geprüft …",
  setupPause: "Pausieren",
  setupRetry: "Erneut versuchen",
  setupKeepOpen: "Bitte lassen Sie Wortlaut geöffnet. Sie können den Download jederzeit pausieren.",

  dropTitle: "Datei hierher ziehen",
  dropSubtitle: "oder klicken, um Dateien auszuwählen",
  dropFormats: "MP3, M4A, WAV, FLAC, OGG, Opus (WhatsApp), MP4, MOV, MKV, WebM",
  dropActive: "Loslassen zum Hinzufügen",
  addMore: "Weitere Dateien",
  languageLabel: "Gesprochene Sprache",
  timestampsLabel: "Zeitmarken einfügen",
  timestampsHint: "z. B. [00:01:30] vor jedem Absatz",
  unsupportedFiles: "Nicht unterstützt und übersprungen: {names}",

  langAuto: "Automatisch erkennen",
  langDe: "Deutsch",
  langEn: "Englisch",
  langFr: "Französisch",
  langEs: "Spanisch",
  langIt: "Italienisch",
  langPt: "Portugiesisch",
  langNl: "Niederländisch",
  langHi: "Hindi",

  statusWaiting: "Wartet",
  statusRunning: "Läuft",
  statusDone: "Fertig",
  statusError: "Fehler",
  statusCancelled: "Abgebrochen",

  stageReading: "Datei wird gelesen …",
  stageLoading: "Sprachmodell wird gestartet (nur beim ersten Mal) …",
  stageTranscribing: "Wird in Text umgewandelt …",
  stageWaiting: "Wartet, bis die vorherige Datei fertig ist.",
  progressOf: "{done} von {total}",
  remaining: "noch ca. {time}",
  cancel: "Abbrechen",
  remove: "Entfernen",
  retry: "Erneut versuchen",

  resultEditHint: "Sie können den Text direkt bearbeiten, bevor Sie ihn speichern.",
  resultEmpty: "In dieser Datei wurde keine Sprache erkannt.",
  saveAs: "Speichern als",
  saveWord: "Word",
  saveText: "Text",
  saveMarkdown: "Markdown",
  copy: "Kopieren",
  copied: "Kopiert",
  saved: "Gespeichert: {name}",
  showInFolder: "Im Ordner zeigen",
  saveAll: "Alle speichern",
  saveAllAs: "Alle fertigen Texte speichern als",
  savedAll: "{count} Dateien gespeichert",

  errorTitle: "Das hat leider nicht geklappt",
  errorDetails: "Technische Details",
  openLog: "Protokoll öffnen",
  err_unsupported_format: "Diese Datei kann leider nicht gelesen werden. Versuchen Sie z. B. MP3, M4A oder MP4.",
  err_no_audio: "In dieser Datei wurde keine Tonspur gefunden.",
  err_file_read: "Die Datei konnte nicht geöffnet werden. Wurde sie verschoben oder gelöscht?",
  err_model_missing: "Das Sprachmodell fehlt. Bitte richten Sie Wortlaut erneut ein.",
  err_model_load:
    "Das Sprachmodell konnte nicht gestartet werden. Oft hilft es, andere Programme zu schließen und es erneut zu versuchen (mind. 8 GB Arbeitsspeicher empfohlen).",
  err_transcription_failed: "Bei der Umwandlung ist ein Fehler aufgetreten.",
  err_network:
    "Keine Verbindung zum Download-Server. Bitte prüfen Sie Ihre Internetverbindung – der Download wird dort fortgesetzt, wo er aufgehört hat.",
  err_checksum: "Der Download war beschädigt und wurde verworfen. Bitte versuchen Sie es erneut.",
  err_no_space: "Nicht genug Speicherplatz. Benötigt werden {needed}, frei sind {free}.",
  err_file_write: "Die Datei konnte nicht gespeichert werden.",
  err_export_failed: "Die Datei konnte nicht gespeichert werden. Ist der Ordner schreibgeschützt?",
  err_unknown: "Ein unerwarteter Fehler ist aufgetreten.",

  footerPrivacy: "Ihre Aufnahmen bleiben auf diesem Computer.",
};

type Messages = typeof de;

const en: Messages = {
  appTagline: "Turn audio and video into text",
  offlineBadge: "Offline · on your computer",

  setupTitle: "Welcome to Wortlaut",
  setupIntro:
    "Wortlaut turns recordings into text – entirely on your computer. Your files never leave this device.",
  setupPointPrivate: "Private and offline",
  setupPointFormats: "Save as Word, text or Markdown",
  setupPointLanguages: "English, German and 6 more languages",
  setupCardTitle: "One-time setup",
  setupCardText: "Wortlaut downloads its speech model once ({size}). After that, everything works without internet.",
  setupFreeSpace: "Free disk space: {free}",
  setupStart: "Set up now",
  setupResume: "Continue setup",
  setupResumeHint: "{done} of {total} already downloaded.",
  setupDownloading: "Downloading the speech model …",
  setupProgress: "{done} of {total} · {speed}",
  setupRemaining: "about {time} left",
  setupVerifying: "Checking the download …",
  setupPause: "Pause",
  setupRetry: "Try again",
  setupKeepOpen: "Please keep Wortlaut open. You can pause the download at any time.",

  dropTitle: "Drop a file here",
  dropSubtitle: "or click to choose files",
  dropFormats: "MP3, M4A, WAV, FLAC, OGG, Opus (WhatsApp), MP4, MOV, MKV, WebM",
  dropActive: "Release to add",
  addMore: "Add files",
  languageLabel: "Spoken language",
  timestampsLabel: "Add timestamps",
  timestampsHint: "e.g. [00:01:30] before each paragraph",
  unsupportedFiles: "Not supported, skipped: {names}",

  langAuto: "Detect automatically",
  langDe: "German",
  langEn: "English",
  langFr: "French",
  langEs: "Spanish",
  langIt: "Italian",
  langPt: "Portuguese",
  langNl: "Dutch",
  langHi: "Hindi",

  statusWaiting: "Waiting",
  statusRunning: "Running",
  statusDone: "Done",
  statusError: "Error",
  statusCancelled: "Cancelled",

  stageReading: "Reading the file …",
  stageLoading: "Starting the speech model (first time only) …",
  stageTranscribing: "Turning speech into text …",
  stageWaiting: "Waiting for the previous file to finish.",
  progressOf: "{done} of {total}",
  remaining: "about {time} left",
  cancel: "Cancel",
  remove: "Remove",
  retry: "Try again",

  resultEditHint: "You can edit the text right here before saving it.",
  resultEmpty: "No speech was found in this file.",
  saveAs: "Save as",
  saveWord: "Word",
  saveText: "Text",
  saveMarkdown: "Markdown",
  copy: "Copy",
  copied: "Copied",
  saved: "Saved: {name}",
  showInFolder: "Show in folder",
  saveAll: "Save all",
  saveAllAs: "Save all finished texts as",
  savedAll: "{count} files saved",

  errorTitle: "Sorry, that didn't work",
  errorDetails: "Technical details",
  openLog: "Open log",
  err_unsupported_format: "This file can't be read. Try MP3, M4A or MP4, for example.",
  err_no_audio: "This file has no audio track.",
  err_file_read: "The file couldn't be opened. Was it moved or deleted?",
  err_model_missing: "The speech model is missing. Please set up Wortlaut again.",
  err_model_load:
    "The speech model couldn't start. Closing other programs and trying again often helps (8 GB of memory or more recommended).",
  err_transcription_failed: "Something went wrong while transcribing.",
  err_network:
    "Can't reach the download server. Please check your internet connection – the download will continue where it stopped.",
  err_checksum: "The download was damaged and has been discarded. Please try again.",
  err_no_space: "Not enough disk space. {needed} needed, {free} free.",
  err_file_write: "The file couldn't be saved.",
  err_export_failed: "The file couldn't be saved. Is the folder read-only?",
  err_unknown: "An unexpected error occurred.",

  footerPrivacy: "Your recordings stay on this computer.",
};

export type MessageKey = keyof Messages;

export const locale: "de" | "en" =
  typeof navigator !== "undefined" && !navigator.language.toLowerCase().startsWith("de") ? "en" : "de";

const messages = locale === "de" ? de : en;

export function t(key: MessageKey, params: Record<string, string | number> = {}): string {
  return messages[key].replace(/\{(\w+)\}/g, (_, name) => String(params[name] ?? `{${name}}`));
}

/** Friendly text for an error code coming from Rust. */
export function errorText(code: string, params: Record<string, string | number> = {}): string {
  const key = `err_${code}` as MessageKey;
  return key in messages ? t(key, params) : t("err_unknown");
}

export const languages: { code: string; label: MessageKey }[] = [
  { code: "auto", label: "langAuto" },
  { code: "de", label: "langDe" },
  { code: "en", label: "langEn" },
  { code: "fr", label: "langFr" },
  { code: "es", label: "langEs" },
  { code: "it", label: "langIt" },
  { code: "pt", label: "langPt" },
  { code: "nl", label: "langNl" },
  { code: "hi", label: "langHi" },
];
