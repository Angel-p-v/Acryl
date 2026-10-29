import { defineConfig } from 'vite';
import react from '@vitejs/plugin-react';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const __dirname = path.dirname(fileURLToPath(import.meta.url));

// El build debe salir en ../build (app-media-manager/build) para coincidir
// con frontendDist de src-tauri/tauri.conf.json.
// El dev server corre en el puerto 3000 (devUrl de tauri.conf.json).
export default defineConfig({
  plugins: [react()],
  root: __dirname,
  build: {
    outDir: path.resolve(__dirname, '..', 'build'),
    emptyOutDir: true,
  },
  server: {
    port: 3000,
    strictPort: true,
  },
});
