import type { Platform } from 'node:process';

export function normalizePlatform(system?: Platform, arch?: string): string;
export function nodePlatform(system?: Platform, arch?: string): string;
export function artifactName(version: string, platform: string): string;
export function bundleName(version: string, platform: string): string;
export function nodeArchiveName(version: string, system?: Platform, arch?: string): string;
