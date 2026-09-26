import { Image, View, StyleSheet } from '@react-pdf/renderer';
import type { Tournament, PaperSize } from '../../types';

/**
 * The page sizes a document can be printed on, in portrait points.
 *
 * Dimensions rather than @react-pdf's size names: the names are only a lookup
 * table into the same numbers, 13x19 has no name in it at all, and one source
 * of truth is what keeps `<Page size>` and the bracket's hand-built geometry
 * from ever disagreeing. `orientation="landscape"` flips whatever it is given,
 * so portrait is the only form that needs storing.
 */
export const PAPER_SIZES = {
  letter: { width: 612, height: 792 },
  legal: { width: 612, height: 1008 },
  tabloid: { width: 792, height: 1224 },
  '13x19': { width: 936, height: 1368 },
  a4: { width: 595.28, height: 841.89 },
  a3: { width: 841.89, height: 1190.55 },
} as const satisfies Record<PaperSize, { width: number; height: number }>;

export const PAPER_SIZE_OPTIONS = Object.keys(PAPER_SIZES) as PaperSize[];

/**
 * US Letter, not A4: the clubs running this print on US paper. It is what each
 * document's size picker opens on, and the fallback if an unknown size ever
 * reaches here.
 */
export const DEFAULT_PAPER_SIZE: PaperSize = 'letter';

function paperOf(paperSize: PaperSize) {
  return PAPER_SIZES[paperSize] ?? PAPER_SIZES[DEFAULT_PAPER_SIZE];
}

type Orientation = 'portrait' | 'landscape';

/** The `size` and `orientation` props for a `<Page>` on this paper. */
export function pageProps(paperSize: PaperSize, orientation: Orientation = 'portrait') {
  const paper = paperOf(paperSize);
  return { size: [paper.width, paper.height] as [number, number], orientation } as const;
}

/** How much room is left inside the page once its padding is taken off. */
export function contentSize(
  paperSize: PaperSize,
  orientation: Orientation,
  padding: number
) {
  const paper = paperOf(paperSize);
  const width = orientation === 'landscape' ? paper.height : paper.width;
  const height = orientation === 'landscape' ? paper.width : paper.height;
  return { width: width - padding * 2, height: height - padding * 2 };
}

/**
 * The logo box, in points. The image is fitted inside it rather than filling
 * it, so a tall logo and a wide one both stay clear of the page content; the
 * headers reserve this much room on their right.
 *
 * The sizes are as large as each header can carry, measured rather than
 * guessed. Width costs nothing but title room, of which there is plenty; it is
 * the height that runs into things, because the box is absolutely positioned
 * and so pushes nothing out of its way:
 *
 * - the court sheet's rule sits 20 pt below the old box, and its header now
 *   carries a `minHeight` so the short "nothing to show" variant cannot let
 *   that rule ride up through the logo;
 * - the standings table starts 34 pt below it;
 * - the bracket is the tight one. A 32-team draw stretches its columns to the
 *   page edge, putting the "Final" round label directly under the logo with
 *   half a point to spare, so the compact box can only grow if the bracket
 *   moves down with it - hence the header margin and `VERTICAL_RESERVE` in
 *   BracketPDF.
 */
export const LOGO_BOX = { width: 116, height: 58 };
export const LOGO_BOX_COMPACT = { width: 88, height: 44 };

const styles = StyleSheet.create({
  frame: {
    position: 'absolute',
    top: 0,
    right: 0,
    alignItems: 'flex-end',
    justifyContent: 'flex-start',
  },
  image: {
    objectFit: 'contain',
  },
});

/**
 * The tournament logo, in the top-right corner of a header.
 *
 * Absolutely positioned so it never pushes the title around, and bounded on
 * both axes so an oversized upload cannot run over the page. Renders nothing
 * when no logo has been set, which is the normal case.
 */
export function PdfLogo({
  tournament,
  box = LOGO_BOX,
}: {
  tournament: Pick<Tournament, 'logo'>;
  box?: { width: number; height: number };
}) {
  if (!tournament.logo) return null;

  return (
    <View style={[styles.frame, box]}>
      <Image src={tournament.logo} style={[styles.image, box]} />
    </View>
  );
}
