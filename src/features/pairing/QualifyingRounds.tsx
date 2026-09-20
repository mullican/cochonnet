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
    generatePanacheRounds,
    generatePanacheRound,
    redrawPanacheRounds,
    generatePanacheFinal,
    deleteAllQualifyingRounds,
    fetchStandings,
    teams,
    currentTournament,
  } = useTournamentStore();

  const [selectedRoundId, setSelectedRoundId] = useState<string | null>(null);
  const [deleteDialogOpen, setDeleteDialogOpen] = useState(false);
  const [deleteError, setDeleteError] = useState<string | null>(null);
  const [redrawFromRound, setRedrawFromRound] = useState<number | null>(null);
  const [actionError, setActionError] = useState<string | null>(null);

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

  const pairingMethod = currentTournament?.pairingMethod || 'swiss';
  const isPanache = pairingMethod === 'panache';

  const handleGenerateAllRounds = async () => {
    setActionError(null);
    try {
      const rounds = await generateAllQualifyingRounds(tournamentId);
      if (rounds.length > 0) {
        setSelectedRoundId(rounds[0].id);
      }
    } catch (error) {
      setActionError(String(error));
    }
  };

  // Panache draws through its own scheduler; every other format shares one.
  const handleGenerateNextRound = async () => {
    setActionError(null);
    try {
      const round = isPanache
        ? await generatePanacheRound(tournamentId)
        : await generatePairings(tournamentId);
      setSelectedRoundId(round.id);
    } catch (error) {
      setActionError(String(error));
    }
  };

  const handleGeneratePanacheRounds = async () => {
    setActionError(null);
    try {
      const rounds = await generatePanacheRounds(tournamentId);
      if (rounds.length > 0) {
        setSelectedRoundId(rounds[0].id);
      }
    } catch (error) {
      setActionError(String(error));
    }
  };

  const handleRedrawRounds = async () => {
    if (redrawFromRound === null) return;
    setActionError(null);
    try {
      const rounds = await redrawPanacheRounds(tournamentId, redrawFromRound);
      setRedrawFromRound(null);
      setSelectedRoundId(rounds.length > 0 ? rounds[0].id : null);
    } catch (error) {
      setActionError(String(error));
      setRedrawFromRound(null);
    }
  };

  const handleGenerateFinal = async () => {
    setActionError(null);
    try {
      const round = await generatePanacheFinal(tournamentId);
      setSelectedRoundId(round.id);
    } catch (error) {
      setActionError(String(error));
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

  // Panache needs enough individuals to fill both sides of one game.
  const panacheTeamSize = currentTournament?.format === 'triple' ? 3 : 2;
  const minimumEntrants = isPanache ? panacheTeamSize * 2 : 2;
  // Withdrawn entrants are not going on a court again, so they do not count
  // towards a draw: what matters is who is still in.
  const activeTeams = teams.filter((team) => !team.isWithdrawn);
  const canGeneratePairings = activeTeams.length >= minimumEntrants;
  const hasRounds = qualifyingRounds.length > 0;

  // Check if any games have scores - if so, deletion is not allowed
  const hasScores = qualifyingGames.some(
    (g) => g.team1Score !== null || g.team2Score !== null
  );
  const canDeleteRounds = hasRounds && !hasScores;

  // Swiss and Pool Play build each round from the last one's results, so they
  // cannot be drawn ahead. The rest can be drawn either way: up front, which is
  // what these formats are usually chosen for, or a round at a time, which is
  // what a director does when the roster may still move.
  const requiresRoundByRound = pairingMethod === 'swiss' || pairingMethod === 'poolPlay';

  // Panache ends with a single final game once every qualifying round is scored.
  const qualifyingOnlyRounds = qualifyingRounds.filter((r) => !r.isFinal);
  const hasFinal = qualifyingRounds.some((r) => r.isFinal);
  const allQualifyingComplete =
    qualifyingOnlyRounds.length > 0 && qualifyingOnlyRounds.every((r) => r.isComplete);

  const lastRound = qualifyingOnlyRounds[qualifyingOnlyRounds.length - 1];
  const maxRounds = pairingMethod === 'poolPlay' ? 3 : (currentTournament?.numberOfQualifyingRounds || 5);

  const canGenerateNextRound =
    canGeneratePairings &&
    !hasFinal &&
    (!lastRound || lastRound.isComplete) &&
    qualifyingOnlyRounds.length < maxRounds;

  const selectedRound = qualifyingRounds.find((r) => r.id === selectedRoundId) || null;

  // Determine which generate button to show
  const showGenerateAllButton = !hasRounds && !requiresRoundByRound && !isPanache;
  const showGenerateNextButton = canGenerateNextRound;
  const showGeneratePanacheButton = isPanache && !hasRounds;
  const showGenerateFinalButton = isPanache && allQualifyingComplete && !hasFinal;

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
              // Drawing the whole schedule is the usual choice for the formats
              // that offer it, so one round at a time sits beside it as the
              // alternative rather than competing with it.
              variant={showGenerateAllButton || showGeneratePanacheButton ? 'secondary' : 'primary'}
              onClick={handleGenerateNextRound}
              disabled={loading}
            >
              {t('pairing.generateNextRound')}
            </Button>
          )}
          {showGeneratePanacheButton && (
            <Button
              onClick={handleGeneratePanacheRounds}
              disabled={!canGeneratePairings || loading}
            >
              {t('pairing.generateAllRounds')}
            </Button>
          )}
          {showGenerateFinalButton && (
            <Button onClick={handleGenerateFinal} disabled={loading}>
              {t('pairing.generateFinal')}
            </Button>
          )}
        </div>
      </div>

      {actionError && (
        <div className="rounded-md bg-red-50 p-4 text-sm text-red-700">{actionError}</div>
      )}

      {!canGeneratePairings && (
        <Card>
          <CardContent className="py-8 text-center text-gray-500">
            {isPanache
              ? t('pairing.notEnoughPlayers', { count: minimumEntrants })
              : t('teams.noTeams')}
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
              <div className="flex flex-wrap gap-2">
                {qualifyingRounds.map((round) => (
                  <Button
                    key={round.id}
                    variant={selectedRoundId === round.id ? 'primary' : 'secondary'}
                    size="sm"
                    onClick={() => setSelectedRoundId(round.id)}
                  >
                    {round.isFinal
                      ? t('pairing.final')
                      : t('pairing.round', { number: round.roundNumber })}
                    {round.isComplete && ' ✓'}
                  </Button>
                ))}
              </div>

              {/* A panache round that has not been scored can be re-shuffled against
                  the current roster; earlier rounds are held as drawn. */}
              {isPanache && selectedRound && !selectedRound.isFinal && !selectedRound.isComplete && !hasScores && (
                <div className="flex items-center gap-2 text-sm text-gray-500">
                  <span>
                    {t('pairing.redrawFrom', { number: selectedRound.roundNumber })}
                  </span>
                  <Button
                    variant="secondary"
                    size="sm"
                    onClick={() => setRedrawFromRound(selectedRound.roundNumber)}
                    disabled={loading}
                  >
                    {t('pairing.redraw')}
                  </Button>
                </div>
              )}

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

      {/* Redraw Confirmation Dialog */}
      <Dialog
        open={redrawFromRound !== null}
        onOpenChange={(open) => !open && setRedrawFromRound(null)}
      >
        <DialogContent>
          <DialogHeader>
            <DialogTitle>{t('pairing.redraw')}</DialogTitle>
          </DialogHeader>
          <p className="text-sm text-gray-500">
            {t('pairing.redrawConfirm', { number: redrawFromRound ?? 0 })}
          </p>
          <DialogFooter>
            <Button variant="secondary" onClick={() => setRedrawFromRound(null)}>
              {t('common.cancel')}
            </Button>
            <Button onClick={handleRedrawRounds} disabled={loading}>
              {t('pairing.redraw')}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>

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
