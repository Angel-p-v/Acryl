#!/bin/sh
# Entrypoint del contenedor media-manager.
#
# Los volúmenes montados en tiempo de ejecución (p. ej. mm-data en /data) son
# propiedad de root, pero la app corre como usuario no-root `mmuser`. Si estamos
# como root, chown de /data y /config a mmuser y bajamos privilegios con su-exec
# antes de lanzar node; si ya no somos root, ejecutamos directamente.
set -eu

APP_USER="mmuser"
APP_GROUP="mmgroup"
DATA_DIR="/data"
CONFIG_DIR="/config"

if [ "$(id -u)" = "0" ]; then
  chown -R "$APP_USER:$APP_GROUP" "$DATA_DIR" "$CONFIG_DIR" 2>/dev/null || true
  exec su-exec "$APP_USER:$APP_GROUP" node /app/dist/cli.js "$@"
else
  exec node /app/dist/cli.js "$@"
fi
