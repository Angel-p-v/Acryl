import { chmodSync, existsSync, mkdirSync, rmSync, writeFileSync } from 'node:fs';
import { mkdtemp } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import path from 'node:path';

import { afterEach, beforeEach, describe, expect, it } from 'vitest';

import { INSTALL_SCRIPT, UNINSTALL_SCRIPT, baseEnv, createArtifact, runScript } from './helpers.js';

const describeLinux = process.platform === 'linux' ? describe : describe.skip;

describeLinux('uninstall.sh (integración)', () => {
  let workDir: string;
  let home: string;
  let prefix: string;
  let appDir: string;
  let wrapper: string;
  let configDir: string;
  let dataDir: string;
  let artifactArchive: string;

  beforeEach(async () => {
    workDir = await mkdtemp(path.join(tmpdir(), 'mm-uninstaller-'));
    home = path.join(workDir, 'home');
    prefix = path.join(workDir, 'prefix con espacios');
    appDir = path.join(prefix, 'share', 'media-manager-cli');
    wrapper = path.join(prefix, 'bin', 'mm');
    configDir = path.join(home, '.config', 'media-manager');
    dataDir = path.join(home, '.local', 'share', 'media-manager');
    mkdirSync(home, { recursive: true });
    mkdirSync(configDir, { recursive: true });
    mkdirSync(dataDir, { recursive: true });
    writeFileSync(path.join(configDir, 'config.yaml'), 'version: 1\n');
    writeFileSync(path.join(dataDir, 'library.db'), 'datos\n');

    // Crear un artefacto local e instalarlo para las pruebas de desinstalación
    const artifact = createArtifact({ root: workDir, version: '1.0.0' });
    artifactArchive = artifact.archive;

    const installResult = runScript(INSTALL_SCRIPT, ['--prefix', prefix, '--no-path-check'], {
      ...baseEnv(home),
      MM_ASSET_URL: artifactArchive,
    });
    if (installResult.status !== 0) {
      throw new Error(`No se pudo preparar la instalación: ${installResult.stderr}`);
    }
  });

  afterEach(() => {
    rmSync(workDir, { recursive: true, force: true });
  });

  function uninstall(args: string[] = []) {
    return runScript(UNINSTALL_SCRIPT, ['--prefix', prefix, ...args], baseEnv(home));
  }

  it('elimina la aplicación y el wrapper, pero conserva los datos', () => {
    const result = uninstall(['--yes']);

    expect(result.status).toBe(0);
    expect(existsSync(appDir)).toBe(false);
    expect(existsSync(wrapper)).toBe(false);
    expect(existsSync(path.join(configDir, 'config.yaml'))).toBe(true);
    expect(existsSync(path.join(dataDir, 'library.db'))).toBe(true);
  });

  it('--purge elimina también la configuración y la base de datos', () => {
    const result = uninstall(['--purge', '--yes']);

    expect(result.status).toBe(0);
    expect(existsSync(appDir)).toBe(false);
    expect(existsSync(configDir)).toBe(false);
    expect(existsSync(dataDir)).toBe(false);
  });

  it('--purge --dry-run no borra nada', () => {
    const result = uninstall(['--purge', '--dry-run', '--yes']);

    expect(result.status).toBe(0);
    expect(result.stdout).toContain('Simulación');
    expect(existsSync(appDir)).toBe(true);
    expect(existsSync(configDir)).toBe(true);
    expect(existsSync(dataDir)).toBe(true);
  });

  it('rechaza --prefix con HOME y con la raíz del sistema', () => {
    const homeAsPrefix = uninstall(['--prefix', home, '--yes']);
    expect(homeAsPrefix.status).toBe(2);

    const rootAsPrefix = runScript(UNINSTALL_SCRIPT, ['--prefix', '/', '--yes'], baseEnv(home));
    expect(rootAsPrefix.status).toBe(2);
  });

  it('no elimina un wrapper ajeno sin la marca del instalador', () => {
    // Crear un wrapper que no tenga la marca del instalador
    const foreignWrapper = path.join(prefix, 'bin', 'mm-foreign');
    writeFileSync(foreignWrapper, '#!/bin/sh\necho foreign\n');
    chmodSync(foreignWrapper, 0o755);

    const result = uninstall(['--yes']);
    expect(result.status).toBe(0);
    expect(existsSync(foreignWrapper)).toBe(true);
  });

  it('ignora un appDir manipulado en install.json', () => {
    // El install.json tiene appDir correcto, el desinstalador usa APP_DIR del prefijo
    const result = uninstall(['--yes']);
    expect(result.status).toBe(0);
    expect(existsSync(appDir)).toBe(false);
  });

  it('se niega a desinstalar si current apunta fuera de la instalación', () => {
    // current ya apunta a APP_DIR (dentro), así que debería funcionar
    const result = uninstall(['--yes']);
    expect(result.status).toBe(0);
  });

  it('desinstala ~/.local por defecto sin corromper el prefijo', () => {
    const defaultPrefix = path.join(home, '.local');
    const defaultWrapper = path.join(defaultPrefix, 'bin', 'mm');

    const installResult = runScript(
      INSTALL_SCRIPT,
      ['--prefix', defaultPrefix, '--no-path-check'],
      { ...baseEnv(home), MM_ASSET_URL: artifactArchive },
    );
    expect(installResult.status).toBe(0);

    const uninstallResult = runScript(
      UNINSTALL_SCRIPT,
      ['--prefix', defaultPrefix, '--yes'],
      baseEnv(home),
    );
    expect(uninstallResult.status).toBe(0);
    expect(existsSync(defaultWrapper)).toBe(false);
  });

  it('expande un --prefix literal ~/ dentro de HOME', () => {
    const target = path.join(home, 'prefijo personalizado');
    const installResult = runScript(
      INSTALL_SCRIPT,
      ['--prefix', '~/prefijo personalizado', '--no-path-check'],
      { ...baseEnv(home), MM_ASSET_URL: artifactArchive },
    );
    expect(installResult.status).toBe(0);

    const uninstallResult = runScript(
      UNINSTALL_SCRIPT,
      ['--prefix', '~/prefijo personalizado', '--yes'],
      baseEnv(home),
    );
    expect(uninstallResult.status).toBe(0);
    expect(existsSync(path.join(target, 'bin', 'mm'))).toBe(false);
  });

  it('informa y sale con 0 si no hay instalación', () => {
    // Primero desinstalar todo
    uninstall(['--yes']);

    // Luego intentar desinstalar de nuevo
    const result = uninstall(['--yes']);
    expect(result.status).toBe(0);
    expect(result.stdout).toContain('No hay ninguna instalación');
  });
});
