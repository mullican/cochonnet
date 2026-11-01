/**
 * Advance widths of Helvetica (the regular weight), in thousandths of an em,
 * from the font's standard metrics.
 *
 * @react-pdf lays text out but gives nothing back, so a document that has to
 * decide in advance what goes on which page - the two-column court sheet -
 * has to measure its own text. Only Helvetica is needed: it is the one font
 * these documents use.
 */
const HELVETICA: Record<string, number> = {
  ' ': 278, '-': 333, '.': 278, ',': 278, '/': 278, "'": 191, '(': 333, ')': 333, ':': 278,
  '0': 556, '1': 556, '2': 556, '3': 556, '4': 556,
  '5': 556, '6': 556, '7': 556, '8': 556, '9': 556,
  A: 667, B: 667, C: 722, D: 722, E: 667, F: 611, G: 778, H: 722, I: 278,
  J: 500, K: 667, L: 556, M: 833, N: 722, O: 778, P: 667, Q: 778, R: 722,
  S: 667, T: 611, U: 722, V: 667, W: 944, X: 667, Y: 667, Z: 611,
  a: 556, b: 556, c: 500, d: 556, e: 556, f: 278, g: 556, h: 556, i: 222,
  j: 222, k: 500, l: 222, m: 833, n: 556, o: 556, p: 556, q: 556, r: 333,
  s: 500, t: 278, u: 556, v: 500, w: 722, x: 500, y: 500, z: 500,
};

/**
 * Anything not in the table, which in practice means an accented letter in a
 * French name. As wide as the widest ordinary capitals, so a guess can only
 * err towards a line breaking early, never towards text overrunning its cell.
 */
const FALLBACK = 778;

/** The width of `text` set in Helvetica at `fontSize`, in points. */
export function textWidth(text: string, fontSize: number): number {
  let units = 0;
  for (const ch of text) units += HELVETICA[ch] ?? FALLBACK;
  return (units / 1000) * fontSize;
}

/**
 * The pieces @react-pdf may break `word` into, as a hyphenation callback
 * returns them. A word that fits comes back whole. One that does not breaks
 * first at its own hyphens - DANCAUSSE-TREHIN becomes DANCAUSSE- and TREHIN -
 * and any piece still too wide is cut by characters, with room left for the
 * hyphen @react-pdf adds at such a cut.
 *
 * The empty strings between hyphenated pieces are break points that add no
 * hyphen of their own. @react-pdf treats a blank piece as a zero-width space
 * and adds a hyphen only between two pieces that both have text in them;
 * without the blank, DANCAUSSE-TREHIN would break as DANCAUSSE--.
 */
export function splitToFit(word: string, fontSize: number, width: number): string[] {
  if (textWidth(word, fontSize) <= width) return [word];
  return word
    .split(/(?<=-)/)
    .flatMap((part, i) => [...(i > 0 ? [''] : []), ...cutToFit(part, fontSize, width)]);
}

function cutToFit(part: string, fontSize: number, width: number): string[] {
  if (textWidth(part, fontSize) <= width) return [part];
  const room = width - textWidth('-', fontSize);
  const pieces: string[] = [];
  let piece = '';
  for (const ch of part) {
    if (piece && textWidth(piece + ch, fontSize) > room) {
      pieces.push(piece);
      piece = '';
    }
    piece += ch;
  }
  pieces.push(piece);
  return pieces;
}

/**
 * How many lines `text` takes in a box `width` points wide, breaking between
 * words, at every newline, and inside a word only where `splitToFit` would.
 */
export function lineCount(text: string, fontSize: number, width: number): number {
  return text
    .split('\n')
    .reduce((sum, line) => sum + paragraphLines(line, fontSize, width), 0);
}

function paragraphLines(text: string, fontSize: number, width: number): number {
  const space = textWidth(' ', fontSize);
  let lines = 1;
  let used = 0;
  for (const word of text.split(' ')) {
    const w = textWidth(word, fontSize);
    if (used > 0 && used + space + w > width) {
      lines += 1;
      used = 0;
    }
    if (used === 0 && w > width) {
      // Each piece on a line of its own. Two short hyphenated pieces may in
      // fact share one, so this can only count too many lines, never too few.
      const pieces = splitToFit(word, fontSize, width).filter(Boolean);
      lines += pieces.length - 1;
      used = textWidth(pieces[pieces.length - 1], fontSize);
    } else {
      used += (used > 0 ? space : 0) + w;
    }
  }
  return lines;
}
