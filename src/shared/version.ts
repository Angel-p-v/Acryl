import { readFileSync } from 'node:fs';

/** Lee la versión declarada en package.json (funciona en src/ y en dist/). */
export function getVersion(): string {
  try {
    const packageJsonUrl = new URL('../../package.json', import.meta.url);
    const content = readFileSync(packageJsonUrl, 'utf8');
    const parsed: unknown = JSON.parse(content);
    if (typeof parsed === 'object' && parsed !== null && 'version' in parsed) {
      const version = parsed.version;
      if (typeof version === 'string') return version;
    }
  } catch {
    // Si no se puede leer, la CLI sigue funcionando sin número de versión.
  }
  return '0.0.0';
}
