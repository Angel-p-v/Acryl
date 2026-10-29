import { readFile, readdir } from 'node:fs/promises';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

import { describe, expect, it } from 'vitest';

const SRC = fileURLToPath(new URL('../../src', import.meta.url));

async function sourceFiles(): Promise<{ relativePath: string; content: string }[]> {
  const entries = await readdir(SRC, { recursive: true });
  const files = entries.filter((entry) => entry.endsWith('.ts') || entry.endsWith('.tsx')).sort();

  return Promise.all(
    files.map(async (relativePath) => ({
      relativePath: relativePath.replaceAll(path.sep, '/'),
      content: await readFile(path.join(SRC, relativePath), 'utf8'),
    })),
  );
}

describe('reglas de arquitectura', () => {
  it('core no importa desde ui ni desde commands', async () => {
    for (const { relativePath, content } of await sourceFiles()) {
      if (!relativePath.startsWith('core/')) continue;
      expect(content, relativePath).not.toMatch(/from\s+['"](?:\.\.\/)+(?:ui|commands)\//);
    }
  });

  it('shared no depende de módulos del proyecto', async () => {
    for (const { relativePath, content } of await sourceFiles()) {
      if (!relativePath.startsWith('shared/')) continue;
      expect(content, relativePath).not.toMatch(/from\s+['"]\.{1,2}\//);
    }
  });

  it('solo core/db importa better-sqlite3', async () => {
    for (const { relativePath, content } of await sourceFiles()) {
      if (relativePath.startsWith('core/db/')) continue;
      expect(content, relativePath).not.toContain("from 'better-sqlite3'");
    }
  });

  it('la presentación no escribe SQL', async () => {
    for (const { relativePath, content } of await sourceFiles()) {
      if (!relativePath.startsWith('ui/') && !relativePath.startsWith('commands/')) continue;
      expect(content, relativePath).not.toContain('.prepare(');
    }
  });
});
