import { Document, Page, Text, View, StyleSheet } from '@react-pdf/renderer';
import type { Tournament, Team, QualifyingRound, QualifyingGame } from '../../types';
import { formatTeamName } from '../../lib/utils';

// Card dimensions - 3 columns x 4 rows = 12 cards per page
// A4 is 595 x 842 points, with padding we have ~565 x 812 usable
const CARD_WIDTH = 175;
const CARD_HEIGHT = 190;
const CARD_MARGIN = 4;
const CARDS_PER_ROW = 3;
const CARDS_PER_COL = 4;
const CARDS_PER_PAGE = CARDS_PER_ROW * CARDS_PER_COL;

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
}

export function ScoreSheetPDF({ tournament, teams, rounds, games }: ScoreSheetPDFProps) {
  const getTeamName = (teamId: string | null | undefined) => {
    if (!teamId) return 'TBD';
    const team = teams.find((t) => t.id === teamId);
    return formatTeamName(team?.captain);
  };

  // Build list of all game cards (excluding BYE games)
  const gameCards: { round: QualifyingRound; game: QualifyingGame }[] = [];

  rounds.forEach((round) => {
    const roundGames = games
      .filter((g) => g.roundId === round.id && !g.isBye)
      .sort((a, b) => a.courtNumber - b.courtNumber);

    roundGames.forEach((game) => {
      gameCards.push({ round, game });
    });
  });

  // Split into pages
  const pages: { round: QualifyingRound; game: QualifyingGame }[][] = [];
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
            {pageCards.map(({ round, game }) => (
              <View key={game.id} style={styles.card} wrap={false}>
                <View style={styles.cardHeader}>
                  <Text style={styles.roundBadge}>Round {round.roundNumber}</Text>
                  <Text style={styles.courtBadge}>Court {game.courtNumber}</Text>
                </View>

                <View style={styles.teamSection}>
                  <View style={styles.teamRow}>
                    <Text style={styles.teamName}>{getTeamName(game.team1Id)}</Text>
                    <View style={styles.scoreBox}>
                      <Text style={styles.scoreText}>
                        {game.team1Score !== null ? game.team1Score : ''}
                      </Text>
                    </View>
                  </View>

                  <Text style={styles.vsText}>vs</Text>

                  <View style={styles.teamRow}>
                    <Text style={styles.teamName}>{getTeamName(game.team2Id)}</Text>
                    <View style={styles.scoreBox}>
                      <Text style={styles.scoreText}>
                        {game.team2Score !== null ? game.team2Score : ''}
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
