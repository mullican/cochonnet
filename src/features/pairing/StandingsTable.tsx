import { useEffect } from 'react';
import { useTranslation } from 'react-i18next';
import { useTournamentStore } from '../../stores/tournamentStore';
import {
  Card,
  Table,
  TableHeader,
  TableBody,
  TableRow,
  TableHead,
  TableCell,
  TeamLabel,
} from '../../components/ui';

interface StandingsTableProps {
  tournamentId: string;
}

export function StandingsTable({ tournamentId }: StandingsTableProps) {
  const { t } = useTranslation();
  const { standings, teams, loading, fetchStandings, currentTournament } = useTournamentStore();
  const pairingMethod = currentTournament?.pairingMethod || 'swiss';

  // Determine which columns to show based on pairing method
  // Panache ranks on wins then differential, so neither tiebreaker column applies.
  const isPanache = pairingMethod === 'panache';
  const showBuchholz = pairingMethod === 'swiss';
  const showPointQuotient = pairingMethod === 'swissHotel' || pairingMethod === 'roundRobin' || pairingMethod === 'poolPlay';
  const showStatus = pairingMethod === 'poolPlay';

  useEffect(() => {
    fetchStandings(tournamentId);
  }, [tournamentId, fetchStandings]);

  const getTeam = (teamId: string) => teams.find((team) => team.id === teamId);

  if (loading) {
    return <div className="text-center py-4 text-gray-500">{t('common.loading')}</div>;
  }

  if (standings.length === 0) {
    return (
      <Card>
        <div className="py-8 text-center text-gray-500">
          {t('pairing.noStandings')}
        </div>
      </Card>
    );
  }

  // Sort standings by rank
  const sortedStandings = [...standings].sort((a, b) => a.rank - b.rank);

  return (
    <Card className="overflow-hidden">
      <Table>
        <TableHeader>
          <TableRow>
            <TableHead className="w-16">{t('pairing.rank')}</TableHead>
            <TableHead>{isPanache ? t('teams.player') : t('teams.captain')}</TableHead>
            <TableHead className="text-center">{t('pairing.wins')}</TableHead>
            <TableHead className="text-center">{t('pairing.losses')}</TableHead>
            <TableHead className="text-center">{t('pairing.pointsFor')}</TableHead>
            <TableHead className="text-center">{t('pairing.pointsAgainst')}</TableHead>
            {showBuchholz && (
              <>
                <TableHead className="text-center">{t('pairing.buchholz')}</TableHead>
                <TableHead className="text-center">{t('pairing.fineBuchholz')}</TableHead>
              </>
            )}
            {showPointQuotient && (
              <TableHead className="text-center">{t('pairing.pointQuotient')}</TableHead>
            )}
            <TableHead className="text-center">{t('pairing.differential')}</TableHead>
            {showStatus && (
              <TableHead className="text-center">{t('common.status')}</TableHead>
            )}
          </TableRow>
        </TableHeader>
        <TableBody>
          {sortedStandings.map((standing) => (
            <TableRow
              key={standing.id}
              className={standing.isEliminated ? 'opacity-50 bg-gray-50' : ''}
            >
              <TableCell className="font-medium">{standing.rank}</TableCell>
              <TableCell className="font-medium">
                <TeamLabel team={getTeam(standing.teamId)} />
                {isPanache && getTeam(standing.teamId)?.isChampion && (
                  <span className="ml-1 text-amber-600" title={t('teams.champion')}>
                    ★
                  </span>
                )}
              </TableCell>
              <TableCell className="text-center">{standing.wins}</TableCell>
              <TableCell className="text-center">{standing.losses}</TableCell>
              <TableCell className="text-center">{standing.pointsFor}</TableCell>
              <TableCell className="text-center">{standing.pointsAgainst}</TableCell>
              {showBuchholz && (
                <>
                  <TableCell className="text-center">{standing.buchholzScore.toFixed(1)}</TableCell>
                  <TableCell className="text-center">{standing.fineBuchholzScore.toFixed(1)}</TableCell>
                </>
              )}
              {showPointQuotient && (
                <TableCell className="text-center">
                  {standing.pointQuotient === Infinity || standing.pointQuotient > 100
                    ? '∞'
                    : standing.pointQuotient.toFixed(2)}
                </TableCell>
              )}
              <TableCell className="text-center">
                <span
                  className={
                    standing.differential > 0
                      ? 'text-green-600'
                      : standing.differential < 0
                      ? 'text-red-600'
                      : ''
                  }
                >
                  {standing.differential > 0 ? '+' : ''}
                  {standing.differential}
                </span>
              </TableCell>
              {showStatus && (
                <TableCell className="text-center">
                  {standing.isEliminated ? (
                    <span className="text-red-600">{t('pairing.eliminated')}</span>
                  ) : standing.wins >= 2 ? (
                    <span className="text-green-600">{t('pairing.qualified')}</span>
                  ) : null}
                </TableCell>
              )}
            </TableRow>
          ))}
        </TableBody>
      </Table>
    </Card>
  );
}
