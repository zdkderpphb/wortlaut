// Remembered choices. A convenience only: everything works without storage.

function load<T>(key: string, fallback: T): T {
  try {
    const raw = localStorage.getItem(`wortlaut.${key}`);
    return raw === null ? fallback : (JSON.parse(raw) as T);
  } catch {
    return fallback;
  }
}

function store(key: string, value: unknown) {
  try {
    localStorage.setItem(`wortlaut.${key}`, JSON.stringify(value));
  } catch {
    /* private mode or blocked storage */
  }
}

class Settings {
  language = $state(load("language", "auto"));
  detectSpeakers = $state(load("detectSpeakers", false));
  /** null = automatic */
  speakerCount = $state<number | null>(load("speakerCount", null));
  timestamps = $state(load("timestamps", false));

  constructor() {
    $effect.root(() => {
      $effect(() => store("language", this.language));
      $effect(() => store("detectSpeakers", this.detectSpeakers));
      $effect(() => store("speakerCount", this.speakerCount));
      $effect(() => store("timestamps", this.timestamps));
    });
  }
}

export const settings = new Settings();
