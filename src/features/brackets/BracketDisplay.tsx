import { useEffect, useState, useRef, useCallback } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { useTranslation } from 'react-i18next';
import { useTournamentStore } from '../../stores/tournamentStore';
import {
  Card,
  CardContent,
  Input,
  Button,
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
  DialogFooter,
  TeamLabel,
} from '../../components/ui';
import type { BracketMatch } from '../../types';
import { randomScores, useShortcut } from '../demo';
import { useDemoStore } from '../../stores/demoStore';

interface BracketDisplayProps {
  bracketId: string;
  bracketSize: number;
}

// Height of a match card in pixels
const MATCH_HEIGHT = 84;
// Width of connector lines
const CONNECTOR_WIDTH = 24;

export function BracketDisplay({ bracketId, bracketSize }: BracketDisplayProps) {
  const { t } = useTranslation();
  const {
    bracketMatches,
    teams,
    loading,
    fetchMatchesForBracket,
    updateMatchScore,
    updateMatchCourt,
    currentTournament,
  } = useTournamentStore();

  const [selectedMatch, setSelectedMatch] = useState<BracketMatch | null>(null);
  const [scoreDialogOpen, setScoreDialogOpen] = useState(false);
  const [team1Score, setTeam1Score] = useState('');
  const [team2Score, setTeam2Score] = useState('');
  const [court, setCourt] = useState('');
  const [saveError, setSaveError] = useState<string | null>(null);
  // Matches sharing a court with another game played at the same time. A wave
  // spans every bracket, and this view only holds one, so the backend works it
  // out - it is where the wave is defined.
  const [clashing, setClashing] = useState<Set<string>>(new Set());
  const containerRef = useRef<HTMLDivElement>(null);
  const demoArmed = useDemoStore((state) => state.armed);
  const reportDemo = useDemoStore((state) => state.report);

  const tournamentId = currentTournament?.id;

  const refreshConflicts = useCallback(async () => {
    if (!tournamentId) return;
    try {
      const ids = await invoke<string[]>('get_bracket_court_conflicts', { tournamentId });
      setClashing(new Set(ids));
    } catch (error) {
      console.error('Failed to check court conflicts:', error);
    }
  }, [tournamentId]);

  useEffect(() => {
    fetchMatchesForBracket(bracketId);
    refreshConflicts();
  }, [bracketId, fetchMatchesForBracket, refreshConflicts]);

  // A bracket of fewer than two entrants has no matches to lay out; log2 then
  // gives 0 (or -Infinity) rounds and the view renders an empty frame with no
  // explanation. Newly generated draws cannot be this shape, but an older
  // tournament may still hold one.
  const numRounds = Math.log2(bracketSize);
  const isDrawable = bracketSize >= 2 && Number.isInteger(numRounds);

  const getMatchesByRound = (roundNumber: number) => {
    return bracketMatches
      .filter((m) => m.roundNumber === roundNumber)
      .sort((a, b) => a.matchNumber - b.matchNumber);
  };

  const getRoundName = (roundNumber: number, totalRounds: number) => {
    const roundsFromEnd = totalRounds - roundNumber + 1;
    switch (roundsFromEnd) {
      case 1:
        return t('brackets.final');
      case 2:
        return t('brackets.semiFinal');
      case 3:
        return t('brackets.quarterFinal');
      default:
        return t('brackets.round', { number: roundNumber });
    }
  };

  const getTeam = (teamId: string | null | undefined) =>
    teamId ? teams.find((t) => t.id === teamId) : undefined;

  // Check if a match can be edited (has both teams, and next round match hasn't been scored)
  const canEditMatch = (match: BracketMatch) => {
    // Must have both teams to enter a score
    if (!match.team1Id || !match.team2Id) return false;

    // BYE matches can't be edited
    if (match.isBye) return false;

    // If there's a next match, check if it has been scored
    if (match.nextMatchId) {
      const nextMatch = bracketMatches.find((m) => m.id === match.nextMatchId);
      if (nextMatch && (nextMatch.team1Score !== null || nextMatch.team2Score !== null)) {
        // Next round has scores, can't edit this match
        return false;
      }
    }

    return true;
  };

  // Once a game has been played its court is a record of where it happened, so
  // only an unplayed match can be moved.
  const canEditCourt = (match: BracketMatch) =>
    !match.isBye && match.team1Score === null && match.team2Score === null;

  // The dialog is worth opening if either half of it is live.
  const canOpenMatch = (match: BracketMatch) =>
    canEditMatch(match) || canEditCourt(match);

  /**
   * Scores every match in this bracket that is ready to be played, which is
   * exactly one wave: `canEditMatch` requires both teams, so the next round's
   * matches only become fillable once these results have advanced into them.
   * Pressing again walks the bracket down one round at a time, the way the real
   * thing progresses.
   */
  const handleDemoFill = async () => {
    const ready = bracketMatches.filter(
      (match) =>
        canEditMatch(match) && match.team1Score === null && match.team2Score === null
    );

    if (ready.length === 0) {
      reportDemo(t('demo.nothingToFill'));
      return;
    }

    // Strictly sequential: every result re-runs the court draw for the wave, so
    // these cannot be fired off in parallel.
    for (const match of ready) {
      const result = randomScores();
      await updateMatchScore(match.id, result.team1, result.team2);
    }
    await refreshConflicts();
    reportDemo(t('demo.filledMatches', { count: ready.length }));
  };

  useShortcut('KeyR', handleDemoFill, demoArmed);

  const handleMatchClick = (match: BracketMatch) => {
    if (!canOpenMatch(match)) return;
    setSelectedMatch(match);
    setTeam1Score(match.team1Score?.toString() || '');
    setTeam2Score(match.team2Score?.toString() || '');
    setCourt(match.courtNumber?.toString() || '');
    setSaveError(null);
    setScoreDialogOpen(true);
  };

  const handleSave = async () => {
    if (!selectedMatch) return;
    setSaveError(null);

    const courtEditable = canEditCourt(selectedMatch);
    const nextCourt = parseInt(court, 10);
    if (courtEditable && court.trim() !== '' && (isNaN(nextCourt) || nextCourt < 1)) {
      setSaveError(t('validation.positiveNumber'));
      return;
    }

    const scoresEditable = canEditMatch(selectedMatch);
    const s1 = parseInt(team1Score);
    const s2 = parseInt(team2Score);
    const scoresEntered = !isNaN(s1) && !isNaN(s2);

    if (scoresEditable && scoresEntered && s1 === s2) {
      setSaveError(t('brackets.tiedScore'));
      return;
    }

    try {
      if (courtEditable && !isNaN(nextCourt) && nextCourt !== selectedMatch.courtNumber) {
        await updateMatchCourt(selectedMatch.id, nextCourt);
      }
      if (scoresEditable && scoresEntered) {
        await updateMatchScore(selectedMatch.id, s1, s2);
      }
      await fetchMatchesForBracket(bracketId);
      await refreshConflicts();
      setScoreDialogOpen(false);
      setSelectedMatch(null);
    } catch (error) {
      setSaveError(String(error));
    }
  };

  // Calculate the vertical spacing for a round based on the number of matches
  const getMatchSpacing = (roundNumber: number) => {
    // Each round has half the matches of the previous round
    const matchesInRound = bracketSize / Math.pow(2, roundNumber);
    const totalHeight = bracketSize / 2 * MATCH_HEIGHT + (bracketSize / 2 - 1) * 16;
    const spacePerMatch = totalHeight / matchesInRound;
    return spacePerMatch - MATCH_HEIGHT;
  };

  if (loading) {
    return <div className="text-center py-4 text-gray-500">{t('common.loading')}</div>;
  }

  if (!isDrawable) {
    return (
      <div className="py-8 text-center text-gray-500">{t('brackets.notDrawable')}</div>
    );
  }

  return (
    <div className="overflow-x-auto" ref={containerRef}>
      <div className="flex min-w-max py-4">
        {Array.from({ length: numRounds }, (_, i) => i + 1).map((roundNumber) => {
          const matches = getMatchesByRound(roundNumber);
          const roundName = getRoundName(roundNumber, numRounds);
          const matchSpacing = getMatchSpacing(roundNumber);
          const isFirstRound = roundNumber === 1;
          const isLastRound = roundNumber === numRounds;

          return (
            <div key={roundNumber} className="flex">
              {/* Round column */}
              <div className="flex flex-col">
                <div className="text-sm font-medium text-gray-500 mb-4 text-center w-48">
                  {roundName}
                </div>
                <div
                  className="flex flex-col"
                  style={{
                    gap: `${matchSpacing}px`,
                    paddingTop: isFirstRound ? 0 : `${matchSpacing / 2}px`,
                  }}
                >
                  {matches.map((match) => {
                    const isOpenable = canOpenMatch(match);
                    const hasClash = clashing.has(match.id);
                    return (
                      <Card
                        key={match.id}
                        className={`w-48 transition-shadow ${
                          isOpenable
                            ? 'cursor-pointer hover:shadow-md hover:border-primary-300'
                            : 'cursor-default'
                        } ${hasClash ? 'border-red-400' : ''}`}
                        onClick={() => handleMatchClick(match)}
                        style={{ height: `${MATCH_HEIGHT}px` }}
                      >
                        <CardContent className="p-2 h-full flex flex-col justify-between">
                          {/* Court number */}
                          <div
                            className={`text-xs text-center ${
                              hasClash ? 'font-medium text-red-600' : 'text-gray-400'
                            }`}
                            title={hasClash ? t('brackets.courtClash') : undefined}
                          >
                            {match.courtNumber ? `${t('pairing.court')} ${match.courtNumber}` : ''}
                          </div>

                          {/* Team 1 */}
                          <div
                            className={`flex justify-between items-center ${
                              match.winnerId === match.team1Id ? 'font-bold' : ''
                            }`}
                          >
                            <span className="truncate text-sm">
                              {match.isBye && !match.team1Id ? 'BYE' : <TeamLabel team={getTeam(match.team1Id)} />}
                            </span>
                            <span className="text-sm ml-2">
                              {match.team1Score !== null ? match.team1Score : '-'}
                            </span>
                          </div>

                          <div className="border-t border-gray-100" />

                          {/* Team 2 */}
                          <div
                            className={`flex justify-between items-center ${
                              match.winnerId === match.team2Id ? 'font-bold' : ''
                            }`}
                          >
                            <span className="truncate text-sm">
                              {match.isBye ? 'BYE' : <TeamLabel team={getTeam(match.team2Id)} />}
                            </span>
                            <span className="text-sm ml-2">
                              {match.isBye ? '7' : match.team2Score !== null ? match.team2Score : '-'}
                            </span>
                          </div>
                        </CardContent>
                      </Card>
                    );
                  })}
                </div>
              </div>

              {/* Connectors to next round */}
              {!isLastRound && (
                <div className="flex flex-col" style={{ width: `${CONNECTOR_WIDTH}px` }}>
                  <div className="mb-4 h-5" /> {/* Spacer for header */}
                  <div
                    className="flex flex-col"
                    style={{
                      gap: `${matchSpacing}px`,
                      paddingTop: isFirstRound ? 0 : `${matchSpacing / 2}px`,
                    }}
                  >
                    {matches.map((match, matchIndex) => {
                      // Calculate if this match connects to upper or lower part of next match
                      const isUpperMatch = matchIndex % 2 === 0;
                      const pairSpacing = MATCH_HEIGHT + matchSpacing;

                      return (
                        <div
                          key={`connector-${match.id}`}
                          className="relative"
                          style={{ height: `${MATCH_HEIGHT}px` }}
                        >
                          {/* Horizontal line from match */}
                          <div
                            className="absolute bg-gray-300"
                            style={{
                              left: 0,
                              top: '50%',
                              width: `${CONNECTOR_WIDTH / 2}px`,
                              height: '2px',
                              transform: 'translateY(-50%)',
                            }}
                          />
                          {/* Vertical line connecting pair */}
                          {isUpperMatch && (
                            <div
                              className="absolute bg-gray-300"
                              style={{
                                left: `${CONNECTOR_WIDTH / 2 - 1}px`,
                                top: '50%',
                                width: '2px',
                                height: `${pairSpacing}px`,
                              }}
                            />
                          )}
                          {/* Horizontal line to next match */}
                          <div
                            className="absolute bg-gray-300"
                            style={{
                              left: `${CONNECTOR_WIDTH / 2}px`,
                              top: isUpperMatch ? `${pairSpacing / 2 + MATCH_HEIGHT / 2}px` : `${-pairSpacing / 2 + MATCH_HEIGHT / 2}px`,
                              width: `${CONNECTOR_WIDTH / 2}px`,
                              height: '2px',
                            }}
                          />
                        </div>
                      );
                    })}
                  </div>
                </div>
              )}
            </div>
          );
        })}

        {/* Winner column */}
        <div className="flex flex-col">
          <div className="text-sm font-medium text-gray-500 mb-4 text-center w-48">
            {t('brackets.winner')}
          </div>
          <div
            className="flex flex-col justify-center"
            style={{
              paddingTop: `${getMatchSpacing(numRounds) / 2}px`,
            }}
          >
            {bracketMatches
              .filter((m) => m.roundNumber === numRounds && m.winnerId)
              .map((finalMatch) => (
                <div key={`winner-${finalMatch.id}`} className="flex items-center">
                  {/* Connector line from final */}
                  <div
                    className="bg-gray-300"
                    style={{
                      width: `${CONNECTOR_WIDTH / 2}px`,
                      height: '2px',
                    }}
                  />
                  <Card className="w-48 bg-green-50 border-green-200">
                    <CardContent className="p-3 text-center">
                      <div className="text-lg font-bold text-green-700">
                        <TeamLabel team={getTeam(finalMatch.winnerId)} />
                      </div>
                    </CardContent>
                  </Card>
                </div>
              ))}
          </div>
        </div>
      </div>

      {/* Score Entry Dialog */}
      <Dialog open={scoreDialogOpen} onOpenChange={setScoreDialogOpen}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>
              {selectedMatch && !canEditMatch(selectedMatch)
                ? t('brackets.editCourt')
                : t('brackets.enterScore')}
            </DialogTitle>
            {/* A card only opens when something on it is live, so the dialog
                never shows up with everything greyed out. */}
          </DialogHeader>
          {selectedMatch && (
            <div className="space-y-4">
              {canEditCourt(selectedMatch) ? (
                <div className="flex items-center gap-2">
                  {/* Three digits is more courts than any venue has. */}
                  <Input
                    label={t('pairing.court')}
                    type="number"
                    min={1}
                    max={999}
                    value={court}
                    onChange={(e) => setCourt(e.target.value)}
                    className={`w-14 px-1 text-center ${
                      clashing.has(selectedMatch.id) ? 'border-red-500 focus:border-red-500' : ''
                    }`}
                  />
                  {clashing.has(selectedMatch.id) && (
                    <span className="pt-6 text-xs text-red-600">{t('brackets.courtClash')}</span>
                  )}
                </div>
              ) : (
                selectedMatch.courtNumber && (
                  <div className="text-sm text-gray-500">
                    {t('pairing.court')} {selectedMatch.courtNumber}
                  </div>
                )
              )}

              {canEditMatch(selectedMatch) ? (
                <div className="flex items-center gap-4">
                  <div className="flex-1">
                    <label className="text-sm font-medium text-gray-700">
                      <TeamLabel team={getTeam(selectedMatch.team1Id)} />
                    </label>
                    <Input
                      type="number"
                      min={0}
                      max={13}
                      value={team1Score}
                      onChange={(e) => setTeam1Score(e.target.value)}
                      className="mt-1"
                    />
                  </div>
                  <span className="text-gray-400 pt-6">{t('pairing.vs')}</span>
                  <div className="flex-1">
                    <label className="text-sm font-medium text-gray-700">
                      <TeamLabel team={getTeam(selectedMatch.team2Id)} />
                    </label>
                    <Input
                      type="number"
                      min={0}
                      max={13}
                      value={team2Score}
                      onChange={(e) => setTeam2Score(e.target.value)}
                      className="mt-1"
                    />
                  </div>
                </div>
              ) : (
                /* No score to enter yet - its feeders have not finished - but
                   the court can still be moved. */
                <p className="text-sm text-gray-500">{t('brackets.courtOnly')}</p>
              )}

              {saveError && (
                <div className="rounded-md bg-red-50 p-3 text-sm text-red-700">{saveError}</div>
              )}
            </div>
          )}
          <DialogFooter>
            <Button variant="secondary" onClick={() => setScoreDialogOpen(false)}>
              {t('common.cancel')}
            </Button>
            <Button onClick={handleSave}>{t('common.save')}</Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </div>
  );
}
