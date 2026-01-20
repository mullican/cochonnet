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
} from '../../components/ui';
import { formatTeamName } from '../../lib/utils';

interface StandingsTableProps {
  tournamentId: string;
}

export function StandingsTable({ tournamentId }: StandingsTableProps) {
  const { t } = useTranslation();
  const { standings, teams, loading, fetchStandings } = useTournamentStore();

  useEffect(() => {
    fetchStandings(tournamentId);
  }, [tournamentId, fetchStandings]);

  const getTeamName = (teamId: string) => {
    const team = teams.find((t) => t.id === teamId);
    return formatTeamName(team?.captain);
  };

  if (loading) {
    return <div className="text-center py-4 text-gray-500">{t('common.loading')}</div>;
  }

  if (standings.length === 0) {
    return (
      <Card>
        <div className="py-8 text-center text-gray-500">
          No standings yet. Complete a qualifying round to see standings.
        </div>
      </Card>
    );
  }

  // Sort standings by rank
  const sortedStandings = [...standings].sort((a, b) => a.rank - b.rank);

  return (
    <Card>
      <Table>
        <TableHeader>
          <TableRow>
            <TableHead className="w-16">{t('pairing.rank')}</TableHead>
            <TableHead>{t('teams.captain')}</TableHead>
            <TableHead className="text-center">{t('pairing.wins')}</TableHead>
            <TableHead className="text-center">{t('pairing.losses')}</TableHead>
            <TableHead className="text-center">{t('pairing.pointsFor')}</TableHead>
            <TableHead className="text-center">{t('pairing.pointsAgainst')}</TableHead>
            <TableHead className="text-center">{t('pairing.differential')}</TableHead>
            <TableHead className="text-center">{t('pairing.buchholz')}</TableHead>
          </TableRow>
        </TableHeader>
        <TableBody>
          {sortedStandings.map((standing) => (
            <TableRow key={standing.id}>
              <TableCell className="font-medium">{standing.rank}</TableCell>
              <TableCell className="font-medium">{getTeamName(standing.teamId)}</TableCell>
              <TableCell className="text-center">{standing.wins}</TableCell>
              <TableCell className="text-center">{standing.losses}</TableCell>
              <TableCell className="text-center">{standing.pointsFor}</TableCell>
              <TableCell className="text-center">{standing.pointsAgainst}</TableCell>
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
              <TableCell className="text-center">{standing.buchholzScore.toFixed(1)}</TableCell>
            </TableRow>
          ))}
        </TableBody>
      </Table>
    </Card>
  );
}
