import { useEffect, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { useTournamentStore } from '../../stores/tournamentStore';
import {
  Button,
  Card,
  CardContent,
  Input,
} from '../../components/ui';
import { formatTeamName } from '../../lib/utils';

interface RoundGamesProps {
  roundId: string;
  tournamentId: string;
  isComplete: boolean;
}

export function RoundGames({ roundId, tournamentId, isComplete }: RoundGamesProps) {
  const { t } = useTranslation();
  const {
    qualifyingGames,
    teams,
    loading,
    fetchGamesForRound,
    updateGameScore,
    completeRound,
    fetchStandings,
    fetchQualifyingRounds,
  } = useTournamentStore();

  const [scores, setScores] = useState<Record<string, { team1: string; team2: string }>>({});

  useEffect(() => {
    fetchGamesForRound(roundId);
  }, [roundId, fetchGamesForRound]);

  useEffect(() => {
    const initialScores: Record<string, { team1: string; team2: string }> = {};
    qualifyingGames.forEach((game) => {
      initialScores[game.id] = {
        team1: game.team1Score?.toString() || '',
        team2: game.team2Score?.toString() || '',
      };
    });
    setScores(initialScores);
  }, [qualifyingGames]);

  const getTeamName = (teamId: string | null | undefined) => {
    if (!teamId) return 'TBD';
    const team = teams.find((t) => t.id === teamId);
    return formatTeamName(team?.captain);
  };

  const handleScoreChange = (gameId: string, team: 'team1' | 'team2', value: string) => {
    setScores((prev) => ({
      ...prev,
      [gameId]: {
        ...prev[gameId],
        [team]: value,
      },
    }));
  };

  const handleSaveScore = async (gameId: string) => {
    const gameScores = scores[gameId];
    if (!gameScores) return;

    const team1Score = parseInt(gameScores.team1);
    const team2Score = parseInt(gameScores.team2);

    if (isNaN(team1Score) || isNaN(team2Score)) {
      return;
    }

    try {
      await updateGameScore(gameId, team1Score, team2Score);
    } catch (error) {
      console.error('Failed to save score:', error);
    }
  };

  const handleCompleteRound = async () => {
    const allGamesScored = qualifyingGames.every((game) => {
      if (game.isBye) return true;
      const gameScores = scores[game.id];
      if (!gameScores) return false;
      return gameScores.team1 !== '' && gameScores.team2 !== '';
    });

    if (!allGamesScored) {
      alert('Please enter scores for all games before completing the round.');
      return;
    }

    for (const game of qualifyingGames) {
      if (!game.isBye) {
        await handleSaveScore(game.id);
      }
    }

    try {
      await completeRound(roundId);
      await fetchStandings(tournamentId);
      await fetchQualifyingRounds(tournamentId);
    } catch (error) {
      console.error('Failed to complete round:', error);
    }
  };

  if (loading) {
    return <div className="text-center py-4 text-gray-500">{t('common.loading')}</div>;
  }

  return (
    <div className="space-y-4">
      <div className="grid gap-4 sm:grid-cols-2 lg:grid-cols-3">
        {qualifyingGames.map((game) => (
          <Card key={game.id}>
            <CardContent className="py-4">
              <div className="text-xs text-gray-500 mb-2">
                {t('pairing.court')} {game.courtNumber}
              </div>

              {game.isBye ? (
                <div className="text-center">
                  <div className="font-medium">{getTeamName(game.team1Id)}</div>
                  <div className="text-sm text-gray-500 mt-2">{t('pairing.bye')}</div>
                  <div className="text-sm text-green-600 mt-1">13 - 7</div>
                </div>
              ) : (
                <div className="space-y-3">
                  <div className="flex items-center gap-2">
                    <div className="flex-1">
                      <div className="font-medium truncate">
                        {getTeamName(game.team1Id)}
                      </div>
                    </div>
                    <Input
                      type="number"
                      min={0}
                      max={13}
                      value={scores[game.id]?.team1 || ''}
                      onChange={(e) => handleScoreChange(game.id, 'team1', e.target.value)}
                      onBlur={() => handleSaveScore(game.id)}
                      disabled={isComplete}
                      className="w-16 text-center"
                    />
                  </div>

                  <div className="text-center text-xs text-gray-400">
                    {t('pairing.vs')}
                  </div>

                  <div className="flex items-center gap-2">
                    <div className="flex-1">
                      <div className="font-medium truncate">
                        {getTeamName(game.team2Id)}
                      </div>
                    </div>
                    <Input
                      type="number"
                      min={0}
                      max={13}
                      value={scores[game.id]?.team2 || ''}
                      onChange={(e) => handleScoreChange(game.id, 'team2', e.target.value)}
                      onBlur={() => handleSaveScore(game.id)}
                      disabled={isComplete}
                      className="w-16 text-center"
                    />
                  </div>
                </div>
              )}
            </CardContent>
          </Card>
        ))}
      </div>

      {!isComplete && (
        <div className="flex justify-end">
          <Button onClick={handleCompleteRound}>{t('pairing.completeRound')}</Button>
        </div>
      )}
    </div>
  );
}
