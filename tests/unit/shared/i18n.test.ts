import { describe, expect, it, beforeEach } from 'vitest';

import {
  AVAILABLE_LOCALES,
  getLocale,
  loadTranslations,
  t,
  type Locale,
} from '../../../src/shared/i18n.js';

beforeEach(() => {
  void loadTranslations('en');
});

describe('i18n', () => {
  it('t devuelve la clave cuando no hay traducción', () => {
    expect(t('nonexistent.key')).toBe('nonexistent.key');
  });

  it('t devuelve la traducción cargada', async () => {
    await loadTranslations('es');
    expect(getLocale()).toBe('es');
    expect(t('app.library')).toBe('Biblioteca');
  });

  it('t sustituye placeholders', async () => {
    await loadTranslations('en');
    expect(t('list.search_empty', { query: 'Dark' })).toBe('Sin resultados para «Dark».');
  });

  it('AVAILABLE_LOCALES incluye en y es', () => {
    expect(AVAILABLE_LOCALES).toContain('en');
    expect(AVAILABLE_LOCALES).toContain('es');
  });

  it('cargar locale inválido no rompe', async () => {
    await expect(loadTranslations('fr' as Locale)).resolves.toBeUndefined();
    expect(t('app.library')).toBe('app.library');
  });
});
