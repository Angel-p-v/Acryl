import { describe, it, expect } from 'vitest';

import {
  encodeKittyGraphic,
  deleteKittyImage,
  deleteAllKittyImages,
  type KittyFrame,
  CHUNK_SIZE,
  FORMAT_PNG,
} from '../../../src/shared/kittyProtocol.js';

describe('kittyProtocol', () => {
  it('CHUNK_SIZE es 4096 y FORMAT_PNG es 100', () => {
    expect(CHUNK_SIZE).toBe(4096);
    expect(FORMAT_PNG).toBe(100);
  });

  it('encodeKittyGraphic genera una secuencia válida para un buffer pequeño', () => {
    const buffer = Buffer.from('imagen-prueba');

    const frames = encodeKittyGraphic({
      buffer,
      columns: 45,
      rows: 32,
      imageId: 42,
    });

    expect(frames.length).toBe(1);
    const frame = frames[0]!;
    expect(frame.isLast).toBe(true);

    const seq = frame.sequence;
    expect(seq.slice(0, 3)).toBe('\x1b_G');
    expect(seq.slice(-2)).toBe('\x1b\\');

    // Parámetros obligatorios del protocolo Kitty (según especificación).
    expect(seq).toContain('a=T');
    expect(seq).toContain('f=100'); // formato PNG (Sharp)
    expect(seq).toContain('t=d'); // datos directos (no ruta de fichero)
    expect(seq).toContain('c=45'); // columnas
    expect(seq).toContain('r=32'); // filas
    expect(seq).toContain('i=42'); // ID único
    expect(seq).toContain('z=1'); // Z-index
    expect(seq).toContain('m=0'); // mensaje final (único fragmento)
    // No debe indicar ruta de fichero (t=f), solo datos directos (t=d).
    expect(seq).not.toContain('t=f');

    // Los datos van después del punto y coma, en Base64.
    const payload = seq.split(';')[1] ?? '';
    expect(Buffer.from(payload.slice(0, -2), 'base64').toString()).toBe('imagen-prueba');
  });

  it('encodeKittyGraphic fragmenta en bloques de 4096 bytes para buffers grandes', () => {
    // 9000 bytes → Base64 de 12000 chars → 3 fragmentos de 4096.
    const bigBuffer = Buffer.alloc(9000, 0x41); // 'A' repetido

    const frames = encodeKittyGraphic({
      buffer: bigBuffer,
      columns: 45,
      rows: 32,
      imageId: 7,
    });

    expect(frames.length).toBe(3);
    expect(frames[0]!.isLast).toBe(false);
    expect(frames[1]!.isLast).toBe(false);
    expect(frames[2]!.isLast).toBe(true);

    // m=1 en intermedios, m=0 en el último.
    expect(frames[0]!.sequence).toContain('m=1');
    expect(frames[1]!.sequence).toContain('m=1');
    expect(frames[2]!.sequence).toContain('m=0');

    // Todos los fragmentos llevan los parámetros completos.
    for (const frame of frames) {
      expect(frame.sequence).toContain('a=T');
      expect(frame.sequence).toContain('f=100');
      expect(frame.sequence).toContain('t=d');
      expect(frame.sequence).toContain('c=45');
      expect(frame.sequence).toContain('r=32');
      expect(frame.sequence).toContain('i=7');
      expect(frame.sequence).toContain('z=1');
      // No debe indicar ruta de fichero (t=f), solo datos directos (t=d).
      expect(frame.sequence).not.toContain('t=f');
    }

    // Cada fragmento es una secuencia válida.
    frames.forEach((frame: KittyFrame) => {
      expect(frame.sequence.slice(0, 3)).toBe('\x1b_G');
      expect(frame.sequence.slice(-2)).toBe('\x1b\\');
    });
  });

  it('encodeKittyGraphic no trunca nunca: envía todos los chunks', () => {
    // 20000 bytes → Base64 de ~26667 chars → 7 fragmentos de 4096.
    const bigBuffer = Buffer.alloc(20000, 0x42);

    const frames = encodeKittyGraphic({
      buffer: bigBuffer,
      columns: 45,
      rows: 32,
      imageId: 99,
    });

    expect(frames.length).toBe(7);
    expect(frames[6]!.isLast).toBe(true);
    expect(frames[0]!.isLast).toBe(false);
  });

  it('encodeKittyGraphic devuelve array vacío si el buffer está vacío', () => {
    const result = encodeKittyGraphic({
      buffer: Buffer.alloc(0),
      columns: 45,
      rows: 32,
      imageId: 1,
    });

    expect(result).toEqual([]);
  });

  it('deleteKittyImage genera la secuencia de borrado para un ID específico', () => {
    const seq = deleteKittyImage(42);
    expect(seq).toContain('i=42');
    expect(seq).toContain('a=d');
    expect(seq.slice(0, 3)).toBe('\x1b_G');
    expect(seq.slice(-2)).toBe('\x1b\\');
  });

  it('deleteAllKittyImages genera la secuencia de borrado de todas las imágenes', () => {
    const seq = deleteAllKittyImages();
    expect(seq).toContain('a=d');
    expect(seq.slice(0, 3)).toBe('\x1b_G');
    expect(seq.slice(-2)).toBe('\x1b\\');
  });
});
