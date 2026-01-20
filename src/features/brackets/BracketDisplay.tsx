import { useEffect, useState, useRef } from 'react';
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
} from '../../components/ui';
import type { BracketMatch } from '../../types';
import { formatTeamName } from '../../lib/utils';

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
  } = useTournamentStore();

  const [selectedMatch, setSelectedMatch] = useState<BracketMatch | null>(null);
  const [scoreDialogOpen, setScoreDialogOpen] = useState(false);
  const [team1Score, setTeam1Score] = useState('');
  const [team2Score, setTeam2Score] = useState('');
  const containerRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    fetchMatchesForBracket(bracketId);
  }, [bracketId, fetchMatchesForBracket]);

  const numRounds = Math.log2(bracketSize);

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

  const getTeamName = (teamId: string | null | undefined) => {
    if (!teamId) return 'TBD';
    const team = teams.find((t) => t.id === teamId);
    return formatTeamName(team?.captain);
  };

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

  const handleMatchClick = (match: BracketMatch) => {
    if (!canEditMatch(match)) return;
    setSelectedMatch(match);
    setTeam1Score(match.team1Score?.toString() || '');
    setTeam2Score(match.team2Score?.toString() || '');
    setScoreDialogOpen(true);
  };

  const handleSaveScore = async () => {
    if (!selectedMatch) return;

    const s1 = parseInt(team1Score);
    const s2 = parseInt(team2Score);

    if (isNaN(s1) || isNaN(s2)) {
      return;
    }

    if (s1 === s2) {
      alert('Scores cannot be tied. One team must win.');
      return;
    }

    try {
      await updateMatchScore(selectedMatch.id, s1, s2);
      await fetchMatchesForBracket(bracketId);
      setScoreDialogOpen(false);
      setSelectedMatch(null);
    } catch (error) {
      console.error('Failed to save score:', error);
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
                    const isEditable = canEditMatch(match);
                    return (
                      <Card
                        key={match.id}
                        className={`w-48 transition-shadow ${
                          isEditable
                            ? 'cursor-pointer hover:shadow-md hover:border-primary-300'
                            : 'cursor-default'
                        }`}
                        onClick={() => handleMatchClick(match)}
                        style={{ height: `${MATCH_HEIGHT}px` }}
                      >
                        <CardContent className="p-2 h-full flex flex-col justify-between">
                          {/* Court number */}
                          <div className="text-xs text-gray-400 text-center">
                            {match.courtNumber ? `Court ${match.courtNumber}` : ''}
                          </div>

                          {/* Team 1 */}
                          <div
                            className={`flex justify-between items-center ${
                              match.winnerId === match.team1Id ? 'font-bold' : ''
                            }`}
                          >
                            <span className="truncate text-sm">
                              {match.isBye && !match.team1Id ? 'BYE' : getTeamName(match.team1Id)}
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
                              {match.isBye ? 'BYE' : getTeamName(match.team2Id)}
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
                        {getTeamName(finalMatch.winnerId)}
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
            <DialogTitle>{t('brackets.enterScore')}</DialogTitle>
          </DialogHeader>
          {selectedMatch && (
            <div className="space-y-4">
              {selectedMatch.courtNumber && (
                <div className="text-sm text-gray-500 text-center">
                  Court {selectedMatch.courtNumber}
                </div>
              )}
              <div className="flex items-center gap-4">
                <div className="flex-1">
                  <label className="text-sm font-medium text-gray-700">
                    {getTeamName(selectedMatch.team1Id)}
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
                    {getTeamName(selectedMatch.team2Id)}
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
            </div>
          )}
          <DialogFooter>
            <Button variant="secondary" onClick={() => setScoreDialogOpen(false)}>
              {t('common.cancel')}
            </Button>
            <Button onClick={handleSaveScore}>{t('common.save')}</Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </div>
  );
}
