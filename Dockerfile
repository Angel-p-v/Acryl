# syntax=docker/dockerfile:1
FROM node:22-alpine AS builder

WORKDIR /app

# Herramientas de compilación necesarias para mejor-sqlite3 (nativo) en Alpine.
RUN apk add --no-cache python3 make g++

# Dependencias (incluye dev para compilar better-sqlite3 y TypeScript)
COPY package*.json ./
RUN npm ci

# Código fuente y compilación
COPY . .
RUN npm run build

# Imagen final: solo runtime
FROM node:22-alpine

WORKDIR /app

# mpv para reproducir desde la CLI/TUI dentro del contenedor (cuando haya
# salida de vídeo disponible; de lo contrario `mm play` degrada con mensaje claro).
# su-exec para bajar privilegios desde el entrypoint (volúmenes montados son root).
RUN apk add --no-cache mpv su-exec

# Usuario no-root para ejecutar mm
RUN addgroup -S mmgroup && adduser -S mmuser -G mmgroup

# Copiar artefactos compilados y módulos de node
COPY --from=builder /app/dist ./dist
COPY --from=builder /app/node_modules ./node_modules
COPY package*.json ./

# Entrypoint: corre como root para poder chownear los volúmenes montados
# (propiedad de root) y luego baja privilegios con su-exec para ejecutar node
# como el usuario no-root mmuser. Se omite la directiva USER para que el
# entrypoint disponga de privilegios de root al arrancar.
COPY docker-entrypoint.sh /usr/local/bin/docker-entrypoint.sh
RUN chmod +x /usr/local/bin/docker-entrypoint.sh

ENTRYPOINT ["docker-entrypoint.sh"]
CMD []
