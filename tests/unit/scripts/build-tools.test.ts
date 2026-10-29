import { describe, expect, it } from 'vitest';

import { findChecksum } from '../../../scripts/build/fetch-node.mjs';
import {
  artifactName,
  bundleName,
  nodeArchiveName,
  nodePlatform,
  normalizePlatform,
} from '../../../scripts/build/platform.mjs';

describe('herramientas de construcción', () => {
  describe('normalizePlatform', () => {
    it('traduce linux y macOS a los nombres de los assets', () => {
      expect(normalizePlatform('linux', 'x64')).toBe('linux-x64');
      expect(normalizePlatform('linux', 'arm64')).toBe('linux-arm64');
      expect(normalizePlatform('darwin', 'arm64')).toBe('macos-arm64');
    });

    it('traduce Windows a windows-x64', () => {
      expect(normalizePlatform('win32', 'x64')).toBe('windows-x64');
    });

    it('rechaza plataformas sin bundle', () => {
      expect(() => normalizePlatform('linux', 'ia32')).toThrow('linux-ia32');
    });
  });

  describe('nodePlatform', () => {
    it('usa el nombre oficial de las distribuciones de Node', () => {
      expect(nodePlatform('linux', 'x64')).toBe('linux-x64');
      expect(nodePlatform('darwin', 'arm64')).toBe('darwin-arm64');
      expect(nodePlatform('win32', 'x64')).toBe('win-x64');
    });
  });

  describe('nombres de artefactos', () => {
    it('incluye versión y plataforma', () => {
      expect(artifactName('1.2.3', 'linux-x64')).toBe('media-manager-cli-1.2.3-linux-x64.tar.gz');
      expect(artifactName('1.2.3', 'windows-x64')).toBe('media-manager-cli-1.2.3-windows-x64.zip');
      expect(bundleName('1.2.3', 'linux-x64')).toBe('media-manager-cli-1.2.3-linux-x64');
      expect(bundleName('1.2.3', 'windows-x64')).toBe('media-manager-cli-1.2.3-windows-x64');
      expect(nodeArchiveName('24.22.1', 'linux', 'arm64')).toBe('node-v24.22.1-linux-arm64.tar.gz');
      expect(nodeArchiveName('24.22.1', 'win32', 'x64')).toBe('node-v24.22.1-win-x64.zip');
    });
  });

  describe('findChecksum', () => {
    const checksums = [
      'aaaa  node-v24.22.1-linux-x64.tar.gz',
      '0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef  node-v24.22.1-linux-arm64.tar.gz',
      'zzzz  node-v24.22.1-darwin-arm64.tar.gz',
    ].join('\n');

    it('devuelve el hash del archivo solicitado', () => {
      expect(findChecksum(checksums, 'node-v24.22.1-linux-arm64.tar.gz')).toBe(
        '0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef',
      );
    });

    it('ignora archivos distintos y hashes mal formados', () => {
      expect(findChecksum(checksums, 'node-v24.22.1-linux-x64.tar.gz')).toBeUndefined();
      expect(findChecksum(checksums, 'node-v24.22.1-darwin-arm64.tar.gz')).toBeUndefined();
      expect(findChecksum('', 'node-v24.22.1-linux-x64.tar.gz')).toBeUndefined();
    });
  });
});
