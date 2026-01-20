import { Document, Page, Text, View, StyleSheet } from '@react-pdf/renderer';
import type { Tournament, Team, Bracket, BracketMatch } from '../../types';
import { formatTeamName } from '../../lib/utils';

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
  bracketTitle: {
    fontSize: 14,
    fontWeight: 'bold',
    marginTop: 20,
    marginBottom: 10,
    backgroundColor: '#f0f0f0',
    padding: 8,
  },
  roundContainer: {
    flexDirection: 'row',
    marginBottom: 20,
  },
  roundColumn: {
    marginRight: 20,
    minWidth: 150,
  },
  roundHeader: {
    fontSize: 11,
    fontWeight: 'bold',
    marginBottom: 10,
    textAlign: 'center',
    color: '#444',
  },
  match: {
    marginBottom: 15,
    borderWidth: 1,
    borderColor: '#ccc',
    borderRadius: 4,
  },
  matchTeam: {
    flexDirection: 'row',
    justifyContent: 'space-between',
    paddingVertical: 6,
    paddingHorizontal: 8,
    borderBottomWidth: 1,
    borderBottomColor: '#eee',
  },
  matchTeamLast: {
    borderBottomWidth: 0,
  },
  winner: {
    fontWeight: 'bold',
    backgroundColor: '#e6ffe6',
  },
  teamName: {
    flex: 1,
  },
  score: {
    width: 25,
    textAlign: 'right',
  },
  tbd: {
    color: '#999',
    fontStyle: 'italic',
  },
  bye: {
    color: '#888',
    fontStyle: 'italic',
  },
  winnerSection: {
    marginTop: 20,
    padding: 15,
    backgroundColor: '#f0f8ff',
    borderRadius: 4,
    borderWidth: 2,
    borderColor: '#4a90d9',
  },
  winnerTitle: {
    fontSize: 12,
    color: '#666',
    marginBottom: 5,
  },
  winnerName: {
    fontSize: 16,
    fontWeight: 'bold',
    color: '#2a5298',
  },
});

interface BracketPDFProps {
  tournament: Tournament;
  teams: Team[];
  brackets: Bracket[];
  matches: BracketMatch[];
}

export function BracketPDF({ tournament, teams, brackets, matches }: BracketPDFProps) {
  const getTeamName = (teamId: string | null | undefined) => {
    if (!teamId) return 'TBD';
    const team = teams.find((t) => t.id === teamId);
    return formatTeamName(team?.captain);
  };

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
        return 'Final';
      case 2:
        return 'Semi-Final';
      case 3:
        return 'Quarter-Final';
      default:
        return `Round ${roundNumber}`;
    }
  };

  return (
    <Document>
      {brackets.map((bracket) => {
        const bracketMatches = getMatchesForBracket(bracket.id);
        const numRounds = Math.log2(bracket.size);
        const finalMatch = bracketMatches.find(
          (m) => m.roundNumber === numRounds && m.winnerId
        );

        return (
          <Page key={bracket.id} size="A4" orientation="landscape" style={styles.page}>
            <View style={styles.header}>
              <Text style={styles.title}>{tournament.name}</Text>
              <Text style={styles.subtitle}>
                {bracket.isConsolante ? 'Consolante' : 'Main'} Bracket {bracket.name} |{' '}
                {formatDate(tournament.startDate)}
              </Text>
            </View>

            <View style={styles.roundContainer}>
              {Array.from({ length: numRounds }, (_, i) => i + 1).map((roundNumber) => {
                const roundMatches = bracketMatches
                  .filter((m) => m.roundNumber === roundNumber)
                  .sort((a, b) => a.matchNumber - b.matchNumber);

                return (
                  <View key={roundNumber} style={styles.roundColumn}>
                    <Text style={styles.roundHeader}>
                      {getRoundName(roundNumber, numRounds)}
                    </Text>

                    {roundMatches.map((match) => (
                      <View key={match.id} style={styles.match}>
                        <View
                          style={[
                            styles.matchTeam,
                            match.winnerId === match.team1Id ? styles.winner : {},
                          ]}
                        >
                          <Text
                            style={[
                              styles.teamName,
                              !match.team1Id ? styles.tbd : {},
                            ]}
                          >
                            {getTeamName(match.team1Id)}
                          </Text>
                          <Text style={styles.score}>
                            {match.team1Score !== null ? match.team1Score : '-'}
                          </Text>
                        </View>
                        <View
                          style={[
                            styles.matchTeam,
                            styles.matchTeamLast,
                            match.winnerId === match.team2Id ? styles.winner : {},
                            match.isBye ? styles.bye : {},
                          ]}
                        >
                          <Text
                            style={[
                              styles.teamName,
                              !match.team2Id ? styles.tbd : {},
                              match.isBye ? styles.bye : {},
                            ]}
                          >
                            {match.isBye ? 'BYE' : getTeamName(match.team2Id)}
                          </Text>
                          <Text style={styles.score}>
                            {match.isBye ? '-' : match.team2Score !== null ? match.team2Score : '-'}
                          </Text>
                        </View>
                      </View>
                    ))}
                  </View>
                );
              })}

              {finalMatch && (
                <View style={styles.roundColumn}>
                  <Text style={styles.roundHeader}>Winner</Text>
                  <View style={styles.winnerSection}>
                    <Text style={styles.winnerTitle}>Champion</Text>
                    <Text style={styles.winnerName}>
                      {getTeamName(finalMatch.winnerId)}
                    </Text>
                  </View>
                </View>
              )}
            </View>
          </Page>
        );
      })}
    </Document>
  );
}
