import { useEffect, useState } from 'react';

import { loadTranslations, getLocale, t, type Locale } from '../../shared/i18n.js';

/** Hook para traducir strings de la interfaz. */
export function useTranslation(): {
  t: typeof t;
  locale: Locale;
  setLocale: (locale: Locale) => Promise<void>;
} {
  const [locale, setLocaleState] = useState<Locale>(() => getLocale());

  useEffect(() => {
    void loadTranslations(locale).then(() => {
      setLocaleState(getLocale());
    });
  }, [locale]);

  const setLocale = async (next: Locale): Promise<void> => {
    await loadTranslations(next);
    setLocaleState(getLocale());
  };

  return { t, locale, setLocale };
}
