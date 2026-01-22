import { Document, Page, Text, View, StyleSheet } from '@react-pdf/renderer';
import type { Tournament, Team, Bracket, BracketMatch } from '../../types';
import { formatTeamName } from '../../lib/utils';

// Compact dimensions for fitting 16-team bracket on one page
const MATCH_WIDTH = 120;
const MATCH_HEIGHT = 28;
const ROUND_GAP = 30;
const VERTICAL_GAP = 4;

const styles = StyleSheet.create({
  page: {
    padding: 20,
    fontSize: 8,
    fontFamily: 'Helvetica',
  },
  header: {
    marginBottom: 10,
  },
  title: {
    fontSize: 14,
    fontWeight: 'bold',
    marginBottom: 3,
  },
  subtitle: {
    fontSize: 10,
    color: '#666',
  },
  bracketContainer: {
    flexDirection: 'row',
    alignItems: 'flex-start',
  },
  roundColumn: {
    width: MATCH_WIDTH + ROUND_GAP,
    alignItems: 'flex-start',
  },
  roundLabel: {
    fontSize: 8,
    fontWeight: 'bold',
    color: '#666',
    textAlign: 'center',
    width: MATCH_WIDTH,
    marginBottom: 6,
  },
  matchWrapper: {
    flexDirection: 'row',
    alignItems: 'center',
  },
  match: {
    width: MATCH_WIDTH,
    height: MATCH_HEIGHT,
    borderWidth: 1,
    borderColor: '#999',
    backgroundColor: '#fff',
  },
  matchTeam: {
    flexDirection: 'row',
    justifyContent: 'space-between',
    paddingHorizontal: 4,
    paddingVertical: 2,
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
    fontSize: 7,
    flex: 1,
  },
  score: {
    fontSize: 7,
    width: 16,
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
  connector: {
    width: ROUND_GAP,
    flexDirection: 'column',
    justifyContent: 'center',
  },
  lineHorizontal: {
    height: 0.5,
    backgroundColor: '#999',
  },
  lineVertical: {
    width: 0.5,
    backgroundColor: '#999',
  },
  winnerColumn: {
    width: MATCH_WIDTH,
  },
  winnerBox: {
    width: MATCH_WIDTH,
    padding: 6,
    backgroundColor: '#e3f2fd',
    borderWidth: 1,
    borderColor: '#1976d2',
  },
  winnerLabel: {
    fontSize: 7,
    color: '#666',
    marginBottom: 2,
  },
  winnerName: {
    fontSize: 9,
    fontWeight: 'bold',
    color: '#1565c0',
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

  const renderMatch = (match: BracketMatch) => (
    <View style={styles.match}>
      <View
        style={[
          styles.matchTeam,
          match.winnerId === match.team1Id ? styles.winner : {},
        ]}
      >
        <Text style={[styles.teamName, !match.team1Id ? styles.tbd : {}]}>
          {getTeamName(match.team1Id)}
        </Text>
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
          {match.isBye ? '' : match.team2Score !== null ? match.team2Score : ''}
        </Text>
      </View>
    </View>
  );

  return (
    <Document>
      {brackets.map((bracket) => {
        const bracketMatches = getMatchesForBracket(bracket.id);
        const numRounds = Math.log2(bracket.size);

        // Calculate spacing for each round
        const baseMatchHeight = MATCH_HEIGHT + VERTICAL_GAP;

        const finalMatch = bracketMatches.find(
          (m) => m.roundNumber === numRounds && m.winnerId
        );

        return (
          <Page key={bracket.id} size="A4" orientation="landscape" style={styles.page}>
            <View style={styles.header}>
              <Text style={styles.title}>{tournament.name}</Text>
              <Text style={styles.subtitle}>
                {bracket.isConsolante ? 'Consolante' : 'Concours'} {bracket.name} | {formatDate(tournament.startDate)}
              </Text>
            </View>

            <View style={styles.bracketContainer}>
              {Array.from({ length: numRounds }, (_, roundIdx) => {
                const roundNumber = roundIdx + 1;
                const roundMatches = bracketMatches
                  .filter((m) => m.roundNumber === roundNumber)
                  .sort((a, b) => a.matchNumber - b.matchNumber);

                const spacingMultiplier = Math.pow(2, roundIdx);
                const matchSpacing = baseMatchHeight * spacingMultiplier;
                const topPadding = (matchSpacing - baseMatchHeight) / 2;

                return (
                  <View key={roundNumber} style={styles.roundColumn}>
                    <Text style={styles.roundLabel}>
                      {getRoundName(roundNumber, numRounds)}
                    </Text>
                    <View style={{ paddingTop: topPadding }}>
                      {roundMatches.map((match, idx) => (
                        <View
                          key={match.id}
                          style={{
                            height: matchSpacing,
                            flexDirection: 'row',
                            alignItems: 'center',
                          }}
                        >
                          {renderMatch(match)}
                          {roundNumber < numRounds && (
                            <View style={{ width: ROUND_GAP, height: matchSpacing }}>
                              <View style={{ flex: 1, flexDirection: 'row' }}>
                                <View style={{ flex: 1 }} />
                                <View style={{ width: ROUND_GAP / 2 }}>
                                  <View style={[styles.lineHorizontal, { marginTop: matchSpacing / 2 - 0.25 }]} />
                                  {idx % 2 === 0 ? (
                                    <View style={[styles.lineVertical, { height: matchSpacing / 2, marginLeft: ROUND_GAP / 2 - 0.5 }]} />
                                  ) : (
                                    <View style={[styles.lineVertical, { height: matchSpacing / 2, marginLeft: ROUND_GAP / 2 - 0.5, marginTop: -matchSpacing / 2 }]} />
                                  )}
                                </View>
                              </View>
                            </View>
                          )}
                        </View>
                      ))}
                    </View>
                  </View>
                );
              })}

              {/* Winner column */}
              <View style={styles.winnerColumn}>
                <Text style={styles.roundLabel}>Winner</Text>
                <View style={{ paddingTop: (baseMatchHeight * Math.pow(2, numRounds - 1) - baseMatchHeight) / 2 }}>
                  {finalMatch ? (
                    <View style={styles.winnerBox}>
                      <Text style={styles.winnerLabel}>Champion</Text>
                      <Text style={styles.winnerName}>{getTeamName(finalMatch.winnerId)}</Text>
                    </View>
                  ) : (
                    <View style={[styles.winnerBox, { backgroundColor: '#f5f5f5', borderColor: '#ccc' }]}>
                      <Text style={styles.winnerLabel}>Champion</Text>
                      <Text style={[styles.winnerName, { color: '#999' }]}>TBD</Text>
                    </View>
                  )}
                </View>
              </View>
            </View>
          </Page>
        );
      })}
    </Document>
  );
}
