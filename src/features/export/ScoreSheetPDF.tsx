import { Document, Page, Text, View, StyleSheet } from '@react-pdf/renderer';
import type { Tournament, Team, QualifyingRound, QualifyingGame } from '../../types';

const styles = StyleSheet.create({
  page: {
    padding: 30,
    fontSize: 10,
    fontFamily: 'Helvetica',
  },
  header: {
    marginBottom: 20,
  },
  title: {
    fontSize: 18,
    fontWeight: 'bold',
    marginBottom: 5,
  },
  subtitle: {
    fontSize: 12,
    color: '#666',
  },
  roundHeader: {
    fontSize: 14,
    fontWeight: 'bold',
    marginTop: 20,
    marginBottom: 10,
    backgroundColor: '#f0f0f0',
    padding: 8,
  },
  table: {
    width: '100%',
    marginBottom: 20,
  },
  tableHeader: {
    flexDirection: 'row',
    backgroundColor: '#e0e0e0',
    borderBottomWidth: 1,
    borderBottomColor: '#000',
    paddingVertical: 5,
    paddingHorizontal: 3,
  },
  tableRow: {
    flexDirection: 'row',
    borderBottomWidth: 1,
    borderBottomColor: '#ccc',
    paddingVertical: 8,
    paddingHorizontal: 3,
  },
  courtCol: {
    width: '10%',
  },
  teamCol: {
    width: '35%',
  },
  scoreCol: {
    width: '10%',
    textAlign: 'center',
  },
  bold: {
    fontWeight: 'bold',
  },
  byeText: {
    color: '#888',
    fontStyle: 'italic',
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
    return team?.captain || 'Unknown';
  };

  const getGamesForRound = (roundId: string) => {
    return games.filter((g) => g.roundId === roundId).sort((a, b) => a.courtNumber - b.courtNumber);
  };

  const formatDate = (dateString: string) => {
    return new Date(dateString).toLocaleDateString();
  };

  return (
    <Document>
      <Page size="A4" style={styles.page}>
        <View style={styles.header}>
          <Text style={styles.title}>{tournament.name}</Text>
          <Text style={styles.subtitle}>
            {formatDate(tournament.startDate)} | {tournament.director} | Courts: {tournament.numberOfCourts}
          </Text>
        </View>

        {rounds.map((round) => {
          const roundGames = getGamesForRound(round.id);

          return (
            <View key={round.id}>
              <Text style={styles.roundHeader}>
                Round {round.roundNumber} {round.isComplete ? '(Complete)' : ''}
              </Text>

              <View style={styles.table}>
                <View style={styles.tableHeader}>
                  <Text style={[styles.courtCol, styles.bold]}>Court</Text>
                  <Text style={[styles.teamCol, styles.bold]}>Team 1</Text>
                  <Text style={[styles.scoreCol, styles.bold]}>Score</Text>
                  <Text style={[styles.scoreCol, styles.bold]}></Text>
                  <Text style={[styles.scoreCol, styles.bold]}>Score</Text>
                  <Text style={[styles.teamCol, styles.bold]}>Team 2</Text>
                </View>

                {roundGames.map((game) => (
                  <View key={game.id} style={styles.tableRow}>
                    <Text style={styles.courtCol}>{game.courtNumber}</Text>
                    <Text style={styles.teamCol}>{getTeamName(game.team1Id)}</Text>
                    <Text style={styles.scoreCol}>
                      {game.isBye ? '13' : game.team1Score !== null ? game.team1Score : '___'}
                    </Text>
                    <Text style={styles.scoreCol}>-</Text>
                    <Text style={styles.scoreCol}>
                      {game.isBye ? '0' : game.team2Score !== null ? game.team2Score : '___'}
                    </Text>
                    <Text style={[styles.teamCol, game.isBye ? styles.byeText : {}]}>
                      {game.isBye ? 'BYE' : getTeamName(game.team2Id)}
                    </Text>
                  </View>
                ))}
              </View>
            </View>
          );
        })}
      </Page>
    </Document>
  );
}
