export interface ModEntity {
  zhName?: string | null
  title?: string | null
  slug?: string | null
}

export interface LocalEntry {
  zhName?: string | null
  fileName: string
  title?: string | null
}

/**
 * Returns the primary display name for a mod entity based on Chinese name setting.
 */
export function modDisplayName(entity: ModEntity, showZh: boolean): string {
  return (showZh && entity.zhName) || entity.title || entity.slug || ''
}

/**
 * Returns the subtitle (English title) when a separate Chinese name is being shown.
 * Returns `null` when `showZh` is false, when no Chinese name exists, or when `zhName === title`.
 */
export function modSubtitle(
  entity: { zhName?: string | null; title?: string | null },
  showZh: boolean,
): string | null {
  if (showZh && entity.zhName && entity.title && entity.zhName !== entity.title) {
    return entity.title
  }
  return null
}

/**
 * Formats local resource entry names for display.
 * Only mods with `showZh` and `zhName` present produce a secondary name.
 * `style === 1` swaps primary and secondary so the jar file name leads.
 */
export function localEntryNames(
  entry: LocalEntry,
  kind: string,
  showZh: boolean,
  style: 0 | 1 = 0,
): { primary: string; secondary: string | null } {
  if (kind === 'mod' && showZh && entry.zhName) {
    return style === 1
      ? { primary: entry.fileName, secondary: entry.zhName }
      : { primary: entry.zhName, secondary: entry.fileName }
  }
  return {
    primary: entry.title || entry.fileName,
    secondary: null,
  }
}
