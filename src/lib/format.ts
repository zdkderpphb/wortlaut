import { locale } from "./i18n";

const number = new Intl.NumberFormat(locale === "de" ? "de-AT" : "en-US", { maximumFractionDigits: 1 });

export function formatBytes(bytes: number): string {
  if (bytes >= 1e9) return `${number.format(bytes / 1e9)} GB`;
  if (bytes >= 1e6) return `${number.format(bytes / 1e6)} MB`;
  return `${number.format(Math.max(bytes, 0) / 1e3)} KB`;
}

export function formatSpeed(bytesPerSec: number): string {
  return `${formatBytes(bytesPerSec)}/s`;
}

/** "12 Min.", "45 Sek.", "1 Std. 5 Min." – rough on purpose. */
export function formatDuration(seconds: number): string {
  const de = locale === "de";
  const s = Math.max(0, Math.round(seconds));
  if (s < 60) return de ? `${Math.max(s, 5)} Sek.` : `${Math.max(s, 5)} sec`;
  const minutes = Math.round(s / 60);
  if (minutes < 60) return de ? `${minutes} Min.` : `${minutes} min`;
  const h = Math.floor(minutes / 60);
  const m = minutes % 60;
  return de ? `${h} Std. ${m} Min.` : `${h} h ${m} min`;
}

/** 0:42, 12:05, 1:02:03 */
export function formatClock(ms: number): string {
  const total = Math.floor(ms / 1000);
  const h = Math.floor(total / 3600);
  const m = Math.floor((total % 3600) / 60);
  const s = String(total % 60).padStart(2, "0");
  return h > 0 ? `${h}:${String(m).padStart(2, "0")}:${s}` : `${m}:${s}`;
}

/** [00:01:30] – same format as the exported files. */
export function formatStamp(ms: number): string {
  const total = Math.floor(ms / 1000);
  const pad = (n: number) => String(n).padStart(2, "0");
  return `[${pad(Math.floor(total / 3600))}:${pad(Math.floor((total % 3600) / 60))}:${pad(total % 60)}]`;
}

export function fileName(path: string): string {
  return path.split(/[\\/]/).pop() ?? path;
}

export function stem(path: string): string {
  const name = fileName(path);
  const dot = name.lastIndexOf(".");
  return dot > 0 ? name.slice(0, dot) : name;
}

export function extension(path: string): string {
  const name = fileName(path);
  const dot = name.lastIndexOf(".");
  return dot > 0 ? name.slice(dot + 1).toLowerCase() : "";
}

/** Same folder, new extension – keeps the OS's own separator. */
export function siblingPath(path: string, ext: string): string {
  const name = fileName(path);
  return `${path.slice(0, path.length - name.length)}${stem(path)}.${ext}`;
}
