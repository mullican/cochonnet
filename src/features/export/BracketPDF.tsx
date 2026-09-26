import { Document, Page, Text, View, StyleSheet } from '@react-pdf/renderer';
import type { Tournament, Team, Bracket, BracketMatch, PaperSize } from '../../types';
import { TeamLabelPDF } from './TeamLabelPDF';
import type { PDFTranslations } from './pdfTranslations';
import { pageProps, contentSize, PdfLogo, LOGO_BOX_COMPACT } from './pdfPage';

// Compact dimensions for fitting a bracket on one page
const MATCH_HEIGHT = 24;
const ROUND_GAP = 25;
const LINE_LENGTH = 12; // Horizontal line from match to vertical
const COURT_WIDTH = 24; // The court cell at the left of every match

const PAGE_PADDING = 15;
const MAX_MATCH_WIDTH = 150;
const MIN_MATCH_WIDTH = 70;

// What the header and the round labels take off the top before the bracket
// starts: ~49 for the header and its margin, ~15 for the labels, and some slack
// so a bracket sized to the rest can never spill off the bottom.
const VERTICAL_RESERVE = 79;

/**
 * How wide each match box can be for a bracket of this depth.
 *
 * Every round is one column, so the deeper the bracket the less room each box
 * gets. Sizing to fit rather than using one fixed width is what pays for the
 * court cell without shrinking any text - and it is why a deep bracket is worth
 * printing on larger paper: the columns stop being squeezed to the minimum.
 */
function matchWidthFor(numRounds: number, contentWidth: number): number {
  const fitted = Math.floor((contentWidth - (numRounds - 1) * ROUND_GAP) / numRounds);
  return Math.max(MIN_MATCH_WIDTH, Math.min(MAX_MATCH_WIDTH, fitted));
}

const styles = StyleSheet.create({
  page: {
    padding: PAGE_PADDING,
    fontSize: 8,
    fontFamily: 'Helvetica',
  },
  header: {
    // Deep enough that the logo clears the round labels. A 32-team bracket
    // stretches its columns the full width of the page, so the last one's
    // label sits directly below the logo rather than safely to its left.
    marginBottom: 22,
    // Keeps the title clear of the logo box in the top-right corner.
    paddingRight: LOGO_BOX_COMPACT.width + 8,
  },
  title: {
    fontSize: 12,
    fontWeight: 'bold',
    marginBottom: 2,
  },
  subtitle: {
    fontSize: 9,
    color: '#666',
  },
  bracketContainer: {
    flexDirection: 'row',
  },
  roundColumn: {
    flexDirection: 'column',
  },
  roundLabel: {
    fontSize: 7,
    fontWeight: 'bold',
    color: '#666',
    textAlign: 'center',
    marginBottom: 4,
    height: 10,
  },
  matchRow: {
    flexDirection: 'row',
    alignItems: 'center',
  },
  match: {
    flexDirection: 'row',
    borderWidth: 1,
    borderColor: '#999',
    backgroundColor: '#fff',
  },
  court: {
    width: COURT_WIDTH,
    height: MATCH_HEIGHT,
    justifyContent: 'center',
    alignItems: 'center',
    backgroundColor: '#eeeeee',
    borderRightWidth: 0.5,
    borderRightColor: '#999',
  },
  courtText: {
    fontSize: 7,
    fontWeight: 'bold',
    color: '#333',
  },
  matchTeams: {
    flex: 1,
  },
  matchTeam: {
    flexDirection: 'row',
    justifyContent: 'space-between',
    paddingHorizontal: 3,
    paddingVertical: 1,
    height: MATCH_HEIGHT / 2,
    borderBottomWidth: 0.5,
    borderBottomColor: '#ddd',
  },
  matchTeamBottom: {
    borderBottomWidth: 0,
  },
  winner: {
    backgroundColor: '#e8f5e9',
  },
  teamName: {
    fontSize: 6,
    flex: 1,
  },
  score: {
    fontSize: 6,
    width: 14,
    textAlign: 'right',
    fontWeight: 'bold',
  },
  tbd: {
    color: '#999',
    fontStyle: 'italic',
  },
  bye: {
    color: '#888',
    fontStyle: 'italic',
  },
  lineContainer: {
    width: ROUND_GAP,
    position: 'relative',
  },
  hLine: {
    position: 'absolute',
    height: 0.5,
    backgroundColor: '#999',
    left: 0,
    width: LINE_LENGTH,
  },
  vLine: {
    position: 'absolute',
    width: 0.5,
    backgroundColor: '#999',
    left: LINE_LENGTH,
  },
  hLineToNext: {
    position: 'absolute',
    height: 0.5,
    backgroundColor: '#999',
    left: LINE_LENGTH,
    width: ROUND_GAP - LINE_LENGTH,
  },
});

interface BracketPDFProps {
  tournament: Tournament;
  /** The sheet this run is being printed on, chosen at the print button. */
  paperSize: PaperSize;
  teams: Team[];
  brackets: Bracket[];
  matches: BracketMatch[];
  translations: PDFTranslations;
}

export function BracketPDF({
  tournament,
  paperSize,
  teams,
  brackets,
  matches,
  translations: t,
}: BracketPDFProps) {
  const getTeam = (teamId: string | null | undefined) =>
    teamId ? teams.find((tm) => tm.id === teamId) : undefined;

  const formatDate = (dateString: string) => {
    return new Date(dateString).toLocaleDateString();
  };

  const getMatchesForBracket = (bracketId: string) => {
    return matches.filter((m) => m.bracketId === bracketId);
  };

  const getRoundName = (roundNumber: number, totalRounds: number) => {
    const roundsFromEnd = totalRounds - roundNumber + 1;
    switch (roundsFromEnd) {
      case 1:
        return t.final;
      case 2:
        return t.semiFinal;
      case 3:
        return t.quarterFinal;
      default:
        return `${t.round} ${roundNumber}`;
    }
  };

  /** A walkover is not played, so it has no court to print. */
  const courtLabel = (match: BracketMatch) => {
    if (match.isBye || match.courtNumber === null) return '';
    return `${t.courtAbbrev}${match.courtNumber}`;
  };

  const renderMatch = (match: BracketMatch, matchWidth: number) => (
    <View style={[styles.match, { width: matchWidth }]}>
      <View style={styles.court}>
        <Text style={styles.courtText}>{courtLabel(match)}</Text>
      </View>
      <View style={styles.matchTeams}>
        <View
          style={[
            styles.matchTeam,
            match.winnerId === match.team1Id ? styles.winner : {},
          ]}
        >
          <TeamLabelPDF
            team={getTeam(match.team1Id)}
            fontSize={6}
            compact
            fallback={t.tbd}
            style={styles.teamName}
            nameStyle={!match.team1Id ? styles.tbd : {}}
          />
          <Text style={styles.score}>
            {match.team1Score !== null ? match.team1Score : ''}
          </Text>
        </View>
        <View
          style={[
            styles.matchTeam,
            styles.matchTeamBottom,
            match.winnerId === match.team2Id ? styles.winner : {},
            match.isBye ? styles.bye : {},
          ]}
        >
          <TeamLabelPDF
            team={match.isBye ? undefined : getTeam(match.team2Id)}
            fontSize={6}
            compact
            fallback={match.isBye ? t.bye : t.tbd}
            style={styles.teamName}
            nameStyle={[!match.team2Id ? styles.tbd : {}, match.isBye ? styles.bye : {}]}
          />
          <Text style={styles.score}>
            {match.isBye ? '' : match.team2Score !== null ? match.team2Score : ''}
          </Text>
        </View>
      </View>
    </View>
  );

  // Everything below is laid out by hand, so it has to know how big the page
  // the operator picked actually is.
  const content = contentSize(paperSize, 'landscape', PAGE_PADDING);

  // A bracket needs at least two entrants to have a single match, and a
  // Document with no Page at all produces a zero-page PDF - a file that some
  // viewers refuse to open and that a printer has nothing to do with. Both
  // are unreachable from a freshly generated draw, but an older tournament can
  // still hold a degenerate bracket, and printing sends these bytes at a
  // printer unattended.
  const drawable = brackets.filter((b) => b.size >= 2 && Number.isInteger(Math.log2(b.size)));

  if (drawable.length === 0) {
    return (
      <Document>
        <Page {...pageProps(paperSize, 'landscape')} style={styles.page}>
          <View style={styles.header}>
            <PdfLogo tournament={tournament} box={LOGO_BOX_COMPACT} />
            <Text style={styles.title}>{tournament.name}</Text>
            <Text style={styles.subtitle}>{t.nothingToShow}</Text>
          </View>
        </Page>
      </Document>
    );
  }

  return (
    <Document>
      {drawable.map((bracket) => {
        const bracketMatches = getMatchesForBracket(bracket.id);
        const numRounds = Math.log2(bracket.size);
        const firstRoundMatchCount = bracket.size / 2;
        const matchWidth = matchWidthFor(numRounds, content.width);

        // Vertical spacing: whatever the chosen paper leaves once the header
        // and the round labels have taken their share.
        const availableHeight = content.height - VERTICAL_RESERVE;
        const matchSpacingRound1 = availableHeight / firstRoundMatchCount;

        return (
          <Page key={bracket.id} {...pageProps(paperSize, 'landscape')} style={styles.page}>
            <View style={styles.header}>
              <PdfLogo tournament={tournament} box={LOGO_BOX_COMPACT} />
              <Text style={styles.title}>{tournament.name}</Text>
              <Text style={styles.subtitle}>
                {/* With every team seeded into a bracket there is no losers'
                    draw to speak of - each bracket is a concours in its own
                    right - so the consolante keeps its name but loses the label. */}
                {bracket.isConsolante && !tournament.advanceAll ? t.consolante : t.concours}{' '}
                {bracket.name} | {formatDate(tournament.startDate)} | {t.courtLegend}
              </Text>
            </View>

            <View style={styles.bracketContainer} wrap={false}>
              {Array.from({ length: numRounds }, (_, roundIdx) => {
                const roundNumber = roundIdx + 1;
                const roundMatches = bracketMatches
                  .filter((m) => m.roundNumber === roundNumber)
                  .sort((a, b) => a.matchNumber - b.matchNumber);

                const spacingMultiplier = Math.pow(2, roundIdx);
                const matchSpacing = matchSpacingRound1 * spacingMultiplier;
                // Center the match vertically within its spacing slot
                const verticalPadding = (matchSpacing - MATCH_HEIGHT) / 2;

                return (
                  <View key={roundNumber} style={styles.roundColumn}>
                    <Text style={[styles.roundLabel, { width: matchWidth }]}>
                      {getRoundName(roundNumber, numRounds)}
                    </Text>
                    <View>
                      {roundMatches.map((match, idx) => {
                        const isLastRound = roundNumber === numRounds;
                        const showLines = !isLastRound;
                        const isTopOfPair = idx % 2 === 0;
                        const lineHeight = matchSpacing / 2;

                        return (
                          <View key={match.id} style={{ height: matchSpacing }}>
                            <View style={[styles.matchRow, { marginTop: verticalPadding }]}>
                              {renderMatch(match, matchWidth)}
                              {showLines && (
                                <View style={[styles.lineContainer, { height: MATCH_HEIGHT }]}>
                                  {/* Horizontal line from this match */}
                                  <View style={[styles.hLine, { top: MATCH_HEIGHT / 2 }]} />
                                  {/* Vertical line segment */}
                                  {isTopOfPair ? (
                                    <View style={[styles.vLine, { top: MATCH_HEIGHT / 2, height: lineHeight }]} />
                                  ) : (
                                    <View style={[styles.vLine, { top: MATCH_HEIGHT / 2 - lineHeight, height: lineHeight }]} />
                                  )}
                                  {/* Horizontal line to next match (only on bottom of pair) */}
                                  {!isTopOfPair && (
                                    <View style={[styles.hLineToNext, { top: MATCH_HEIGHT / 2 - lineHeight }]} />
                                  )}
                                </View>
                              )}
                            </View>
                          </View>
                        );
                      })}
                    </View>
                  </View>
                );
              })}
            </View>
          </Page>
        );
      })}
    </Document>
  );
}
