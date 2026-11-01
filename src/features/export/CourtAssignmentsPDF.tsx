import { Document, Page, Text, View, StyleSheet } from '@react-pdf/renderer';
import type { Tournament, Team, QualifyingRound, GameWithTeams, PaperSize } from '../../types';
import { formatTeamName } from '../../lib/utils';
import type { PDFTranslations } from './pdfTranslations';
import { pageProps, contentSize, PdfLogo, LOGO_BOX } from './pdfPage';
import { TeamLabelPDF, teamLabelStyles } from './TeamLabelPDF';
import { lineCount, splitToFit, textWidth } from './textWidth';

/**
 * The sheet is landscape with the games in two columns, filled left column
 * first. A landscape page holds fewer rows than a portrait one, so a single
 * column would have cost pages; two of them hold half as many again as the
 * old portrait sheet did.
 *
 * @react-pdf has no columns of its own, so the split is decided here, before
 * anything is rendered, and that means knowing how tall each row will be. A
 * row is one line unless a name wraps - the longest captains do, and a panache
 * side of three always does - so every row is measured (`lineCount`) against
 * the geometry below. Each constant here is used both by a style and by that
 * measurement, which is what keeps the two from drifting apart.
 */
const ORIENTATION = 'landscape';
const PAGE_PADDING = 30;
const COLUMN_GAP = 24;
const LINE_HEIGHT = 1.2;

const ROW_PADDING_X = 6;
const ROW_PADDING_Y = 7;
const ROW_BORDER = 1;
// Wide enough for the column headings in both languages ("Court", "Piste")
// and for "contre", which is what the French sheet says for "vs".
const COURT_WIDTH = 44;
const VS_WIDTH = 34;
const COURT_FONT = 14;
// 11 rather than 12 because the number chip takes ~28pt from every name, and
// at 12 the longer surnames in a real field (BRETTSCHNEIDER) no longer fit.
const TEAM_FONT = 11;

/**
 * The number chip beside each name, as `TeamLabelPDF` draws it at TEAM_FONT.
 * Read from its own styles, so the chip can change without this sheet
 * mismeasuring it.
 */
const CHIP = teamLabelStyles(TEAM_FONT);
const CHIP_FONT = CHIP.chipText.fontSize as number;
const CHIP_PADDING_X = CHIP.chip.paddingHorizontal as number;
const CHIP_MIN_WIDTH = CHIP.chip.minWidth as number;
const CHIP_GAP = CHIP.chip.marginRight as number;
/**
 * The chip's height. Its number is set at Helvetica's natural line height,
 * about 1.16, so LINE_HEIGHT here is a slight overestimate - the safe side.
 */
const CHIP_HEIGHT = CHIP_FONT * LINE_HEIGHT + (CHIP.chip.paddingVertical as number) * 2;

/**
 * Room the chip takes from the name beside it. Measured in regular Helvetica
 * although the number is bold: every digit is 556 units wide in both.
 */
const chipWidth = (teamNumber: number) =>
  Math.max(CHIP_MIN_WIDTH, textWidth(String(teamNumber), CHIP_FONT) + CHIP_PADDING_X * 2) +
  CHIP_GAP;

const HEADER_FONTS = { name: 14, subtitle: 10, title: 24 };
const HEADER_GAP = 2;
const HEADER_PADDING_BOTTOM = 8;
const HEADER_RULE = 2;
const HEADER_MARGIN = 16;
const HEADER_MIN_HEIGHT = LOGO_BOX.height + 8;
/** The page header, as laid out: its three lines, padding and rule, or its floor. */
const HEADER_HEIGHT =
  Math.max(
    LINE_HEIGHT * (HEADER_FONTS.name + HEADER_FONTS.subtitle + HEADER_FONTS.title) +
      HEADER_GAP * 2 +
      HEADER_PADDING_BOTTOM +
      HEADER_RULE,
    HEADER_MIN_HEIGHT
  ) + HEADER_MARGIN;

const TABLE_HEADER_FONT = 10;
const TABLE_HEADER_PADDING_Y = 5;
const TABLE_HEADER_HEIGHT = TABLE_HEADER_FONT * LINE_HEIGHT + TABLE_HEADER_PADDING_Y * 2 + 1;

/**
 * Room held back from every column. The measurement can only overestimate a
 * row, never underestimate it, but a column that comes out a point too tall
 * spills its last game onto a page of its own, so the margin is cheap.
 */
const COLUMN_SLACK = 10;

const styles = StyleSheet.create({
  page: {
    padding: PAGE_PADDING,
    fontFamily: 'Helvetica',
  },
  header: {
    marginBottom: HEADER_MARGIN,
    borderBottomWidth: HEADER_RULE,
    borderBottomColor: '#000',
    paddingBottom: HEADER_PADDING_BOTTOM,
    // Keeps the game title clear of the logo box in the top-right corner.
    paddingRight: LOGO_BOX.width + 10,
    // The "nothing to show" page has no game title, so its header is a good
    // deal shorter than the others - short enough that the rule below it would
    // otherwise cut straight through the logo, which is absolutely positioned
    // and pushes nothing down. This holds the rule under the logo instead.
    minHeight: HEADER_MIN_HEIGHT,
  },
  tournamentName: {
    fontSize: HEADER_FONTS.name,
    lineHeight: LINE_HEIGHT,
    color: '#444',
  },
  subtitle: {
    fontSize: HEADER_FONTS.subtitle,
    lineHeight: LINE_HEIGHT,
    color: '#888',
    textTransform: 'uppercase',
    marginTop: HEADER_GAP,
  },
  roundTitle: {
    fontSize: HEADER_FONTS.title,
    lineHeight: LINE_HEIGHT,
    fontWeight: 'bold',
    marginTop: HEADER_GAP,
  },
  columns: {
    flexDirection: 'row',
  },
  column: {
    flex: 1,
  },
  secondColumn: {
    marginLeft: COLUMN_GAP,
  },
  tableHeader: {
    flexDirection: 'row',
    backgroundColor: '#e0e0e0',
    borderBottomWidth: 1,
    borderBottomColor: '#000',
    paddingVertical: TABLE_HEADER_PADDING_Y,
    paddingHorizontal: ROW_PADDING_X,
  },
  headerText: {
    fontSize: TABLE_HEADER_FONT,
    lineHeight: LINE_HEIGHT,
    fontWeight: 'bold',
    textTransform: 'uppercase',
    color: '#333',
  },
  row: {
    flexDirection: 'row',
    borderBottomWidth: ROW_BORDER,
    borderBottomColor: '#ccc',
    paddingVertical: ROW_PADDING_Y,
    paddingHorizontal: ROW_PADDING_X,
    alignItems: 'center',
  },
  courtCol: {
    width: COURT_WIDTH,
    fontSize: COURT_FONT,
    lineHeight: LINE_HEIGHT,
    fontWeight: 'bold',
  },
  teamCol: {
    flex: 1,
  },
  teamText: {
    fontSize: TEAM_FONT,
    lineHeight: LINE_HEIGHT,
  },
  teamName: {
    lineHeight: LINE_HEIGHT,
  },
  vsCol: {
    width: VS_WIDTH,
    fontSize: 9,
    lineHeight: LINE_HEIGHT,
    color: '#666',
    textAlign: 'center',
  },
  byeText: {
    color: '#999',
    fontStyle: 'italic',
  },
  sitouts: {
    flexDirection: 'row',
    flexWrap: 'wrap',
    alignItems: 'center',
    marginTop: 12,
  },
  sitoutLabel: {
    fontSize: 10,
    color: '#666',
    marginRight: 6,
    marginBottom: 4,
  },
  sitout: {
    marginRight: 10,
    marginBottom: 4,
  },
  sitoutName: {
    // TeamLabelPDF lets the name fill its row; here each label is only as
    // wide as its own name, so the list can wrap like a sentence.
    flexGrow: 0,
    flexBasis: 'auto',
    color: '#666',
  },
  emptyText: {
    fontSize: 12,
    color: '#666',
    marginTop: 20,
  },
});

/**
 * Names break between words, and inside one only when it is wider than the
 * whole cell. @react-pdf hyphenates by default, which turns DANCAUSSE into
 * DAN-CAUSSE whenever the line happens to end there - no help to a player
 * looking for their own name. A word that cannot fit at all still has to be
 * broken, though, or it runs on into the next cell: at its own hyphen if it
 * has one, and only failing that mid-word (`splitToFit`).
 */
const wholeWords = (width: number) => (word: string) => splitToFit(word, TEAM_FONT, width);

/**
 * One side of a game as printed: the teams on it, each under its number -
 * one for a registered team, every player for a panache side - or a word in
 * place of a team, for an empty slot or a bye.
 */
type Side = { teams: Team[] } | { text: string; isBye?: boolean };

/** One game as it will be printed, with the height its row will take. */
interface SheetRow {
  game: GameWithTeams;
  side1: Side;
  side2: Side;
  height: number;
}

/** A page of the sheet: the rows in its left column, then its right. */
type SheetPage = [SheetRow[], SheetRow[]];

const total = (rows: SheetRow[]) => rows.reduce((sum, row) => sum + row.height, 0);

/**
 * Deals the rows into columns of at most `capacity` points, two to a page.
 *
 * Columns fill in order, so courts read down the left column and on down the
 * right. The last page is then evened out: a round of twenty games is ten and
 * ten rather than a full column and a stub, and the left column is the longer
 * one when they cannot match.
 */
function paginate(rows: SheetRow[], capacity: number): SheetPage[] {
  const columns: SheetRow[][] = [];
  let column: SheetRow[] = [];
  let used = 0;
  for (const row of rows) {
    // A row taller than a whole column still has to go somewhere.
    if (column.length > 0 && used + row.height > capacity) {
      columns.push(column);
      column = [];
      used = 0;
    }
    column.push(row);
    used += row.height;
  }
  columns.push(column);

  const pages: SheetPage[] = [];
  for (let i = 0; i < columns.length; i += 2) {
    pages.push([columns[i], columns[i + 1] ?? []]);
  }

  const last = pages[pages.length - 1];
  const rest = [...last[0], ...last[1]];
  let best = { split: last[0].length, tallest: Infinity, rightLonger: true };
  for (let split = 0; split <= rest.length; split++) {
    const left = total(rest.slice(0, split));
    const right = total(rest.slice(split));
    const tallest = Math.max(left, right);
    const rightLonger = right > left;
    if (
      tallest < best.tallest ||
      (tallest === best.tallest && best.rightLonger && !rightLonger)
    ) {
      best = { split, tallest, rightLonger };
    }
  }
  pages[pages.length - 1] = [rest.slice(0, best.split), rest.slice(best.split)];

  return pages;
}

interface CourtAssignmentsPDFProps {
  tournament: Tournament;
  /** The sheet this run is being printed on, chosen at the print button. */
  paperSize: PaperSize;
  teams: Team[];
  rounds: QualifyingRound[];
  games: GameWithTeams[];
  /** Panache only: players resting each round, keyed by round id. */
  sitouts?: Record<string, Team[]>;
  translations: PDFTranslations;
}

export function CourtAssignmentsPDF({
  tournament,
  paperSize,
  teams,
  rounds,
  games,
  sitouts,
  translations: t,
}: CourtAssignmentsPDFProps) {
  /**
   * A panache game names a drawn side; every other format names a registered team.
   *
   * A side puts each player on a line of their own. Run together, as on
   * screen, a triple wraps wherever the column happens to end, which is
   * usually in the middle of somebody's name.
   */
  const getSide = (game: GameWithTeams, which: 1 | 2): Side => {
    if (which === 2 && game.isBye) return { text: t.bye, isBye: true };
    const side = which === 1 ? game.side1 : game.side2;
    if (side) return side.members.length > 0 ? { teams: side.members } : { text: t.tbd };
    const teamId = which === 1 ? game.team1Id : game.team2Id;
    const team = teamId ? teams.find((team) => team.id === teamId) : undefined;
    return team ? { teams: [team] } : { text: t.tbd };
  };

  const content = contentSize(paperSize, ORIENTATION, PAGE_PADDING);
  const columnWidth = (content.width - COLUMN_GAP) / 2;
  const teamWidth = (columnWidth - ROW_PADDING_X * 2 - COURT_WIDTH - VS_WIDTH) / 2;
  const columnCapacity = content.height - HEADER_HEIGHT - TABLE_HEADER_HEIGHT - COLUMN_SLACK;
  /** What is left for a name once its number chip has taken its share. */
  const nameWidth = (team: Team) => teamWidth - chipWidth(team.teamNumber);

  const sideHeight = (side: Side) => {
    const lineHeight = TEAM_FONT * LINE_HEIGHT;
    if ('text' in side) return lineCount(side.text, TEAM_FONT, teamWidth) * lineHeight;
    return side.teams.reduce((sum, team) => {
      const lines = lineCount(formatTeamName(team.captain), TEAM_FONT, nameWidth(team));
      return sum + Math.max(CHIP_HEIGHT, lines * lineHeight);
    }, 0);
  };

  const toRow = (game: GameWithTeams): SheetRow => {
    const side1 = getSide(game, 1);
    const side2 = getSide(game, 2);
    const textHeight = Math.max(
      COURT_FONT * LINE_HEIGHT,
      sideHeight(side1),
      sideHeight(side2)
    );
    return { game, side1, side2, height: textHeight + ROW_PADDING_Y * 2 + ROW_BORDER };
  };

  const sideCell = (side: Side) => (
    <View style={styles.teamCol}>
      {'text' in side ? (
        <Text
          style={side.isBye ? [styles.teamText, styles.byeText] : styles.teamText}
          hyphenationCallback={wholeWords(teamWidth)}
        >
          {side.text}
        </Text>
      ) : (
        side.teams.map((team) => (
          <TeamLabelPDF
            key={team.id}
            team={team}
            fontSize={TEAM_FONT}
            fallback={t.tbd}
            nameStyle={styles.teamName}
            hyphenationCallback={wholeWords(nameWidth(team))}
          />
        ))
      )}
    </View>
  );

  const header = (round: QualifyingRound) => (
    <View style={styles.header} fixed>
      <PdfLogo tournament={tournament} />
      <Text style={styles.tournamentName}>{tournament.name}</Text>
      <Text style={styles.subtitle}>{t.courtAssignments}</Text>
      <Text style={styles.roundTitle}>
        {/* The operator's word for one pass of the whole field is a
            game, not a round - a round is what a bracket has. */}
        {round.isFinal ? t.final : `${t.game} ${round.roundNumber}`}
      </Text>
    </View>
  );

  const column = (rows: SheetRow[], second: boolean) => (
    <View style={second ? [styles.column, styles.secondColumn] : styles.column}>
      {rows.length > 0 && (
        <>
          <View style={styles.tableHeader}>
            <Text style={[styles.courtCol, styles.headerText]}>{t.court}</Text>
            <Text style={[styles.teamCol, styles.headerText]}>{t.team}</Text>
            <Text style={styles.vsCol} />
            <View style={styles.teamCol} />
          </View>
          {rows.map(({ game, side1, side2 }) => (
            <View key={game.id} style={styles.row} wrap={false}>
              <Text style={styles.courtCol}>{game.courtNumber}</Text>
              {sideCell(side1)}
              <Text style={styles.vsCol}>{game.isBye ? '' : t.vs}</Text>
              {sideCell(side2)}
            </View>
          ))}
        </>
      )}
    </View>
  );

  const sortedRounds = [...rounds].sort((a, b) => a.roundNumber - b.roundNumber);

  return (
    <Document>
      {sortedRounds.length === 0 && (
        // No rounds means no pages, and a zero-page PDF is a file some viewers
        // will not open and a printer cannot take.
        <Page {...pageProps(paperSize, ORIENTATION)} style={styles.page}>
          <View style={styles.header}>
            <PdfLogo tournament={tournament} />
            <Text style={styles.tournamentName}>{tournament.name}</Text>
            <Text style={styles.subtitle}>{t.nothingToShow}</Text>
          </View>
        </Page>
      )}
      {sortedRounds.map((round) => {
        const roundGames = games
          .filter((g) => g.roundId === round.id)
          .sort((a, b) => a.courtNumber - b.courtNumber);
        const roundSitouts = sitouts?.[round.id] ?? [];

        if (roundGames.length === 0) {
          return (
            <Page key={round.id} {...pageProps(paperSize, ORIENTATION)} style={styles.page}>
              {header(round)}
              <Text style={styles.emptyText}>{t.tbd}</Text>
            </Page>
          );
        }

        const pages = paginate(roundGames.map(toRow), columnCapacity);

        return pages.map(([left, right], index) => (
          <Page
            key={`${round.id}-${index}`}
            {...pageProps(paperSize, ORIENTATION)}
            style={styles.page}
            wrap
          >
            {header(round)}
            <View style={styles.columns}>
              {column(left, false)}
              {column(right, true)}
            </View>

            {index === pages.length - 1 && roundSitouts.length > 0 && (
              <View style={styles.sitouts}>
                <Text style={styles.sitoutLabel}>{t.sittingOut}:</Text>
                {roundSitouts.map((player) => (
                  <TeamLabelPDF
                    key={player.id}
                    team={player}
                    fontSize={10}
                    fallback={t.tbd}
                    style={styles.sitout}
                    nameStyle={styles.sitoutName}
                  />
                ))}
              </View>
            )}
          </Page>
        ));
      })}
    </Document>
  );
}
