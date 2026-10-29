import { DbError } from '../../../shared/errors.js';

import type { Db } from '../connection.js';

export type TagRow = {
  id: number;
  name: string;
  color: string | null;
  createdAt: string;
};

type TagDbRow = {
  id: number;
  name: string;
  color: string | null;
  created_at: string;
};

export type CreateTagInput = {
  name: string;
  color?: string;
};

export type UpdateTagInput = {
  name?: string;
  color?: string;
};

/** Repositorio de etiquetas (tags) y su asignación a títulos. */
export type TagRepository = {
  /** Crea un nuevo tag. Lanza si el nombre ya existe. */
  create(input: CreateTagInput): TagRow;
  /** Obtiene un tag por id. */
  get(id: number): TagRow | undefined;
  /** Obtiene un tag por nombre. */
  getByName(name: string): TagRow | undefined;
  /** Lista todos los tags ordenados por nombre. */
  list(): TagRow[];
  /** Actualiza un tag. Devuelve false si no existe. */
  update(id: number, input: UpdateTagInput): boolean;
  /** Elimina un tag y todas sus asignaciones. Devuelve false si no existe. */
  remove(id: number): boolean;
  /** Asigna un tag a un medio (no duplica si ya está asignado). */
  assign(mediaId: number, tagId: number): void;
  /** Quita un tag de un medio. */
  unassign(mediaId: number, tagId: number): void;
  /** Quita todos los tags de un medio. */
  unassignAll(mediaId: number): void;
  /** Lista los tags de un medio. */
  listByMedia(mediaId: number): TagRow[];
  /** Lista los IDs de medios que tienen un tag. */
  listMediaByTag(tagId: number): number[];
  /** Cuenta cuántos medios tienen un tag. */
  countByTag(tagId: number): number;
};

/** Crea el repositorio de etiquetas sobre una conexión SQLite dada. */
export function createTagRepository(db: Db): TagRepository {
  const createdAt = new Date().toISOString();

  const createStatement = db.prepare(`
    INSERT INTO tags (name, color, created_at)
    VALUES (@name, @color, @created_at)
    RETURNING *
  `);
  const getStatement = db.prepare('SELECT * FROM tags WHERE id = ?');
  const getByNameStatement = db.prepare('SELECT * FROM tags WHERE name = ?');
  const listStatement = db.prepare<[], TagDbRow>('SELECT * FROM tags ORDER BY name ASC');
  const updateStatement = db.prepare(`
    UPDATE tags
    SET name = COALESCE(@name, name),
        color = COALESCE(@color, color)
    WHERE id = @id
    RETURNING *
  `);
  const removeStatement = db.prepare('DELETE FROM tags WHERE id = ?');

  const assignStatement = db.prepare(`
    INSERT OR IGNORE INTO media_tags (media_id, tag_id)
    VALUES (@media_id, @tag_id)
  `);
  const unassignStatement = db.prepare(`
    DELETE FROM media_tags WHERE media_id = @media_id AND tag_id = @tag_id
  `);
  const unassignAllStatement = db.prepare(`
    DELETE FROM media_tags WHERE media_id = @media_id
  `);
  const listByMediaStatement = db.prepare<[number], TagDbRow>(`
    SELECT t.* FROM tags t
    JOIN media_tags mt ON mt.tag_id = t.id
    WHERE mt.media_id = ?
    ORDER BY t.name ASC
  `);
  const listMediaByTagStatement = db.prepare<[number], { media_id: number }>(`
    SELECT media_id FROM media_tags WHERE tag_id = ?
  `);
  const countByTagStatement = db.prepare<[number], { total: number }>(`
    SELECT COUNT(*) AS total FROM media_tags WHERE tag_id = ?
  `);

  const toTagRow = (row: TagDbRow): TagRow => ({
    id: row.id,
    name: row.name,
    color: row.color,
    createdAt: row.created_at,
  });

  return {
    create(input: CreateTagInput): TagRow {
      const row = createStatement.get({
        name: input.name,
        color: input.color ?? null,
        created_at: createdAt,
      }) as TagDbRow | undefined;
      if (!row) throw new DbError('No se pudo crear el tag');
      return toTagRow(row);
    },

    get(id: number): TagRow | undefined {
      const row = getStatement.get(id) as TagDbRow | undefined;
      return row ? toTagRow(row) : undefined;
    },

    getByName(name: string): TagRow | undefined {
      const row = getByNameStatement.get(name) as TagDbRow | undefined;
      return row ? toTagRow(row) : undefined;
    },

    list(): TagRow[] {
      return listStatement.all().map(toTagRow);
    },

    update(id: number, input: UpdateTagInput): boolean {
      const row = updateStatement.get({
        id,
        name: input.name ?? null,
        color: input.color ?? null,
      }) as TagDbRow | undefined;
      return row !== undefined;
    },

    remove(id: number): boolean {
      return removeStatement.run(id).changes > 0;
    },

    assign(mediaId: number, tagId: number): void {
      assignStatement.run({ media_id: mediaId, tag_id: tagId });
    },

    unassign(mediaId: number, tagId: number): void {
      unassignStatement.run({ media_id: mediaId, tag_id: tagId });
    },

    unassignAll(mediaId: number): void {
      unassignAllStatement.run(mediaId);
    },

    listByMedia(mediaId: number): TagRow[] {
      return listByMediaStatement.all(mediaId).map(toTagRow);
    },

    listMediaByTag(tagId: number): number[] {
      return listMediaByTagStatement.all(tagId).map((r) => r.media_id);
    },

    countByTag(tagId: number): number {
      return countByTagStatement.get(tagId)?.total ?? 0;
    },
  };
}
