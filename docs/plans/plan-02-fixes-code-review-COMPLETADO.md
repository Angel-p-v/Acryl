# Plan de Acción: Corrección de Errores del Code Review

**Fecha:** 2026-10-23  
**Estado:** ✅ Completado (todos los 8 issues implementados en código y verificados con `npm run verify`)  
**Branch sugerida:** `fix/code-review-issues` (ya aplicada en `main`)

---

## Resumen de Prioridades

| #   | Issue                                             | Severidad  | Esfuerzo | Archivos                                              |
| --- | ------------------------------------------------- | ---------- | -------- | ----------------------------------------------------- |
| 1   | install.sh: `find` operator precedence bug        | 🔴 Crítico | 5 min    | `install.sh`                                          |
| 2   | Add `countFavorites` to LibraryService            | 🟠 Alto    | 30 min   | `library/service.ts`, `library/open.ts`, `ui/App.tsx` |
| 3   | mediaRepo.ts: SQL injection + parameter limit     | 🟠 Alto    | 1 hr     | `db/repositories/mediaRepo.ts`                        |
| 4   | Unit tests for filter logic                       | 🟠 Alto    | 1 hr     | `tests/unit/db/mediaRepo-filters.test.ts` (nuevo)     |
| 5   | App.tsx: use `countFavorites` instead of `list()` | 🟡 Medio   | 15 min   | `ui/App.tsx`                                          |
| 6   | install.sh: quarantine all executables            | 🟡 Medio   | 10 min   | `install.sh`                                          |
| 7   | Document watched/unwatched behavior               | 🟡 Medio   | 10 min   | `mediaRepo.ts` (JSDoc)                                |
| 8   | Benchmarks use migration schema                   | 🟢 Bajo    | 30 min   | `tests/bench/playback.bench.ts`                       |

---

## Detalle por Issue

### 1. install.sh: Fix `find` command (CRÍTICO)

**Ubicación:** `install.sh:472`

**Problema:** Precedencia de operadores `-o` vs `-a` en `find`

```bash
# Actual (buggy):
find "$STAGE_DIR" -type f -name 'node' -o -name 'mm'

# Corrección:
find "$STAGE_DIR" -type f \( -name 'node' -o -name 'mm' \)
```

**Validación:** `bash -n install.sh` y test manual en macOS runner

---

### 2. Add `countFavorites` to LibraryService (ALTO)

**Ubicaciones:**

- `src/core/library/service.ts` - interfaz + implementación
- `src/core/library/open.ts` - inyección de `preferenceRepo`
- `src/ui/App.tsx` - uso del nuevo método

**Cambios:**

```typescript
// LibraryService.ts
export type LibraryService = {
  // ... existentes
  countFavorites(): number;
};

// Implementación:
countFavorites(): number {
  // Necesita acceso a preferenceRepo - añadir a LibraryRepositories
}

// LibraryRepositories agregar:
preferenceRepo: PreferenceRepository;
```

---

### 3. mediaRepo.ts: Fix IN clause SQL issues (ALTO)

**Ubicación:** `src/core/db/repositories/mediaRepo.ts:193-233`

**Problemas a resolver:**

1. **Límite de parámetros SQLite (999):** Chunk IDs en batches de 500
2. **Inyección SQL (patrón riesgoso):** Usar `?` parameters con `IN (?, ?, ?)`
3. **Hardcoded 10000:** Hacer configurable o usar subquery
4. **unwatched logic:** Clarificar/documentar comportamiento

**Implementación sugerida:**

```typescript
// Helper para chunk array
function chunkArray<T>(arr: T[], size: number): T[][] {
  const chunks: T[][] = [];
  for (let i = 0; i < arr.length; i += size) {
    chunks.push(arr.slice(i, i + size));
  }
  return chunks;
}

// En list():
const favChunks = chunkArray(Array.from(favIds), 500);
const placeholders = favChunks.map(() => '(?)').join(',');
conditions.push(`m.id IN (${placeholders})`);
// params.favIds = favChunks.flat();
```

---

### 4. Unit tests for filter logic (ALTO)

**Archivo nuevo:** `tests/unit/db/mediaRepo-filters.test.ts`

**Casos a cubrir:**

- `favorites: true` con/sin preferenceRepo
- `inProgress: true` con/sin playbackRepo
- `watched: true` - títulos con completedAt
- `unwatched: true` - títulos sin completedAt
- Combinaciones: `type + favorites`, `favorites + inProgress`
- Edge cases: IDs > 999 (chunking), arrays vacíos, deps faltantes
- SQL generado correcto (snapshot testing)

---

### 5. App.tsx: Use `countFavorites` (MEDIO)

**Ubicación:** `src/ui/App.tsx:83-108`

**Cambio:**

```typescript
// Antes:
const favItems = library.list({ favorites: true, limit: 10000 });
favoritesCount = favItems.length;

// Después:
favoritesCount = library.countFavorites();
```

Requiere que #2 esté completado primero.

---

### 6. install.sh: Quarantine all executables (MEDIO)

**Ubicación:** `install.sh:468-477`

**Mejora:** Remover cuarentena de todos los ejecutables, no solo `node` y `mm`

```bash
find "$STAGE_DIR" -type f -perm -u+x -exec xattr -d com.apple.quarantine {} \; 2>/dev/null || true
```

---

### 7. Document watched/unwatched behavior (MEDIO)

**Ubicación:** `src/core/db/repositories/mediaRepo.ts` JSDoc

Añadir comentario explícito:

```typescript
/**
 * watched: título tiene AL MENOS UN archivo con completedAt !== null
 * unwatched: título NO TIENE archivos con completedAt !== null
 *            (incluye títulos nunca reproducidos y parcialmente vistos sin completar)
 */
```

---

### 8. Benchmarks use migration schema (BAJO)

**Ubicación:** `tests/bench/playback.bench.ts`

Refactorizar para usar `openLibrary(':memory:')` y migraciones reales en lugar de `db.exec()` manual.

---

## Orden de Ejecución Recomendado

```
1. install.sh find bug (independiente, crítico)
   ↓
2. Add countFavorites + App.tsx fix (bloquea #5)
   ↓
3. mediaRepo.ts IN clause fix (independiente)
   ↓
4. Unit tests para mediaRepo (valida #3)
   ↓
5. install.sh quarantine all executables
   ↓
6. JSDoc watched/unwatched
   ↓
7. Benchmarks refactor
```

---

## Criterios de Aceptación (DoD)

- [ ] `npm run verify` pasa (lint, format, typecheck, tests)
- [ ] `npm run test` sin regresiones (412+ tests pass)
- [ ] Cobertura `mediaRepo.ts` ≥ 85% en líneas nuevas
- [ ] `install.sh` pasa shellcheck y test manual macOS
- [ ] TUI muestra contador Favoritos correcto sin `list({limit: 10000})`
- [ ] No warnings de ESLint/TypeScript nuevos

---

## Notas

- El test de estrés ARM64 (`stress.integration.test.ts`) falla por budget de rendimiento pre-existente; **no bloquea** este plan
- Cada fix debe ir en commit separado (política de commits pequeños)
- Actualizar `CHANGELOG.md` / `PLAN.md` solo en commit final del plan
