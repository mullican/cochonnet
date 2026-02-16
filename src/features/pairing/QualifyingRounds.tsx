import { useEffect, useState } from 'react';
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
  DialogFooter,
} from '../../components/ui';
import { RoundGames } from './RoundGames';
import { StandingsTable } from './StandingsTable';

interface QualifyingRoundsProps {
  tournamentId: string;
}

export function QualifyingRounds({ tournamentId }: QualifyingRoundsProps) {
  const { t } = useTranslation();
  const {
    qualifyingRounds,
    qualifyingGames,
    loading,
    fetchQualifyingRounds,
    generateAllQualifyingRounds,
    generatePairings,
    deleteAllQualifyingRounds,
    fetchStandings,
    teams,
    currentTournament,
  } = useTournamentStore();

  const [selectedRoundId, setSelectedRoundId] = useState<string | null>(null);
  const [deleteDialogOpen, setDeleteDialogOpen] = useState(false);
  const [deleteError, setDeleteError] = useState<string | null>(null);

  useEffect(() => {
    fetchQualifyingRounds(tournamentId);
    fetchStandings(tournamentId);
  }, [tournamentId, fetchQualifyingRounds, fetchStandings]);

  useEffect(() => {
    if (qualifyingRounds.length > 0 && !selectedRoundId) {
      const latestRound = qualifyingRounds[qualifyingRounds.length - 1];
      setSelectedRoundId(latestRound.id);
    }
  }, [qualifyingRounds, selectedRoundId]);

  const handleGenerateAllRounds = async () => {
    try {
      const rounds = await generateAllQualifyingRounds(tournamentId);
      if (rounds.length > 0) {
        setSelectedRoundId(rounds[0].id);
      }
    } catch (error) {
      console.error('Failed to generate pairings:', error);
    }
  };

  const handleGenerateNextRound = async () => {
    try {
      const round = await generatePairings(tournamentId);
      setSelectedRoundId(round.id);
    } catch (error) {
      console.error('Failed to generate next round:', error);
    }
  };

  const handleDeleteRounds = async () => {
    setDeleteError(null);
    try {
      await deleteAllQualifyingRounds(tournamentId);
      setSelectedRoundId(null);
      setDeleteDialogOpen(false);
    } catch (error) {
      setDeleteError(String(error));
    }
  };

  const canGeneratePairings = teams.length >= 2;
  const hasRounds = qualifyingRounds.length > 0;
  const pairingMethod = currentTournament?.pairingMethod || 'swiss';

  // Check if any games have scores - if so, deletion is not allowed
  const hasScores = qualifyingGames.some(
    (g) => g.team1Score !== null || g.team2Score !== null
  );
  const canDeleteRounds = hasRounds && !hasScores;

  // For Swiss and Pool Play: can generate next round if prior round is complete
  const lastRound = qualifyingRounds[qualifyingRounds.length - 1];
  const maxRounds = pairingMethod === 'poolPlay' ? 3 : (currentTournament?.numberOfQualifyingRounds || 5);
  const requiresRoundByRound = pairingMethod === 'swiss' || pairingMethod === 'poolPlay';

  const canGenerateNextRound = requiresRoundByRound &&
    canGeneratePairings &&
    (!lastRound || lastRound.isComplete) &&
    qualifyingRounds.length < maxRounds;

  // Determine which generate button to show
  const showGenerateAllButton = !hasRounds && !requiresRoundByRound;
  const showGenerateNextButton = requiresRoundByRound && canGenerateNextRound;

  return (
    <div className="space-y-6">
      <div className="flex items-center justify-between">
        <h2 className="text-lg font-semibold text-gray-900">{t('pairing.title')}</h2>
        <div className="flex gap-2">
          {hasRounds && canDeleteRounds && (
            <Button
              variant="danger"
              onClick={() => setDeleteDialogOpen(true)}
              disabled={loading}
            >
              {t('pairing.deleteRounds')}
            </Button>
          )}
          {showGenerateAllButton && (
            <Button
              onClick={handleGenerateAllRounds}
              disabled={!canGeneratePairings || loading}
            >
              {t('pairing.generatePairings')}
            </Button>
          )}
          {showGenerateNextButton && (
            <Button
              onClick={handleGenerateNextRound}
              disabled={loading}
            >
              {t('pairing.generateNextRound')}
            </Button>
          )}
        </div>
      </div>

      {!canGeneratePairings && (
        <Card>
          <CardContent className="py-8 text-center text-gray-500">
            {t('teams.noTeams')}
          </CardContent>
        </Card>
      )}

      {canGeneratePairings && qualifyingRounds.length === 0 && (
        <Card>
          <CardContent className="py-8 text-center text-gray-500">
            {t('pairing.noRounds')}
          </CardContent>
        </Card>
      )}

      {qualifyingRounds.length > 0 && (
        <Tabs defaultValue="rounds">
          <TabsList>
            <TabsTrigger value="rounds">{t('pairing.title')}</TabsTrigger>
            <TabsTrigger value="standings">{t('pairing.standings')}</TabsTrigger>
          </TabsList>

          <TabsContent value="rounds" className="mt-4">
            <div className="space-y-4">
              <div className="flex gap-2">
                {qualifyingRounds.map((round) => (
                  <Button
                    key={round.id}
                    variant={selectedRoundId === round.id ? 'primary' : 'secondary'}
                    size="sm"
                    onClick={() => setSelectedRoundId(round.id)}
                  >
                    {t('pairing.round', { number: round.roundNumber })}
                    {round.isComplete && ' ✓'}
                  </Button>
                ))}
              </div>

              {selectedRoundId && (
                <RoundGames
                  roundId={selectedRoundId}
                  tournamentId={tournamentId}
                  isComplete={
                    qualifyingRounds.find((r) => r.id === selectedRoundId)?.isComplete || false
                  }
                />
              )}
            </div>
          </TabsContent>

          <TabsContent value="standings" className="mt-4">
            <StandingsTable tournamentId={tournamentId} />
          </TabsContent>
        </Tabs>
      )}

      {/* Delete Rounds Confirmation Dialog */}
      <Dialog open={deleteDialogOpen} onOpenChange={setDeleteDialogOpen}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>{t('pairing.deleteRounds')}</DialogTitle>
          </DialogHeader>
          <p className="text-sm text-gray-500">{t('pairing.deleteRoundsConfirm')}</p>
          {deleteError && (
            <div className="rounded-md bg-red-50 p-3 text-sm text-red-700">
              {deleteError}
            </div>
          )}
          <DialogFooter>
            <Button
              variant="secondary"
              onClick={() => {
                setDeleteDialogOpen(false);
                setDeleteError(null);
              }}
            >
              {t('common.cancel')}
            </Button>
            <Button variant="danger" onClick={handleDeleteRounds} disabled={loading}>
              {t('common.delete')}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </div>
  );
}
