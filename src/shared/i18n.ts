import { readFile } from 'node:fs/promises';
import path from 'node:path';

/** Idiomas soportados por la aplicación. */
export type Locale = 'en' | 'es';

/** Diccionario de traducciones cargado en memoria. */
let translations: Record<string, string> = {};
let currentLocale: Locale = 'en';

/** Idiomas disponibles. */
export const AVAILABLE_LOCALES: readonly Locale[] = ['en', 'es'];

/** Directorio donde residen los archivos de locale. */
const LOCALES_DIR = path.resolve(process.cwd(), 'src', 'shared', 'locales');

/**
 * Carga las traducciones para un idioma desde un archivo JSON.
 * Si el archivo no existe o es inválido, se mantiene el diccionario vacío
 * y se devuelven las claves como texto (fallback).
 */
export async function loadTranslations(locale: Locale): Promise<void> {
  try {
    const content = await readFile(path.join(LOCALES_DIR, `${locale}.json`), 'utf8');
    const parsed = JSON.parse(content) as Record<string, string>;
    translations = parsed;
    currentLocale = locale;
  } catch {
    translations = {};
    currentLocale = locale;
  }
}

/** Devuelve el idioma actualmente activo. */
export function getLocale(): Locale {
  return currentLocale;
}

/**
 * Traduce una clave. Si no hay traducción disponible, devuelve la propia clave.
 * Sustituye placeholders `{key}` con los valores proporcionados.
 */
export function t(key: string, values?: Record<string, string | number>): string {
  let text = translations[key] ?? key;
  if (values !== undefined) {
    for (const [placeholder, value] of Object.entries(values)) {
      text = text.replace(new RegExp(`\\{${placeholder}\\}`, 'g'), String(value));
    }
  }
  return text;
}
