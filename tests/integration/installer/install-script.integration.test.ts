import { spawnSync } from 'node:child_process';
import { existsSync, mkdirSync, readFileSync, rmSync } from 'node:fs';
import { mkdtemp } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import path from 'node:path';

import { afterEach, beforeEach, describe, expect, it } from 'vitest';

import { INSTALL_SCRIPT, baseEnv, createArtifact, runScript } from './helpers.js';

const describeLinux = process.platform === 'linux' ? describe : describe.skip;

describeLinux('install.sh (integración)', () => {
  let workDir: string;
  let home: string;
  let prefix: string;
  let artifactArchive: string;

  beforeEach(async () => {
    workDir = await mkdtemp(path.join(tmpdir(), 'mm-installer-'));
    home = path.join(workDir, 'home');
    prefix = path.join(workDir, 'prefix');
    mkdirSync(home, { recursive: true });

    // Crear un artefacto local para simular un release de GitHub
    const artifact = createArtifact({ root: workDir, version: '1.0.0' });
    artifactArchive = artifact.archive;
  });

  afterEach(() => {
    rmSync(workDir, { recursive: true, force: true });
  });

  it('descarga el artefacto, crea el wrapper y la configuración', () => {
    const result = runScript(INSTALL_SCRIPT, ['--prefix', prefix, '--no-path-check'], {
      ...baseEnv(home),
      MM_ASSET_URL: artifactArchive,
    });

    expect(result.status).toBe(0);

    const wrapper = path.join(prefix, 'bin', 'mm');
    expect(existsSync(wrapper)).toBe(true);

    const version = spawnSync(wrapper, ['--version'], { encoding: 'utf8' });
    expect(version.stdout.trim()).toBe('1.0.0');

    // El archivo de configuración debe existir
    const configFile = path.join(home, '.config', 'media-manager', 'config.yaml');
    expect(existsSync(configFile)).toBe(true);

    const configContent = readFileSync(configFile, 'utf8');
    expect(configContent).toContain('version: 1');
    expect(configContent).toContain('library:');
  });

  it('se niega a reinstalar sin --force', () => {
    const env = { ...baseEnv(home), MM_ASSET_URL: artifactArchive };
    const first = runScript(INSTALL_SCRIPT, ['--prefix', prefix, '--no-path-check'], env);
    expect(first.status).toBe(0);

    const repeated = runScript(INSTALL_SCRIPT, ['--prefix', prefix, '--no-path-check'], env);
    expect(repeated.status).toBe(4);
    expect(repeated.stderr).toContain('--force');
  });

  it('acepta --force para reinstalar', () => {
    const env = { ...baseEnv(home), MM_ASSET_URL: artifactArchive };
    runScript(INSTALL_SCRIPT, ['--prefix', prefix, '--no-path-check'], env);

    const forced = runScript(
      INSTALL_SCRIPT,
      ['--prefix', prefix, '--force', '--no-path-check'],
      env,
    );
    expect(forced.status).toBe(0);
  });

  it('--dry-run no modifica el sistema de archivos', () => {
    const result = runScript(INSTALL_SCRIPT, ['--prefix', prefix, '--dry-run', '--no-path-check'], {
      ...baseEnv(home),
      MM_ASSET_URL: artifactArchive,
    });

    expect(result.status).toBe(0);
    expect(result.stdout).toContain('Simulación');
    expect(existsSync(prefix)).toBe(false);
  });

  it('respeta --bindir personalizado', () => {
    const bindir = path.join(workDir, 'bin-custom');
    const result = runScript(
      INSTALL_SCRIPT,
      ['--prefix', prefix, '--bindir', bindir, '--no-path-check'],
      { ...baseEnv(home), MM_ASSET_URL: artifactArchive },
    );
    expect(result.status).toBe(0);
    expect(existsSync(path.join(bindir, 'mm'))).toBe(true);
  });

  it('no corrompe al usar --prefix /', () => {
    const result = runScript(INSTALL_SCRIPT, ['--prefix', '/'], baseEnv(home));
    expect(result.status).toBe(2);
  });
});
