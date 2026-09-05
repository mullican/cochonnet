import { Document, Page, Text, View, StyleSheet } from '@react-pdf/renderer';
import type { Tournament, Team, QualifyingRound, GameWithTeams } from '../../types';
import { formatTeamLabel, formatPanacheSideLabel } from '../../lib/utils';
import type { PDFTranslations } from './pdfTranslations';

const styles = StyleSheet.create({
  page: {
    padding: 30,
    fontFamily: 'Helvetica',
  },
  header: {
    marginBottom: 16,
    borderBottomWidth: 2,
    borderBottomColor: '#000',
    paddingBottom: 8,
  },
  tournamentName: {
    fontSize: 14,
    color: '#444',
  },
  subtitle: {
    fontSize: 10,
    color: '#888',
    textTransform: 'uppercase',
    marginTop: 2,
  },
  roundTitle: {
    fontSize: 24,
    fontWeight: 'bold',
    marginTop: 2,
  },
  table: {
    width: '100%',
  },
  tableHeader: {
    flexDirection: 'row',
    backgroundColor: '#e0e0e0',
    borderBottomWidth: 1,
    borderBottomColor: '#000',
    paddingVertical: 5,
    paddingHorizontal: 6,
  },
  headerText: {
    fontSize: 10,
    fontWeight: 'bold',
    textTransform: 'uppercase',
    color: '#333',
  },
  row: {
    flexDirection: 'row',
    borderBottomWidth: 1,
    borderBottomColor: '#ccc',
    paddingVertical: 9,
    paddingHorizontal: 6,
    alignItems: 'center',
  },
  courtCol: {
    width: '15%',
    fontSize: 14,
    fontWeight: 'bold',
  },
  teamCol: {
    flex: 1,
    fontSize: 13,
  },
  vsCol: {
    width: '10%',
    fontSize: 10,
    color: '#666',
    textAlign: 'center',
  },
  byeText: {
    color: '#999',
    fontStyle: 'italic',
  },
  sitoutText: {
    marginTop: 12,
    fontSize: 10,
    color: '#666',
  },
  emptyText: {
    fontSize: 12,
    color: '#666',
    marginTop: 20,
  },
});

interface CourtAssignmentsPDFProps {
  tournament: Tournament;
  teams: Team[];
  rounds: QualifyingRound[];
  games: GameWithTeams[];
  /** Panache only: players resting each round, keyed by round id. */
  sitouts?: Record<string, Team[]>;
  translations: PDFTranslations;
}

export function CourtAssignmentsPDF({
  tournament,
  teams,
  rounds,
  games,
  sitouts,
  translations: t,
}: CourtAssignmentsPDFProps) {
  const getTeamName = (teamId: string | null | undefined) => {
    if (!teamId) return t.tbd;
    const team = teams.find((team) => team.id === teamId);
    return formatTeamLabel(team);
  };

  /**
   * A panache game names a drawn side; every other format names a registered team.
   */
  const getSideName = (game: GameWithTeams, which: 1 | 2) => {
    const side = which === 1 ? game.side1 : game.side2;
    if (side) return formatPanacheSideLabel(side);
    return getTeamName(which === 1 ? game.team1Id : game.team2Id);
  };

  const sortedRounds = [...rounds].sort((a, b) => a.roundNumber - b.roundNumber);

  return (
    <Document>
      {sortedRounds.map((round) => {
        const roundGames = games
          .filter((g) => g.roundId === round.id)
          .sort((a, b) => a.courtNumber - b.courtNumber);
        const roundSitouts = sitouts?.[round.id] ?? [];

        return (
          <Page key={round.id} size="A4" style={styles.page} wrap>
            <View style={styles.header} fixed>
              <Text style={styles.tournamentName}>{tournament.name}</Text>
              <Text style={styles.subtitle}>{t.courtAssignments}</Text>
              <Text style={styles.roundTitle}>
                {round.isFinal ? t.final : `${t.round} ${round.roundNumber}`}
              </Text>
            </View>

            {roundGames.length === 0 ? (
              <Text style={styles.emptyText}>{t.tbd}</Text>
            ) : (
              <View style={styles.table}>
                <View style={styles.tableHeader} fixed>
                  <Text style={[styles.courtCol, styles.headerText]}>{t.court}</Text>
                  <Text style={[styles.teamCol, styles.headerText]}>{t.team}</Text>
                  <Text style={styles.vsCol} />
                  <Text style={[styles.teamCol, styles.headerText]} />
                </View>
                {roundGames.map((game) => (
                  <View key={game.id} style={styles.row} wrap={false}>
                    <Text style={styles.courtCol}>{game.courtNumber}</Text>
                    <Text style={styles.teamCol}>{getSideName(game, 1)}</Text>
                    <Text style={styles.vsCol}>{game.isBye ? '' : t.vs}</Text>
                    <Text style={[styles.teamCol, game.isBye ? styles.byeText : {}]}>
                      {game.isBye ? t.bye : getSideName(game, 2)}
                    </Text>
                  </View>
                ))}
              </View>
            )}

            {roundSitouts.length > 0 && (
              <Text style={styles.sitoutText}>
                {t.sittingOut}: {roundSitouts.map((p) => formatTeamLabel(p)).join(', ')}
              </Text>
            )}
          </Page>
        );
      })}
    </Document>
  );
}
