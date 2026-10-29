import { describe, expect, it } from 'vitest';

import { parseMovie } from '../../../src/core/indexer/parsers/movie.js';
import {
  normalizeTitle,
  parseTitleAndYear,
  stripExtension,
} from '../../../src/core/indexer/parsers/text.js';

describe('parseMovie', () => {
  const cases: { file: string; title: string; year: number | null }[] = [
    { file: 'Arrival.2016.1080p.BluRay.x264.mkv', title: 'Arrival', year: 2016 },
    { file: 'Blade.Runner.2049.2017.1080p.mkv', title: 'Blade Runner 2049', year: 2017 },
    { file: '1917.2019.1080p.mkv', title: '1917', year: 2019 },
    { file: '2001.A.Space.Odyssey.1968.mkv', title: '2001 A Space Odyssey', year: 1968 },
    { file: 'Spider-Man.2002.mkv', title: 'Spider-Man', year: 2002 },
    { file: '[SubsPlease] Your Name (2016).mkv', title: 'Your Name', year: 2016 },
    { file: 'The.Matrix.1999.mkv', title: 'The Matrix', year: 1999 },
    { file: 'Amélie.2001.FRENCH.1080p.mkv', title: 'Amélie', year: 2001 },
    { file: 'Dune.Part.Two.2024.2160p.WEB-DL.mkv', title: 'Dune Part Two', year: 2024 },
    {
      file: 'Movie.Name.2019.1080p.WEB-DL.DDP5.1.H.264-GROUP.mkv',
      title: 'Movie Name',
      year: 2019,
    },
    { file: 'Movie.1080p.mkv', title: 'Movie', year: null },
    { file: '1984.mkv', title: '1984', year: null },
    { file: 'Blade Runner 2049.mkv', title: 'Blade Runner 2049', year: null },
  ];

  it.each(cases)('interpreta "$file" como "$title" ($year)', ({ file, title, year }) => {
    expect(parseMovie(file)).toEqual({ title, year });
  });

  it('devuelve null si solo había etiquetas de release', () => {
    expect(parseMovie('WEB-DL.mkv')).toBeNull();
    expect(parseMovie('1080p.mkv')).toBeNull();
  });

  it('ignora los grupos entre corchetes', () => {
    expect(parseMovie('[SubsPlease] Frieren.mkv')).toEqual({ title: 'Frieren', year: null });
  });

  it('acepta la ruta completa y solo mira el nombre del fichero', () => {
    expect(parseMovie('/media/movies/Arrival.2016.mkv')).toEqual({
      title: 'Arrival',
      year: 2016,
    });
  });
});

describe('parseTitleAndYear', () => {
  it('extrae el año de una carpeta entre paréntesis', () => {
    expect(parseTitleAndYear('Severance (2022)')).toEqual({ title: 'Severance', year: 2022 });
  });

  it('acepta el año entre corchetes', () => {
    expect(parseTitleAndYear('Severance [2022]')).toEqual({ title: 'Severance', year: 2022 });
  });

  it('no confunde un número del título con el año', () => {
    expect(parseTitleAndYear('Blade Runner 2049')).toEqual({
      title: 'Blade Runner 2049',
      year: null,
    });
  });

  it('conserva un título que es solo un año', () => {
    expect(parseTitleAndYear('1917')).toEqual({ title: '1917', year: null });
  });

  it('devuelve null si no queda título', () => {
    expect(parseTitleAndYear('[SubsPlease]')).toBeNull();
    expect(parseTitleAndYear('   ')).toBeNull();
  });
});

describe('normalizeTitle', () => {
  it('convierte puntos y guiones bajos en espacios', () => {
    expect(normalizeTitle('The_Matrix.1999')).toBe('The Matrix 1999');
  });

  it('elimina las etiquetas de release', () => {
    expect(normalizeTitle('Dune 2160p WEB-DL x265 HEVC DDP5.1')).toBe('Dune');
  });

  it('elimina el grupo de release al final', () => {
    expect(normalizeTitle('Movie 1080p x264-GROUP')).toBe('Movie');
  });

  it('respeta los guiones que forman parte del título', () => {
    expect(normalizeTitle('Spider-Man')).toBe('Spider-Man');
  });

  it('conserva un año entre corchetes y descarta el resto de grupos', () => {
    expect(normalizeTitle('[SubsPlease] Dune [2024]')).toBe('Dune 2024');
  });
});

describe('stripExtension', () => {
  it('quita solo la extensión final', () => {
    expect(stripExtension('Show.S01E02.mkv')).toBe('Show.S01E02');
  });

  it('no recorta nombres sin extensión real', () => {
    expect(stripExtension('Show.S01E02')).toBe('Show.S01E02');
    expect(stripExtension('Show')).toBe('Show');
  });
});
