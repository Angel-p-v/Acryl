export type ErrorCode =
  | 'APP_ERROR'
  | 'CONFIG_ERROR'
  | 'DB_ERROR'
  | 'INDEX_ERROR'
  | 'PLAYER_ERROR'
  | 'METADATA_ERROR'
  | 'SELF_UPDATE_ERROR'
  | 'VALIDATION_ERROR'
  | 'NOT_FOUND'
  | 'AMBIGUOUS';

/**
 * Error base de la aplicación. Todos los errores que llegan a la capa de
 * presentación deberían heredar de aquí para poder mostrar un mensaje útil.
 */
export class AppError extends Error {
  readonly code: ErrorCode;

  constructor(message: string, code: ErrorCode = 'APP_ERROR', options?: { cause?: unknown }) {
    super(message, options);
    this.name = new.target.name;
    this.code = code;
  }
}

/** Error de configuración: el archivo no existe, no es YAML válido o no cumple el esquema. */
export class ConfigError extends AppError {
  readonly filePath: string | undefined;

  constructor(message: string, filePath?: string, options?: { cause?: unknown }) {
    super(message, 'CONFIG_ERROR', options);
    this.filePath = filePath;
  }
}

/** Error de la capa de persistencia. */
export class DbError extends AppError {
  constructor(message: string, options?: { cause?: unknown }) {
    super(message, 'DB_ERROR', options);
  }
}

/** Error durante la indexación de la biblioteca. */
export class IndexError extends AppError {
  constructor(message: string, options?: { cause?: unknown }) {
    super(message, 'INDEX_ERROR', options);
  }
}

/** Error al lanzar o configurar un reproductor externo. */
export class PlayerError extends AppError {
  constructor(message: string, options?: { cause?: unknown }) {
    super(message, 'PLAYER_ERROR', options);
  }
}

/** Recurso solicitado inexistente (título, fichero, reproductor...). */
export class NotFoundError extends AppError {
  constructor(message: string, options?: { cause?: unknown }) {
    super(message, 'NOT_FOUND', options);
  }
}

/** Varios recursos coinciden con la consulta y hace falta desambiguar. */
export class AmbiguousError extends AppError {
  constructor(message: string, options?: { cause?: unknown }) {
    super(message, 'AMBIGUOUS', options);
  }
}

/** Error al obtener o procesar metadatos de proveedores externos (TMDB, TVDB, OMDB). */
export class MetadataError extends AppError {
  constructor(message: string, options?: { cause?: unknown }) {
    super(message, 'METADATA_ERROR', options);
  }
}

/** Error durante la auto-actualización (`mm self-update`). */
export class SelfUpdateError extends AppError {
  constructor(message: string, options?: { cause?: unknown }) {
    super(message, 'SELF_UPDATE_ERROR', options);
  }
}

/** Error de validación de argumentos o entrada del usuario. */
export class ValidationError extends AppError {
  constructor(message: string, options?: { cause?: unknown }) {
    super(message, 'VALIDATION_ERROR', options);
  }
}

/** Convierte cualquier valor lanzado en un mensaje legible para el usuario. */
export function formatError(error: unknown): string {
  return describeError(error).message;
}

/** Código y mensaje normalizados de un error, sin depender de su clase concreta. */
export type ErrorInfo = {
  code: ErrorCode;
  message: string;
};

/** Normaliza errores de la aplicación y errores inesperados a un mismo contrato. */
export function describeError(error: unknown): ErrorInfo {
  if (error instanceof AppError) {
    return { code: error.code, message: error.message };
  }
  if (error instanceof Error) {
    return { code: 'APP_ERROR', message: error.message };
  }
  return { code: 'APP_ERROR', message: String(error) };
}

/** Mensaje con el código estable delante, útil para soporte y scripts. */
export function formatErrorWithCode(error: unknown): string {
  const info = describeError(error);
  return `[${info.code}] ${info.message}`;
}
