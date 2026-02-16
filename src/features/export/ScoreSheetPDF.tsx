import { Document, Page, Text, View, StyleSheet } from '@react-pdf/renderer';
import type { Tournament, Team, QualifyingRound, QualifyingGame } from '../../types';
import { formatTeamName } from '../../lib/utils';

export interface PDFTranslations {
  round: string;
  court: string;
  vs: string;
  champion: string;
  winner: string;
  tbd: string;
  bye: string;
  final: string;
  semiFinal: string;
  quarterFinal: string;
  rank: string;
  team: string;
  wins: string;
  losses: string;
  pointsFor: string;
  pointsAgainst: string;
  differential: string;
  concours: string;
  consolante: string;
  standings: string;
  standingsAsOf: string;
  topTeamsAdvance: string;
  legendWins: string;
  legendLosses: string;
  legendPointsFor: string;
  legendPointsAgainst: string;
  legendDifferential: string;
  legendBuchholz: string;
  legendFineBuchholz: string;
  tiebreaker: string;
}

// Card dimensions - 3 columns x 4 rows = 12 cards per page
// A4 is 595 x 842 points, with padding we have ~565 x 812 usable
const CARD_WIDTH = 175;
const CARD_HEIGHT = 190;
const CARD_MARGIN = 4;
const CARDS_PER_ROW = 3;
const CARDS_PER_COL = 4;
const CARDS_PER_PAGE = CARDS_PER_ROW * CARDS_PER_COL;

// Colors for each round (avoiding light colors for readability)
const ROUND_COLORS = [
  '#1976d2', // Blue
  '#c62828', // Red
  '#2e7d32', // Green
  '#6a1b9a', // Purple
  '#e65100', // Orange
  '#00838f', // Teal
  '#4527a0', // Deep Purple
  '#ad1457', // Pink
  '#1565c0', // Dark Blue
  '#558b2f', // Light Green
  '#d84315', // Deep Orange
  '#00695c', // Dark Teal
];

const styles = StyleSheet.create({
  page: {
    padding: 12,
    fontSize: 10,
    fontFamily: 'Helvetica',
  },
  cardGrid: {
    flexDirection: 'row',
    flexWrap: 'wrap',
    justifyContent: 'flex-start',
  },
  card: {
    width: CARD_WIDTH,
    height: CARD_HEIGHT,
    margin: CARD_MARGIN,
    borderWidth: 1,
    borderColor: '#000',
    borderStyle: 'dashed',
    padding: 8,
    flexDirection: 'column',
  },
  cardHeader: {
    flexDirection: 'row',
    justifyContent: 'space-between',
    borderBottomWidth: 1,
    borderBottomColor: '#ccc',
    paddingBottom: 5,
    marginBottom: 6,
  },
  roundBadge: {
    backgroundColor: '#1976d2',
    color: '#fff',
    paddingHorizontal: 6,
    paddingVertical: 2,
    borderRadius: 2,
    fontSize: 8,
    fontWeight: 'bold',
  },
  courtBadge: {
    backgroundColor: '#666',
    color: '#fff',
    paddingHorizontal: 6,
    paddingVertical: 2,
    borderRadius: 2,
    fontSize: 8,
    fontWeight: 'bold',
  },
  teamSection: {
    flex: 1,
    justifyContent: 'center',
  },
  teamRow: {
    flexDirection: 'row',
    alignItems: 'center',
    marginVertical: 4,
  },
  teamName: {
    flex: 1,
    fontSize: 10,
    fontWeight: 'bold',
  },
  scoreBox: {
    width: 36,
    height: 28,
    borderWidth: 2,
    borderColor: '#000',
    backgroundColor: '#fafafa',
    justifyContent: 'center',
    alignItems: 'center',
  },
  scoreText: {
    fontSize: 14,
    fontWeight: 'bold',
  },
  vsText: {
    textAlign: 'center',
    fontSize: 8,
    color: '#666',
    marginVertical: 2,
  },
  tournamentName: {
    fontSize: 7,
    color: '#666',
    textAlign: 'center',
    marginTop: 'auto',
    paddingTop: 4,
    borderTopWidth: 1,
    borderTopColor: '#eee',
  },
});

interface ScoreSheetPDFProps {
  tournament: Tournament;
  teams: Team[];
  rounds: QualifyingRound[];
  games: QualifyingGame[];
  translations: PDFTranslations;
}

export function ScoreSheetPDF({ tournament, teams, rounds, games, translations: t }: ScoreSheetPDFProps) {
  const getTeamName = (teamId: string | null | undefined) => {
    if (!teamId) return t.tbd;
    const team = teams.find((t) => t.id === teamId);
    return formatTeamName(team?.captain);
  };

  const getRoundColor = (roundNumber: number) => {
    return ROUND_COLORS[(roundNumber - 1) % ROUND_COLORS.length];
  };

  // Build list of all game cards (excluding BYE games)
  // Each game gets 2 cards (one for each team)
  // Copy 0: team1 on top, Copy 1: team2 on top (reversed)
  // Cards are sorted by first team name, then round number
  interface GameCard {
    round: QualifyingRound;
    game: QualifyingGame;
    copyIndex: number;
    firstTeamName: string; // The team shown on top of the card
  }

  const gameCards: GameCard[] = [];

  rounds.forEach((round) => {
    const roundGames = games.filter((g) => g.roundId === round.id && !g.isBye);

    roundGames.forEach((game) => {
      const team1Name = getTeamName(game.team1Id);
      const team2Name = getTeamName(game.team2Id);

      // Copy 0: team1 on top
      gameCards.push({ round, game, copyIndex: 0, firstTeamName: team1Name });
      // Copy 1: team2 on top (reversed order)
      gameCards.push({ round, game, copyIndex: 1, firstTeamName: team2Name });
    });
  });

  // Sort cards by first team name, then by round number
  gameCards.sort((a, b) => {
    const nameCompare = a.firstTeamName.localeCompare(b.firstTeamName);
    if (nameCompare !== 0) return nameCompare;
    return a.round.roundNumber - b.round.roundNumber;
  });

  // Split into pages
  const pages: GameCard[][] = [];
  for (let i = 0; i < gameCards.length; i += CARDS_PER_PAGE) {
    pages.push(gameCards.slice(i, i + CARDS_PER_PAGE));
  }

  // If no games, show empty page
  if (pages.length === 0) {
    pages.push([]);
  }

  return (
    <Document>
      {pages.map((pageCards, pageIndex) => (
        <Page key={pageIndex} size="A4" style={styles.page}>
          <View style={styles.cardGrid}>
            {pageCards.map(({ round, game, copyIndex }) => (
              <View key={`${game.id}-${copyIndex}`} style={styles.card} wrap={false}>
                <View style={styles.cardHeader}>
                  <Text style={[styles.roundBadge, { backgroundColor: getRoundColor(round.roundNumber) }]}>
                    {t.round} {round.roundNumber}
                  </Text>
                  <Text style={styles.courtBadge}>{t.court} {game.courtNumber}</Text>
                </View>

                <View style={styles.teamSection}>
                  {/* Copy 0: team1 on top, Copy 1: team2 on top */}
                  <View style={styles.teamRow}>
                    <Text style={styles.teamName}>
                      {copyIndex === 0 ? getTeamName(game.team1Id) : getTeamName(game.team2Id)}
                    </Text>
                    <View style={styles.scoreBox}>
                      <Text style={styles.scoreText}>
                        {copyIndex === 0
                          ? (game.team1Score !== null ? game.team1Score : '')
                          : (game.team2Score !== null ? game.team2Score : '')}
                      </Text>
                    </View>
                  </View>

                  <Text style={styles.vsText}>{t.vs}</Text>

                  <View style={styles.teamRow}>
                    <Text style={styles.teamName}>
                      {copyIndex === 0 ? getTeamName(game.team2Id) : getTeamName(game.team1Id)}
                    </Text>
                    <View style={styles.scoreBox}>
                      <Text style={styles.scoreText}>
                        {copyIndex === 0
                          ? (game.team2Score !== null ? game.team2Score : '')
                          : (game.team1Score !== null ? game.team1Score : '')}
                      </Text>
                    </View>
                  </View>
                </View>

                <Text style={styles.tournamentName}>{tournament.name}</Text>
              </View>
            ))}
          </View>
        </Page>
      ))}
    </Document>
  );
}
