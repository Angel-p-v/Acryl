/**
 * Fusión profunda de objetos planos.
 *
 * Reglas:
 * - Los objetos se fusionan clave a clave.
 * - Los arrays se reemplazan por completo (no se concatenan): así el usuario puede
 *   redefinir `library.exclude` sin arrastrar los valores por defecto.
 * - `null` y `undefined` en la capa superior se ignoran, lo que permite usar
 *   `null` para dejar de heredar un valor.
 */
export function mergeDeep<T>(base: T, override: unknown): T {
  if (override === undefined) return base;
  if (override === null) return base;

  if (Array.isArray(base) || Array.isArray(override)) {
    return override as T;
  }

  if (isPlainObject(base) && isPlainObject(override)) {
    const result: Record<string, unknown> = { ...base };
    for (const [key, value] of Object.entries(override)) {
      result[key] = key in base ? mergeDeep(base[key], value) : value;
    }
    return result as T;
  }

  return override as T;
}

/** Indica si el valor es un objeto plano (no un array, no null, no una clase). */
export function isPlainObject(value: unknown): value is Record<string, unknown> {
  if (typeof value !== 'object' || value === null) return false;
  if (Array.isArray(value)) return false;
  const prototype: unknown = Object.getPrototypeOf(value);
  return prototype === Object.prototype || prototype === null;
}
