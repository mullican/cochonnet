import { useEffect, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { useTournamentStore } from '../../stores/tournamentStore';
import {
  Button,
  Card,
  CardContent,
  Input,
  TeamLabel,
} from '../../components/ui';
import type { GameWithTeams, PanacheSide, Team } from '../../types';
import { randomScores, useShortcut } from '../demo';
import { useDemoStore } from '../../stores/demoStore';

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
    fetchGamesForRound,
    updateGameScore,
    updateGameCourt,
    completeRound,
    fetchStandings,
    fetchQualifyingRounds,
    qualifyingSitouts,
    fetchSitoutsForRound,
    currentTournament,
  } = useTournamentStore();

  const [scores, setScores] = useState<Record<string, { team1: string; team2: string }>>({});
  const [courtDrafts, setCourtDrafts] = useState<Record<string, string>>({});
  const [courtSaveError, setCourtSaveError] = useState<string | null>(null);
  const [completeError, setCompleteError] = useState<string | null>(null);
  const [completing, setCompleting] = useState(false);
  const [initialLoading, setInitialLoading] = useState(true);
  const [search, setSearch] = useState('');
  const demoArmed = useDemoStore((state) => state.armed);
  const reportDemo = useDemoStore((state) => state.report);

  useEffect(() => {
    // Reset scores when switching rounds
    setScores({});
    setCourtDrafts({});
    setCourtSaveError(null);
    setCompleteError(null);
    setCompleting(false);
    setSearch('');
    setInitialLoading(true);
    fetchGamesForRound(roundId).finally(() => setInitialLoading(false));
    // Panache rounds may rest surplus players; other formats return an empty list.
    fetchSitoutsForRound(roundId);
  }, [roundId, fetchGamesForRound, fetchSitoutsForRound]);

  useEffect(() => {
    setScores((prevScores) => {
      const newScores: Record<string, { team1: string; team2: string }> = {};
      let hasChanges = false;

      qualifyingGames.forEach((game) => {
        const existingScore = prevScores[game.id];
        const backendTeam1 = game.team1Score?.toString() || '';
        const backendTeam2 = game.team2Score?.toString() || '';

        if (existingScore) {
          // Keep existing local scores to preserve user edits
          newScores[game.id] = existingScore;
        } else {
          // Initialize scores for new games
          hasChanges = true;
          newScores[game.id] = {
            team1: backendTeam1,
            team2: backendTeam2,
          };
        }
      });

      // Only update state if there are actual changes
      if (!hasChanges && Object.keys(newScores).length === Object.keys(prevScores).length) {
        return prevScores;
      }
      return newScores;
    });
  }, [qualifyingGames]);

  const getTeam = (teamId: string | null | undefined) =>
    teamId ? teams.find((t) => t.id === teamId) : undefined;

  /**
   * The registered entrants on one side of a game: a single team for the team
   * formats, or every member of the drawn side for panache. Both carry the
   * numbers scorekeepers search and sort by.
   */
  const sideEntrants = (
    side: PanacheSide | null | undefined,
    teamId: string | null | undefined
  ): Team[] => {
    if (side) return side.members;
    const team = teams.find((t) => t.id === teamId);
    return team ? [team] : [];
  };

  const gameEntrants = (game: GameWithTeams): Team[] => [
    ...sideEntrants(game.side1, game.team1Id),
    ...sideEntrants(game.side2, game.team2Id),
  ];

  /**
   * Score slips come in by number, so the lowest number in a game orders it.
   * Court number breaks ties and covers a game whose teams have not loaded.
   */
  const lowestNumber = (game: GameWithTeams): number => {
    const numbers = gameEntrants(game).map((team) => team.teamNumber);
    return numbers.length > 0 ? Math.min(...numbers) : Number.MAX_SAFE_INTEGER;
  };

  /**
   * An all-digits query looks up team numbers by prefix, so typing "8" narrows
   * to 8, 80-89 and so on; anything else searches the players' names.
   */
  const matchesSearch = (game: GameWithTeams, query: string): boolean => {
    if (!query) return true;
    const entrants = gameEntrants(game);

    if (/^\d+$/.test(query)) {
      return entrants.some((team) => String(team.teamNumber).startsWith(query));
    }

    const needle = query.toLowerCase();
    return entrants.some((team) =>
      [team.captain, team.player2, team.player3]
        .some((name) => (name || '').toLowerCase().includes(needle))
    );
  };

  const courtCount = currentTournament?.numberOfCourts ?? 0;

  /**
   * A court can only be moved before the game is played. Once a score is in -
   * or the round is closed, which is what settles a bye - the court is a record
   * of where the game happened, not a plan for where it will.
   */
  const courtLocked = (game: GameWithTeams) =>
    isComplete || game.team1Score !== null || game.team2Score !== null;

  /**
   * What the court box shows: the operator's unsaved text if they are typing,
   * otherwise whatever is on the game.
   */
  const courtValue = (game: GameWithTeams) =>
    courtDrafts[game.id] ?? String(game.courtNumber);

  /**
   * Courts holding more than one game this round.
   *
   * Moving games around means passing through states where two share a court,
   * so this is shown rather than prevented - the director may well be halfway
   * through a shuffle.
   */
  const clashingCourts = new Set(
    Object.entries(
      qualifyingGames.reduce<Record<number, number>>((counts, game) => {
        counts[game.courtNumber] = (counts[game.courtNumber] || 0) + 1;
        return counts;
      }, {})
    )
      .filter(([, count]) => count > 1)
      .map(([court]) => Number(court))
  );

  const courtError = (game: GameWithTeams): string | undefined => {
    if (clashingCourts.has(game.courtNumber)) return t('pairing.courtClash');
    // The court count can be lowered after a schedule is drawn, which leaves
    // perfectly valid assignments pointing at courts the venue no longer has.
    if (courtCount > 0 && (game.courtNumber < 1 || game.courtNumber > courtCount)) {
      return t('pairing.courtOutOfRange', { count: courtCount });
    }
    return undefined;
  };

  const handleCourtChange = (gameId: string, value: string) => {
    setCourtDrafts((prev) => ({ ...prev, [gameId]: value }));
  };

  const handleSaveCourt = async (game: GameWithTeams) => {
    const draft = courtDrafts[game.id];
    if (draft === undefined || courtLocked(game)) return;

    const court = parseInt(draft, 10);
    if (isNaN(court) || court < 1) {
      // Nonsense reverts rather than being written; an empty box is a slip.
      setCourtDrafts((prev) => {
        const { [game.id]: _discarded, ...rest } = prev;
        return rest;
      });
      return;
    }

    if (court !== game.courtNumber) {
      try {
        await updateGameCourt(game.id, court);
      } catch (error) {
        // A court that did not save has to say so. Leaving the draft in the
        // box keeps what the operator typed in front of them rather than
        // snapping back to the old number as though nothing happened.
        setCourtSaveError(String(error));
        return;
      }
    }
    setCourtSaveError(null);
    setCourtDrafts((prev) => {
      const { [game.id]: _saved, ...rest } = prev;
      return rest;
    });
  };

  const query = search.trim();
  const orderedGames = [...qualifyingGames].sort(
    (a, b) => lowestNumber(a) - lowestNumber(b) || a.courtNumber - b.courtNumber
  );
  const visibleGames = orderedGames.filter((game) => matchesSearch(game, query));

  /**
   * One side of a game. Panache draws a temporary team, so its members are listed
   * one per line; every other format shows the registered team's number and name.
   */
  const SideLabel = ({
    side,
    teamId,
  }: {
    side: PanacheSide | null | undefined;
    teamId: string | null | undefined;
  }) => {
    if (!side)
      return (
        <div className="font-medium">
          <TeamLabel team={getTeam(teamId)} className="max-w-full" />
        </div>
      );
    return (
      <div className="font-medium leading-tight">
        {side.members.map((member) => (
          <div key={member.id} className="truncate">
            <TeamLabel team={member} className="max-w-full" />
            {member.isChampion && <span className="ml-1 text-amber-600">★</span>}
          </div>
        ))}
      </div>
    );
  };

  const handleScoreChange = (gameId: string, team: 'team1' | 'team2', value: string) => {
    setCompleteError(null);
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
    // Saving the scores takes one round-trip per game, which on a 67-game round
    // is a visible pause with no feedback - long enough for a second click to
    // land and complete the round twice. Both halves of that are fixed: the
    // button goes disabled below, and `complete_round` refuses to apply a round
    // it has already applied.
    if (completing || isComplete) return;
    setCompleting(true);
    try {
      await completeRoundInner();
    } finally {
      setCompleting(false);
    }
  };

  const completeRoundInner = async () => {
    const allGamesScored = qualifyingGames.every((game) => {
      if (game.isBye) return true;
      const gameScores = scores[game.id];
      if (!gameScores) return false;
      return gameScores.team1 !== '' && gameScores.team2 !== '';
    });

    if (!allGamesScored) {
      setCompleteError(t('pairing.completeRoundMissingScores'));
      return;
    }
    setCompleteError(null);

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
      // A refusal from the backend was just as silent as the missing-score one.
      setCompleteError(String(error));
    }
  };

  /**
   * Fills every unscored game in this round with an invented but plausible
   * result, so a demo can walk a tournament through without typing hundreds of
   * scores. Armed from the badge; see `src/features/demo`.
   *
   * It fills the whole round rather than only the games the search box is
   * showing, because a round cannot be completed until every game has a score -
   * filling the filtered subset would leave the demo stuck.
   */
  const handleDemoFill = async () => {
    const unscored = qualifyingGames.filter((game) => {
      // A bye already carries its 13-7; there is nothing to invent.
      if (game.isBye) return false;
      // Never overwrite: a score already saved, or typed and not yet blurred,
      // belongs to the operator.
      if (game.team1Score !== null || game.team2Score !== null) return false;
      const typed = scores[game.id];
      if (typed && (typed.team1 !== '' || typed.team2 !== '')) return false;
      return true;
    });

    if (unscored.length === 0) {
      reportDemo(t('demo.nothingToFill'));
      return;
    }

    const drawn = new Map(unscored.map((game) => [game.id, randomScores()]));
    setScores((prev) => {
      const next = { ...prev };
      drawn.forEach((result, gameId) => {
        next[gameId] = { team1: String(result.team1), team2: String(result.team2) };
      });
      return next;
    });

    // Sequential: each call takes the database lock, and a demo of 60-odd games
    // is still faster than the operator can see.
    for (const [gameId, result] of drawn) {
      await updateGameScore(gameId, result.team1, result.team2);
    }
    reportDemo(t('demo.filledGames', { count: drawn.size }));
  };

  // A completed round has nothing to fill, and its inputs are already disabled.
  useShortcut('KeyR', handleDemoFill, demoArmed && !isComplete);

  if (initialLoading) {
    return <div className="text-center py-4 text-gray-500">{t('common.loading')}</div>;
  }

  return (
    <div className="space-y-4">
      {qualifyingSitouts.length > 0 && (
        <div className="rounded-md bg-gray-50 px-4 py-3 text-sm">
          <span className="font-medium text-gray-700">{t('pairing.sittingOut')}: </span>
          <span className="inline-flex flex-wrap items-baseline gap-x-3 gap-y-1 text-gray-600">
            {qualifyingSitouts.map((p) => (
              <TeamLabel key={p.id} team={p} />
            ))}
          </span>
        </div>
      )}

      {courtSaveError && (
        <div className="rounded-md bg-red-50 p-4 text-sm text-red-700">{courtSaveError}</div>
      )}

      {clashingCourts.size > 0 && (
        <div className="rounded-md bg-red-50 p-4 text-sm text-red-700">
          {t('pairing.courtClashSummary')}
        </div>
      )}

      <div className="flex items-center gap-3">
        <div className="w-full max-w-xs">
          <Input
            type="search"
            value={search}
            onChange={(e) => setSearch(e.target.value)}
            placeholder={t('pairing.searchGames')}
          />
        </div>
        <span className="whitespace-nowrap text-sm text-gray-500">
          {t('pairing.gamesShown', {
            shown: visibleGames.length,
            total: qualifyingGames.length,
          })}
        </span>
        {!isComplete && (
          <Button className="ml-auto" onClick={handleCompleteRound} disabled={completing}>
            {completing ? t('pairing.completingRound') : t('pairing.completeRound')}
          </Button>
        )}
      </div>

      {completeError && (
        <div className="rounded-md bg-red-50 p-4 text-sm text-red-700">{completeError}</div>
      )}

      {visibleGames.length === 0 ? (
        <Card>
          <CardContent className="py-8 text-center text-gray-500">
            {t('pairing.noGamesMatch')}
          </CardContent>
        </Card>
      ) : (
        <div className="grid gap-4 sm:grid-cols-2 lg:grid-cols-3">
          {visibleGames.map((game) => (
            <Card
              key={game.id}
              className={courtError(game) ? 'border-red-400' : undefined}
            >
              <CardContent className="py-4">
                <div className="mb-3">
                  {courtLocked(game) ? (
                    <div className="text-xs text-gray-500">
                      {t('pairing.court')} {game.courtNumber}
                    </div>
                  ) : (
                    <div className="flex items-center gap-2">
                      <label
                        htmlFor={`court-${game.id}`}
                        className="text-xs text-gray-500"
                      >
                        {t('pairing.court')}
                      </label>
                      {/* Three digits is more courts than any venue has. */}
                      <Input
                        id={`court-${game.id}`}
                        type="number"
                        min={1}
                        max={999}
                        value={courtValue(game)}
                        onChange={(e) => handleCourtChange(game.id, e.target.value)}
                        onBlur={() => handleSaveCourt(game)}
                        className={`w-14 px-1 text-center ${
                          courtError(game) ? 'border-red-500 focus:border-red-500' : ''
                        }`}
                      />
                    </div>
                  )}
                  {/* Shown, not enforced: shuffling games around means passing
                      through a double-booking, and being blocked mid-shuffle is
                      worse than the clash itself. */}
                  {courtError(game) && (
                    <p className="mt-1 text-xs text-red-600">{courtError(game)}</p>
                  )}
                </div>

                {game.isBye ? (
                  <div className="text-center">
                    <div className="font-medium">
                      <TeamLabel team={getTeam(game.team1Id)} />
                    </div>
                    <div className="text-sm text-gray-500 mt-2">{t('pairing.bye')}</div>
                    <div className="text-sm text-green-600 mt-1">13 - 7</div>
                  </div>
                ) : (
                  <div className="space-y-3">
                    <div className="flex items-center gap-2">
                      <div className="flex-1 min-w-0">
                        <SideLabel side={game.side1} teamId={game.team1Id} />
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
                      <div className="flex-1 min-w-0">
                        <SideLabel side={game.side2} teamId={game.team2Id} />
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
      )}
    </div>
  );
}
