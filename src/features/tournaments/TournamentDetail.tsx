import { useEffect, useState } from 'react';
import { useParams, useNavigate, Link } from 'react-router-dom';
import { useTranslation } from 'react-i18next';
import { useTournamentStore } from '../../stores/tournamentStore';
import {
  Button,
  Card,
  CardContent,
  Tabs,
  TabsList,
  TabsTrigger,
  TabsContent,
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
  DialogDescription,
  DialogFooter,
} from '../../components/ui';
import { TeamsList } from '../teams/TeamsList';
import { QualifyingRounds } from '../pairing/QualifyingRounds';
import { BracketView } from '../brackets/BracketView';
import { ExportView } from '../export/ExportView';

export function TournamentDetail() {
  const { id } = useParams<{ id: string }>();
  const { t } = useTranslation();
  const navigate = useNavigate();
  const {
    currentTournament,
    loading,
    fetchTournament,
    deleteTournament,
    fetchTeams,
    teams,
  } = useTournamentStore();
  const [deleteDialogOpen, setDeleteDialogOpen] = useState(false);

  useEffect(() => {
    if (id) {
      fetchTournament(id);
      fetchTeams(id);
    }
  }, [id, fetchTournament, fetchTeams]);

  const handleDelete = async () => {
    if (id) {
      try {
        await deleteTournament(id);
        navigate('/');
      } catch (error) {
        console.error('Failed to delete tournament:', error);
      }
    }
  };

  const formatDate = (dateString: string) => {
    return new Date(dateString).toLocaleDateString();
  };

  if (loading && !currentTournament) {
    return <div className="text-center py-8 text-gray-500">{t('common.loading')}</div>;
  }

  if (!currentTournament) {
    return <div className="text-center py-8 text-gray-500">{t('common.error')}</div>;
  }

  return (
    <div className="space-y-6">
      <div className="flex items-center justify-between">
        <div className="flex items-center gap-4">
          <button
            onClick={() => navigate('/')}
            className="text-gray-500 hover:text-gray-700"
          >
            <svg
              xmlns="http://www.w3.org/2000/svg"
              width="24"
              height="24"
              viewBox="0 0 24 24"
              fill="none"
              stroke="currentColor"
              strokeWidth="2"
              strokeLinecap="round"
              strokeLinejoin="round"
            >
              <path d="m15 18-6-6 6-6" />
            </svg>
          </button>
          <h1 className="text-2xl font-bold text-gray-900">{currentTournament.name}</h1>
        </div>
        <div className="flex gap-2">
          <Link to={`/tournaments/${id}/edit`}>
            <Button variant="secondary">{t('common.edit')}</Button>
          </Link>
          <Button variant="danger" onClick={() => setDeleteDialogOpen(true)}>
            {t('common.delete')}
          </Button>
        </div>
      </div>

      <Card>
        <CardContent className="py-4">
          <div className="grid gap-4 sm:grid-cols-2 lg:grid-cols-4 text-sm">
            <div>
              <span className="text-gray-500">{t('tournaments.type')}</span>
              <p className="font-medium">{t(`tournaments.typeOptions.${currentTournament.type}`)}</p>
            </div>
            <div>
              <span className="text-gray-500">{t('tournaments.format')}</span>
              <p className="font-medium">{t(`tournaments.formatOptions.${currentTournament.format}`)}</p>
            </div>
            <div>
              <span className="text-gray-500">{t('tournaments.startDate')}</span>
              <p className="font-medium">{formatDate(currentTournament.startDate)}</p>
            </div>
            <div>
              <span className="text-gray-500">{t('tournaments.teamsRegistered')}</span>
              <p className="font-medium">{teams.length}</p>
            </div>
            <div>
              <span className="text-gray-500">{t('tournaments.director')}</span>
              <p className="font-medium">{currentTournament.director}</p>
            </div>
            <div>
              <span className="text-gray-500">{t('tournaments.headUmpire')}</span>
              <p className="font-medium">{currentTournament.headUmpire}</p>
            </div>
            <div>
              <span className="text-gray-500">{t('tournaments.numberOfCourts')}</span>
              <p className="font-medium">{currentTournament.numberOfCourts}</p>
            </div>
            <div>
              <span className="text-gray-500">{t('tournaments.pairingMethod')}</span>
              <p className="font-medium">{t(`tournaments.pairingMethodOptions.${currentTournament.pairingMethod}`)}</p>
            </div>
          </div>
        </CardContent>
      </Card>

      <Tabs defaultValue="teams">
        <TabsList>
          <TabsTrigger value="teams">{t('nav.teams')}</TabsTrigger>
          <TabsTrigger value="qualifying">{t('nav.qualifying')}</TabsTrigger>
          <TabsTrigger value="brackets">{t('nav.brackets')}</TabsTrigger>
          <TabsTrigger value="export">{t('nav.export')}</TabsTrigger>
        </TabsList>

        <TabsContent value="teams" className="mt-4">
          <TeamsList tournamentId={id!} />
        </TabsContent>

        <TabsContent value="qualifying" className="mt-4">
          <QualifyingRounds tournamentId={id!} />
        </TabsContent>

        <TabsContent value="brackets" className="mt-4">
          <BracketView tournamentId={id!} />
        </TabsContent>

        <TabsContent value="export" className="mt-4">
          <ExportView tournamentId={id!} />
        </TabsContent>
      </Tabs>

      <Dialog open={deleteDialogOpen} onOpenChange={setDeleteDialogOpen}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>{t('tournaments.delete')}</DialogTitle>
            <DialogDescription>{t('tournaments.deleteConfirm')}</DialogDescription>
          </DialogHeader>
          <DialogFooter>
            <Button variant="secondary" onClick={() => setDeleteDialogOpen(false)}>
              {t('common.cancel')}
            </Button>
            <Button variant="danger" onClick={handleDelete}>
              {t('common.delete')}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </div>
  );
}
