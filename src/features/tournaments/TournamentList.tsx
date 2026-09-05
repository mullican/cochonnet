import { useEffect, useState } from 'react';
import { Link, useNavigate } from 'react-router-dom';
import { useTranslation } from 'react-i18next';
import { open } from '@tauri-apps/plugin-dialog';
import { readTextFile } from '@tauri-apps/plugin-fs';
import { invoke } from '@tauri-apps/api/core';
import { useTournamentStore } from '../../stores/tournamentStore';
import { Button, Card, CardContent, CardHeader, CardTitle } from '../../components/ui';

export function TournamentList() {
  const { t } = useTranslation();
  const navigate = useNavigate();
  const { tournaments, loading, fetchTournaments } = useTournamentStore();
  const [importing, setImporting] = useState(false);
  const [importError, setImportError] = useState<string | null>(null);

  useEffect(() => {
    fetchTournaments();
  }, [fetchTournaments]);

  /**
   * Restores a backup as a new tournament. The picker is available on every
   * platform - unlike saving a file, iOS has a perfectly good document picker
   * for opening one - which is what makes moving a tournament onto an iPad work.
   */
  const handleImport = async () => {
    setImportError(null);
    try {
      const path = await open({
        multiple: false,
        directory: false,
        filters: [{ name: 'Tournament backup', extensions: ['json'] }],
      });
      if (!path || typeof path !== 'string') return;

      setImporting(true);
      const json = await readTextFile(path);
      const tournamentId = await invoke<string>('import_tournament_backup', { json });
      await fetchTournaments();
      navigate(`/tournaments/${tournamentId}`);
    } catch (err) {
      console.error('Failed to import backup:', err);
      setImportError(t('tournaments.importFailed', {
        error: err instanceof Error ? err.message : String(err),
      }));
    } finally {
      setImporting(false);
    }
  };

  const formatDate = (dateString: string) => {
    return new Date(dateString).toLocaleDateString();
  };

  return (
    <div className="space-y-6">
      <div className="flex items-center justify-between">
        <h1 className="text-2xl font-bold text-gray-900">{t('tournaments.title')}</h1>
        <div className="flex flex-wrap gap-2">
          <Button variant="secondary" onClick={handleImport} disabled={importing}>
            {importing ? t('common.loading') : t('tournaments.importBackup')}
          </Button>
          <Link to="/tournaments/new">
            <Button>{t('tournaments.create')}</Button>
          </Link>
        </div>
      </div>

      {importError && (
        <div className="rounded-md bg-red-50 p-4 text-sm text-red-700">{importError}</div>
      )}

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
