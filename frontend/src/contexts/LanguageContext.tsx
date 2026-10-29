import { createContext, useContext, useState, useEffect, useCallback, type JSX } from 'react';

import es from '../locales/es.json';
import en from '../locales/en.json';

type Locale = 'es' | 'en';

type Translations = typeof es;

type LanguageContextType = {
  locale: Locale;
  t: (key: string, params?: Record<string, string | number>) => string;
  setLocale: (locale: Locale) => void;
};

const translations: Record<Locale, Translations> = {
  es,
  en,
};

const DEFAULT_LOCALE: Locale = 'es';
const STORAGE_KEY = 'media-manager-language';

/**
 * Contexto de internacionalización.
 *
 * Proporciona el idioma actual, una función de traducción `t` y un
 * setter para cambiar el idioma. La elección se persiste en localStorage
 * y se aplica sin recargar la página.
 */
export const LanguageContext = createContext<LanguageContextType | null>(null);

export function LanguageProvider({ children }: { children: React.ReactNode }): JSX.Element {
  const [locale, setLocaleState] = useState<Locale>(DEFAULT_LOCALE);

  // Cargar el idioma guardado en localStorage al montar.
  useEffect(() => {
    const stored = localStorage.getItem(STORAGE_KEY);
    if (stored === 'es' || stored === 'en') {
      // eslint-disable-next-line react-hooks/set-state-in-effect
      setLocaleState(stored);
    }
  }, []);

  const setLocale = useCallback((next: Locale) => {
    setLocaleState(next);
    localStorage.setItem(STORAGE_KEY, next);
  }, []);

  const t = useCallback(
    (key: string, params?: Record<string, string | number>): string => {
      const keys = key.split('.');
      let value: unknown = translations[locale];

      for (const k of keys) {
        if (value && typeof value === 'object' && k in value) {
          value = (value as Record<string, unknown>)[k];
        } else {
          return key;
        }
      }

      if (typeof value !== 'string') {
        return key;
      }

      // Sustituir placeholders como {count}, {season}, {episode}.
      let result = value;
      if (params) {
        for (const [k, v] of Object.entries(params)) {
          result = result.replace(new RegExp(`\\{${k}\\}`, 'g'), String(v));
        }
      }

      return result;
    },
    [locale]
  );

  return (
    <LanguageContext.Provider value={{ locale, t, setLocale }}>{children}</LanguageContext.Provider>
  );
}

/**
 * Hook para acceder al contexto de idioma.
 * Lanza error si se usa fuera de un LanguageProvider.
 */
export function useLanguage(): LanguageContextType {
  const context = useContext(LanguageContext);
  if (!context) {
    throw new Error('useLanguage debe usarse dentro de un LanguageProvider');
  }
  return context;
}
