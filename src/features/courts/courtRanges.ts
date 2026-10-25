/**
 * Reads a court list the way a print dialog reads a page range: "1,3,5,6-11".
 *
 * This is the preview only - it tells the operator what they have typed while
 * they are still typing. The backend's `parse_court_list` in `courts.rs` is
 * the one that counts, and it refuses the same input on save, so the two must
 * agree on what is valid: spaces ignored, duplicates collapsed, an en dash
 * read as a hyphen, nothing below 1 or above the tournament's court count.
 */

export type CourtListError =
  | { key: 'notANumber'; token: string }
  | { key: 'belowOne' }
  | { key: 'beyondVenue'; court: number; count: number }
  | { key: 'backwards'; from: number; to: number };

export type CourtListResult = { courts: number[] } | { error: CourtListError };

export function parseCourtList(text: string, numberOfCourts: number): CourtListResult {
  const courts = new Set<number>();

  const number = (token: string): number | CourtListError => {
    if (!/^\d+$/.test(token)) return { key: 'notANumber', token };
    const court = Number(token);
    if (court < 1) return { key: 'belowOne' };
    if (court > numberOfCourts) return { key: 'beyondVenue', court, count: numberOfCourts };
    return court;
  };

  for (const raw of text.split(',')) {
    const token = raw.replace(/\s+/g, '').replace(/–/g, '-');
    if (token === '') continue;

    const dash = token.indexOf('-');
    if (dash === -1) {
      const court = number(token);
      if (typeof court !== 'number') return { error: court };
      courts.add(court);
      continue;
    }

    const from = number(token.slice(0, dash));
    if (typeof from !== 'number') return { error: from };
    const to = number(token.slice(dash + 1));
    if (typeof to !== 'number') return { error: to };
    if (from > to) return { error: { key: 'backwards', from, to } };
    for (let court = from; court <= to; court++) courts.add(court);
  }

  return { courts: [...courts].sort((a, b) => a - b) };
}
