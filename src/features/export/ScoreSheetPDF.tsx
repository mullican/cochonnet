import { Document, Page, Text, View, StyleSheet } from '@react-pdf/renderer';
import type { Tournament, Team, QualifyingRound, QualifyingGame } from '../../types';
import { formatTeamName } from '../../lib/utils';

// Card dimensions - 3 columns x 4 rows = 12 cards per page
const CARD_WIDTH = 180;
const CARD_HEIGHT = 180;
const CARD_MARGIN = 8;
const CARDS_PER_ROW = 3;
const CARDS_PER_COL = 4;
const CARDS_PER_PAGE = CARDS_PER_ROW * CARDS_PER_COL;

const styles = StyleSheet.create({
  page: {
    padding: 15,
    fontSize: 10,
    fontFamily: 'Helvetica',
  },
  cardGrid: {
    flexDirection: 'row',
    flexWrap: 'wrap',
  },
  card: {
    width: CARD_WIDTH,
    height: CARD_HEIGHT,
    margin: CARD_MARGIN,
    borderWidth: 1,
    borderColor: '#000',
    borderStyle: 'dashed',
    padding: 10,
    flexDirection: 'column',
  },
  cardHeader: {
    flexDirection: 'row',
    justifyContent: 'space-between',
    borderBottomWidth: 1,
    borderBottomColor: '#ccc',
    paddingBottom: 6,
    marginBottom: 8,
  },
  roundBadge: {
    backgroundColor: '#1976d2',
    color: '#fff',
    paddingHorizontal: 8,
    paddingVertical: 3,
    borderRadius: 3,
    fontSize: 9,
    fontWeight: 'bold',
  },
  courtBadge: {
    backgroundColor: '#666',
    color: '#fff',
    paddingHorizontal: 8,
    paddingVertical: 3,
    borderRadius: 3,
    fontSize: 9,
    fontWeight: 'bold',
  },
  teamSection: {
    flex: 1,
    justifyContent: 'center',
  },
  teamRow: {
    flexDirection: 'row',
    alignItems: 'center',
    marginVertical: 6,
  },
  teamName: {
    flex: 1,
    fontSize: 11,
    fontWeight: 'bold',
  },
  scoreBox: {
    width: 40,
    height: 32,
    borderWidth: 2,
    borderColor: '#000',
    backgroundColor: '#fafafa',
    justifyContent: 'center',
    alignItems: 'center',
  },
  scoreText: {
    fontSize: 16,
    fontWeight: 'bold',
  },
  vsText: {
    textAlign: 'center',
    fontSize: 9,
    color: '#666',
    marginVertical: 4,
  },
  byeCard: {
    backgroundColor: '#f5f5f5',
  },
  byeText: {
    color: '#888',
    fontStyle: 'italic',
  },
  tournamentName: {
    fontSize: 8,
    color: '#666',
    textAlign: 'center',
    marginTop: 'auto',
    paddingTop: 6,
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
              <View key={game.id} style={styles.card}>
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
