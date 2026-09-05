import { Document, Page, Text, View, StyleSheet } from '@react-pdf/renderer';
import type { Tournament, Team, TeamStanding } from '../../types';
import { formatTeamLabel } from '../../lib/utils';
import type { PDFTranslations } from './pdfTranslations';

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
    width: '21%',
  },
  statCol: {
    width: '9%',
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
  translations: PDFTranslations;
}

export function StandingsPDF({ tournament, teams, standings, translations: t }: StandingsPDFProps) {
  const getTeamName = (teamId: string) => {
    const team = teams.find((t) => t.id === teamId);
    return formatTeamLabel(team);
  };

  // Mirror the on-screen column matrix in StandingsTable: only show the tiebreaker
  // columns the tournament's format actually ranks on.
  const pairingMethod = tournament.pairingMethod;
  const isPanache = pairingMethod === 'panache';
  const showBuchholz = pairingMethod === 'swiss';
  const showPointQuotient =
    pairingMethod === 'swissHotel' ||
    pairingMethod === 'roundRobin' ||
    pairingMethod === 'poolPlay';

  const formatDate = (dateString: string) => {
    return new Date(dateString).toLocaleDateString();
  };

  const sortedStandings = [...standings].sort((a, b) => a.rank - b.rank);

  // Top teams based on advancement settings
  // Panache has no bracket, so nothing "advances" — every row is on equal footing.
  const topTeamCount =
    tournament.advanceAll || tournament.pairingMethod === 'panache'
      ? standings.length
      : tournament.advanceCount || tournament.bracketSize;

  return (
    <Document>
      <Page size="A4" style={styles.page}>
        <View style={styles.header}>
          <Text style={styles.title}>{tournament.name}</Text>
          <Text style={styles.subtitle}>
            {t.standingsAsOf} {formatDate(new Date().toISOString())}
          </Text>
          {!tournament.advanceAll && !isPanache && (
            <Text style={styles.subtitle}>
              {t.topTeamsAdvance.replace('{{count}}', String(topTeamCount))}
            </Text>
          )}
        </View>

        <View style={styles.table}>
          <View style={styles.tableHeader}>
            <Text style={[styles.rankCol, styles.bold]}>{t.rank}</Text>
            <Text style={[styles.teamCol, styles.bold]}>{isPanache ? t.player : t.team}</Text>
            <Text style={[styles.statCol, styles.bold]}>{t.wins}</Text>
            <Text style={[styles.statCol, styles.bold]}>{t.losses}</Text>
            <Text style={[styles.statCol, styles.bold]}>{t.pointsFor}</Text>
            <Text style={[styles.statCol, styles.bold]}>{t.pointsAgainst}</Text>
            {showBuchholz && (
              <>
                <Text style={[styles.statCol, styles.bold]}>{t.buchholz}</Text>
                <Text style={[styles.statCol, styles.bold]}>{t.fineBuchholz}</Text>
              </>
            )}
            {showPointQuotient && (
              <Text style={[styles.statCol, styles.bold]}>{t.pointQuotient}</Text>
            )}
            <Text style={[styles.statCol, styles.bold]}>{t.differential}</Text>
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
                {showBuchholz && (
                  <>
                    <Text style={styles.statCol}>{standing.buchholzScore.toFixed(1)}</Text>
                    <Text style={styles.statCol}>{standing.fineBuchholzScore.toFixed(1)}</Text>
                  </>
                )}
                {showPointQuotient && (
                  <Text style={styles.statCol}>
                    {standing.pointQuotient > 100 ? '∞' : standing.pointQuotient.toFixed(2)}
                  </Text>
                )}
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
              </View>
            );
          })}
        </View>

        <View style={{ marginTop: 20 }}>
          <Text style={{ fontSize: 8, color: '#666' }}>
            {[
              t.legendWins,
              t.legendLosses,
              t.legendPointsFor,
              t.legendPointsAgainst,
              ...(showBuchholz ? [t.legendBuchholz, t.legendFineBuchholz] : []),
              ...(showPointQuotient ? [t.legendPointQuotient] : []),
              t.legendDifferential,
            ].join(', ')}
          </Text>
          <Text style={{ fontSize: 8, color: '#666', marginTop: 5 }}>
            {showBuchholz
              ? t.tiebreakerSwiss
              : isPanache
                ? t.tiebreakerPanache
                : t.tiebreakerPointQuotient}
          </Text>
        </View>
      </Page>
    </Document>
  );
}
