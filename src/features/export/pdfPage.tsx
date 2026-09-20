import { Image, View, StyleSheet } from '@react-pdf/renderer';
import type { Tournament, PaperSize } from '../../types';

/**
 * The page sizes a tournament can be printed on.
 *
 * `id` is what @react-pdf takes as a `<Page size>`; the dimensions are the same
 * sizes in points, and documents that lay themselves out by hand - the bracket -
 * need them to know how much room they actually have.
 */
export const PAPER_SIZES = {
  letter: { id: 'LETTER', width: 612, height: 792 },
  tabloid: { id: 'TABLOID', width: 792, height: 1224 },
  a4: { id: 'A4', width: 595.28, height: 841.89 },
  a3: { id: 'A3', width: 841.89, height: 1190.55 },
} as const satisfies Record<PaperSize, { id: string; width: number; height: number }>;

export const PAPER_SIZE_OPTIONS = Object.keys(PAPER_SIZES) as PaperSize[];

/**
 * US Letter, not A4: the clubs running this print on US paper. A tournament
 * restored from a backup written before the setting existed has no value at
 * all, so the fallback has to live here rather than only in the schema.
 */
export const DEFAULT_PAPER_SIZE: PaperSize = 'letter';

function paperOf(tournament: Pick<Tournament, 'paperSize'>) {
  return PAPER_SIZES[tournament.paperSize] ?? PAPER_SIZES[DEFAULT_PAPER_SIZE];
}

type Orientation = 'portrait' | 'landscape';

/** The `size` and `orientation` props for a `<Page>` of this tournament. */
export function pageProps(
  tournament: Pick<Tournament, 'paperSize'>,
  orientation: Orientation = 'portrait'
) {
  return { size: paperOf(tournament).id, orientation } as const;
}

/** How much room is left inside the page once its padding is taken off. */
export function contentSize(
  tournament: Pick<Tournament, 'paperSize'>,
  orientation: Orientation,
  padding: number
) {
  const paper = paperOf(tournament);
  const width = orientation === 'landscape' ? paper.height : paper.width;
  const height = orientation === 'landscape' ? paper.width : paper.height;
  return { width: width - padding * 2, height: height - padding * 2 };
}

/**
 * The logo box, in points. The image is fitted inside it rather than filling
 * it, so a tall logo and a wide one both stay clear of the page content; the
 * headers reserve this much room on their right.
 */
export const LOGO_BOX = { width: 90, height: 45 };
export const LOGO_BOX_COMPACT = { width: 70, height: 35 };

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
