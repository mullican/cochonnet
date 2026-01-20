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
    loading,
    fetchQualifyingRounds,
    generatePairings,
    fetchStandings,
    teams,
  } = useTournamentStore();

  const [selectedRoundId, setSelectedRoundId] = useState<string | null>(null);

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

  const handleGeneratePairings = async () => {
    try {
      const round = await generatePairings(tournamentId);
      setSelectedRoundId(round.id);
    } catch (error) {
      console.error('Failed to generate pairings:', error);
    }
  };

  const canGeneratePairings = teams.length >= 2;
  const hasIncompleteRound = qualifyingRounds.some((r) => !r.isComplete);

  return (
    <div className="space-y-6">
      <div className="flex items-center justify-between">
        <h2 className="text-lg font-semibold text-gray-900">{t('pairing.title')}</h2>
        <Button
          onClick={handleGeneratePairings}
          disabled={!canGeneratePairings || hasIncompleteRound || loading}
        >
          {hasIncompleteRound ? t('pairing.completeRound') : t('pairing.generatePairings')}
        </Button>
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
    </div>
  );
}
