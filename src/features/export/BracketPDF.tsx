import { Document, Page, Text, View, StyleSheet } from '@react-pdf/renderer';
import type { Tournament, Team, Bracket, BracketMatch } from '../../types';
import { formatTeamName } from '../../lib/utils';

// Compact dimensions for fitting 16-team bracket on one page
const MATCH_WIDTH = 110;
const MATCH_HEIGHT = 24;
const ROUND_GAP = 25;
const LINE_LENGTH = 12; // Horizontal line from match to vertical

const styles = StyleSheet.create({
  page: {
    padding: 15,
    fontSize: 8,
    fontFamily: 'Helvetica',
  },
  header: {
    marginBottom: 8,
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
    width: MATCH_WIDTH,
    marginBottom: 4,
    height: 10,
  },
  matchRow: {
    flexDirection: 'row',
    alignItems: 'center',
  },
  match: {
    width: MATCH_WIDTH,
    borderWidth: 1,
    borderColor: '#999',
    backgroundColor: '#fff',
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
  winnerBox: {
    width: MATCH_WIDTH,
    padding: 4,
    backgroundColor: '#e3f2fd',
    borderWidth: 1,
    borderColor: '#1976d2',
  },
  winnerLabel: {
    fontSize: 6,
    color: '#666',
    marginBottom: 1,
  },
  winnerName: {
    fontSize: 8,
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
        const firstRoundMatchCount = bracket.size / 2;

        // Calculate vertical spacing - total height available for matches
        // A4 landscape: 842 x 595, with padding (15) we have about 812 x 565
        // Reserve ~35 for header + ~15 for round labels = ~50, leaves ~515 for bracket
        // Use 500 to ensure no overflow
        const availableHeight = 500;
        const matchSpacingRound1 = availableHeight / firstRoundMatchCount;

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
                    <Text style={styles.roundLabel}>
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
                              {renderMatch(match)}
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

              {/* Winner column */}
              <View style={styles.roundColumn}>
                <Text style={styles.roundLabel}>Winner</Text>
                <View>
                  <View style={{ height: matchSpacingRound1 * Math.pow(2, numRounds - 1) }}>
                    <View style={{ marginTop: (matchSpacingRound1 * Math.pow(2, numRounds - 1) - MATCH_HEIGHT) / 2 }}>
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
              </View>
            </View>
          </Page>
        );
      })}
    </Document>
  );
}
