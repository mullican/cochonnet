import { useEffect } from 'react';
import { Link } from 'react-router-dom';
import { useTranslation } from 'react-i18next';
import { useTournamentStore } from '../../stores/tournamentStore';
import { Button, Card, CardContent, CardHeader, CardTitle } from '../../components/ui';

export function TournamentList() {
  const { t } = useTranslation();
  const { tournaments, loading, fetchTournaments } = useTournamentStore();

  useEffect(() => {
    fetchTournaments();
  }, [fetchTournaments]);

  const formatDate = (dateString: string) => {
    return new Date(dateString).toLocaleDateString();
  };

  return (
    <div className="space-y-6">
      <div className="flex items-center justify-between">
        <h1 className="text-2xl font-bold text-gray-900">{t('tournaments.title')}</h1>
        <Link to="/tournaments/new">
          <Button>{t('tournaments.create')}</Button>
        </Link>
      </div>

      {loading ? (
        <div className="text-center py-8 text-gray-500">{t('common.loading')}</div>
      ) : tournaments.length === 0 ? (
        <Card>
          <CardContent className="py-12 text-center">
            <p className="text-gray-500">{t('tournaments.noTournaments')}</p>
            <Link to="/tournaments/new" className="mt-4 inline-block">
              <Button>{t('tournaments.create')}</Button>
            </Link>
          </CardContent>
        </Card>
      ) : (
        <div className="grid gap-4 sm:grid-cols-2 lg:grid-cols-3">
          {tournaments.map((tournament) => (
            <Link key={tournament.id} to={`/tournaments/${tournament.id}`}>
              <Card className="h-full transition-shadow hover:shadow-md">
                <CardHeader>
                  <CardTitle className="truncate">{tournament.name}</CardTitle>
                </CardHeader>
                <CardContent>
                  <div className="space-y-2 text-sm text-gray-600">
                    <div className="flex justify-between">
                      <span>{t('tournaments.type')}</span>
                      <span className="font-medium">
                        {t(`tournaments.typeOptions.${tournament.type}`)}
                      </span>
                    </div>
                    <div className="flex justify-between">
                      <span>{t('tournaments.format')}</span>
                      <span className="font-medium">
                        {t(`tournaments.formatOptions.${tournament.format}`)}
                      </span>
                    </div>
                    <div className="flex justify-between">
                      <span>{t('tournaments.startDate')}</span>
                      <span className="font-medium">{formatDate(tournament.startDate)}</span>
                    </div>
                    <div className="flex justify-between">
                      <span>{t('tournaments.numberOfCourts')}</span>
                      <span className="font-medium">{tournament.numberOfCourts}</span>
                    </div>
                  </div>
                </CardContent>
              </Card>
            </Link>
          ))}
        </div>
      )}
    </div>
  );
}
