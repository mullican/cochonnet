import { Document, Page, Text, View, StyleSheet, Svg, Line } from '@react-pdf/renderer';
import type { Tournament, Team, Bracket, BracketMatch } from '../../types';
import { formatTeamName } from '../../lib/utils';

// Compact dimensions for fitting 16-team bracket on one page
const MATCH_WIDTH = 120;
const MATCH_HEIGHT = 28;
const ROUND_GAP = 35;
const VERTICAL_GAP = 8;

const styles = StyleSheet.create({
  page: {
    padding: 20,
    fontSize: 8,
    fontFamily: 'Helvetica',
  },
  header: {
    marginBottom: 15,
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
    position: 'relative',
    width: '100%',
    height: '100%',
  },
  roundLabel: {
    position: 'absolute',
    fontSize: 8,
    fontWeight: 'bold',
    color: '#666',
    textAlign: 'center',
    width: MATCH_WIDTH,
  },
  match: {
    position: 'absolute',
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
  winnerBox: {
    position: 'absolute',
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

  const calculateBracketLayout = (bracketSize: number, bracketMatches: BracketMatch[]) => {
    const numRounds = Math.log2(bracketSize);
    const firstRoundMatches = bracketSize / 2;

    // Calculate total height needed
    const firstRoundHeight = firstRoundMatches * (MATCH_HEIGHT + VERTICAL_GAP) - VERTICAL_GAP;

    // Starting positions
    const startX = 20;
    const startY = 35; // Leave room for round labels

    const positions: { [matchId: string]: { x: number; y: number } } = {};
    const lines: { x1: number; y1: number; x2: number; y2: number }[] = [];

    // Position matches by round
    for (let round = 1; round <= numRounds; round++) {
      const roundMatches = bracketMatches
        .filter((m) => m.roundNumber === round)
        .sort((a, b) => a.matchNumber - b.matchNumber);

      const matchesInRound = bracketSize / Math.pow(2, round);
      const spacing = firstRoundHeight / matchesInRound;
      const xPos = startX + (round - 1) * (MATCH_WIDTH + ROUND_GAP);

      roundMatches.forEach((match, idx) => {
        const yPos = startY + spacing * idx + (spacing - MATCH_HEIGHT) / 2;
        positions[match.id] = { x: xPos, y: yPos };

        // Draw connecting lines to next round
        if (match.nextMatchId && positions[match.nextMatchId] === undefined) {
          // Line will be drawn when next match is positioned
        }
      });
    }

    // Generate connecting lines
    for (let round = 1; round < numRounds; round++) {
      const roundMatches = bracketMatches
        .filter((m) => m.roundNumber === round)
        .sort((a, b) => a.matchNumber - b.matchNumber);

      roundMatches.forEach((match) => {
        if (match.nextMatchId) {
          const currentPos = positions[match.id];
          const nextPos = positions[match.nextMatchId];

          if (currentPos && nextPos) {
            const fromX = currentPos.x + MATCH_WIDTH;
            const fromY = currentPos.y + MATCH_HEIGHT / 2;
            const toX = nextPos.x;
            const toY = nextPos.y + MATCH_HEIGHT / 2;

            // Horizontal line from match
            const midX = fromX + (ROUND_GAP / 2);
            lines.push({ x1: fromX, y1: fromY, x2: midX, y2: fromY });
            // Vertical line
            lines.push({ x1: midX, y1: fromY, x2: midX, y2: toY });
            // Horizontal line to next match
            lines.push({ x1: midX, y1: toY, x2: toX, y2: toY });
          }
        }
      });
    }

    return { positions, lines, numRounds, firstRoundHeight };
  };

  return (
    <Document>
      {brackets.map((bracket) => {
        const bracketMatches = getMatchesForBracket(bracket.id);
        const { positions, lines, numRounds } = calculateBracketLayout(bracket.size, bracketMatches);

        const finalMatch = bracketMatches.find(
          (m) => m.roundNumber === numRounds && m.winnerId
        );

        // Calculate winner box position
        const finalMatchPos = bracketMatches.find(m => m.roundNumber === numRounds);
        const winnerX = finalMatchPos ? positions[finalMatchPos.id]?.x + MATCH_WIDTH + ROUND_GAP : 0;
        const winnerY = finalMatchPos ? positions[finalMatchPos.id]?.y : 0;

        return (
          <Page key={bracket.id} size="A4" orientation="landscape" style={styles.page}>
            <View style={styles.header}>
              <Text style={styles.title}>{tournament.name}</Text>
              <Text style={styles.subtitle}>
                {bracket.isConsolante ? 'Consolante' : 'Concours'} {bracket.name} | {formatDate(tournament.startDate)}
              </Text>
            </View>

            <View style={styles.bracketContainer}>
              {/* Round labels */}
              {Array.from({ length: numRounds }, (_, i) => i + 1).map((roundNumber) => (
                <Text
                  key={`label-${roundNumber}`}
                  style={[
                    styles.roundLabel,
                    { left: 20 + (roundNumber - 1) * (MATCH_WIDTH + ROUND_GAP), top: 15 },
                  ]}
                >
                  {getRoundName(roundNumber, numRounds)}
                </Text>
              ))}

              {/* Winner label */}
              {finalMatch && (
                <Text
                  style={[
                    styles.roundLabel,
                    { left: winnerX, top: 15 },
                  ]}
                >
                  Winner
                </Text>
              )}

              {/* Connecting lines */}
              <Svg style={{ position: 'absolute', top: 0, left: 0, width: '100%', height: '100%' }}>
                {lines.map((line, idx) => (
                  <Line
                    key={idx}
                    x1={line.x1}
                    y1={line.y1}
                    x2={line.x2}
                    y2={line.y2}
                    stroke="#999"
                    strokeWidth={0.5}
                  />
                ))}
              </Svg>

              {/* Match boxes */}
              {bracketMatches.map((match) => {
                const pos = positions[match.id];
                if (!pos) return null;

                return (
                  <View key={match.id} style={[styles.match, { left: pos.x, top: pos.y }]}>
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
              })}

              {/* Winner box */}
              {finalMatch && (
                <View style={[styles.winnerBox, { left: winnerX, top: winnerY }]}>
                  <Text style={styles.winnerLabel}>Champion</Text>
                  <Text style={styles.winnerName}>{getTeamName(finalMatch.winnerId)}</Text>
                </View>
              )}
            </View>
          </Page>
        );
      })}
    </Document>
  );
}
