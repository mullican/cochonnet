import { Document, Page, Text, View, StyleSheet } from '@react-pdf/renderer';
import type { Tournament, Team, TeamStanding } from '../../types';

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
  table: {
    width: '100%',
    marginTop: 20,
  },
  tableHeader: {
    flexDirection: 'row',
    backgroundColor: '#e0e0e0',
    borderBottomWidth: 1,
    borderBottomColor: '#000',
    paddingVertical: 8,
    paddingHorizontal: 3,
  },
  tableRow: {
    flexDirection: 'row',
    borderBottomWidth: 1,
    borderBottomColor: '#ccc',
    paddingVertical: 8,
    paddingHorizontal: 3,
  },
  topRow: {
    backgroundColor: '#f0f8ff',
  },
  rankCol: {
    width: '8%',
    textAlign: 'center',
  },
  teamCol: {
    width: '30%',
  },
  statCol: {
    width: '10%',
    textAlign: 'center',
  },
  bold: {
    fontWeight: 'bold',
  },
  positive: {
    color: '#22863a',
  },
  negative: {
    color: '#cb2431',
  },
});

interface StandingsPDFProps {
  tournament: Tournament;
  teams: Team[];
  standings: TeamStanding[];
}

export function StandingsPDF({ tournament, teams, standings }: StandingsPDFProps) {
  const getTeamName = (teamId: string) => {
    const team = teams.find((t) => t.id === teamId);
    return team?.captain || 'Unknown';
  };

  const formatDate = (dateString: string) => {
    return new Date(dateString).toLocaleDateString();
  };

  const sortedStandings = [...standings].sort((a, b) => a.rank - b.rank);

  // Top teams based on advancement settings
  const topTeamCount = tournament.advanceAll
    ? standings.length
    : tournament.advanceCount || tournament.bracketSize;

  return (
    <Document>
      <Page size="A4" style={styles.page}>
        <View style={styles.header}>
          <Text style={styles.title}>{tournament.name}</Text>
          <Text style={styles.subtitle}>
            Standings as of {formatDate(new Date().toISOString())}
          </Text>
          {!tournament.advanceAll && (
            <Text style={styles.subtitle}>
              Top {topTeamCount} teams advance to elimination rounds
            </Text>
          )}
        </View>

        <View style={styles.table}>
          <View style={styles.tableHeader}>
            <Text style={[styles.rankCol, styles.bold]}>Rank</Text>
            <Text style={[styles.teamCol, styles.bold]}>Team</Text>
            <Text style={[styles.statCol, styles.bold]}>W</Text>
            <Text style={[styles.statCol, styles.bold]}>L</Text>
            <Text style={[styles.statCol, styles.bold]}>PF</Text>
            <Text style={[styles.statCol, styles.bold]}>PA</Text>
            <Text style={[styles.statCol, styles.bold]}>+/-</Text>
            <Text style={[styles.statCol, styles.bold]}>Buch</Text>
          </View>

          {sortedStandings.map((standing, index) => {
            const isTopTeam = index < topTeamCount;

            return (
              <View
                key={standing.id}
                style={[styles.tableRow, isTopTeam ? styles.topRow : {}]}
              >
                <Text style={[styles.rankCol, styles.bold]}>{standing.rank}</Text>
                <Text style={styles.teamCol}>{getTeamName(standing.teamId)}</Text>
                <Text style={styles.statCol}>{standing.wins}</Text>
                <Text style={styles.statCol}>{standing.losses}</Text>
                <Text style={styles.statCol}>{standing.pointsFor}</Text>
                <Text style={styles.statCol}>{standing.pointsAgainst}</Text>
                <Text
                  style={[
                    styles.statCol,
                    standing.differential > 0
                      ? styles.positive
                      : standing.differential < 0
                      ? styles.negative
                      : {},
                  ]}
                >
                  {standing.differential > 0 ? '+' : ''}
                  {standing.differential}
                </Text>
                <Text style={styles.statCol}>{standing.buchholzScore.toFixed(1)}</Text>
              </View>
            );
          })}
        </View>

        <View style={{ marginTop: 20 }}>
          <Text style={{ fontSize: 8, color: '#666' }}>
            W = Wins, L = Losses, PF = Points For, PA = Points Against, +/- = Point Differential, Buch = Buchholz Score
          </Text>
          <Text style={{ fontSize: 8, color: '#666', marginTop: 5 }}>
            Tie-breaker order: Wins → Point Differential → Buchholz Score → Points For
          </Text>
        </View>
      </Page>
    </Document>
  );
}
