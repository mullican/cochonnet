import { useEffect, useState } from 'react';
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

interface BracketDisplayProps {
  bracketId: string;
  bracketSize: number;
}

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

  useEffect(() => {
    fetchMatchesForBracket(bracketId);
  }, [bracketId, fetchMatchesForBracket]);

  const numRounds = Math.log2(bracketSize);

  const getMatchesByRound = (roundNumber: number) => {
    return bracketMatches.filter((m) => m.roundNumber === roundNumber);
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
    return team?.captain || 'Unknown';
  };

  const handleMatchClick = (match: BracketMatch) => {
    if (!match.team1Id || !match.team2Id || match.winnerId) return;
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

  if (loading) {
    return <div className="text-center py-4 text-gray-500">{t('common.loading')}</div>;
  }

  return (
    <div className="overflow-x-auto">
      <div className="flex gap-8 min-w-max py-4">
        {Array.from({ length: numRounds }, (_, i) => i + 1).map((roundNumber) => {
          const matches = getMatchesByRound(roundNumber);
          const roundName = getRoundName(roundNumber, numRounds);

          return (
            <div key={roundNumber} className="flex flex-col">
              <div className="text-sm font-medium text-gray-500 mb-4 text-center">
                {roundName}
              </div>
              <div
                className="flex flex-col justify-around flex-1"
                style={{
                  gap: `${Math.pow(2, roundNumber - 1) * 40}px`,
                }}
              >
                {matches.map((match) => (
                  <Card
                    key={match.id}
                    className={`w-48 cursor-pointer transition-shadow ${
                      match.team1Id && match.team2Id && !match.winnerId
                        ? 'hover:shadow-md hover:border-primary-300'
                        : ''
                    }`}
                    onClick={() => handleMatchClick(match)}
                  >
                    <CardContent className="p-3">
                      <div
                        className={`flex justify-between items-center py-1 ${
                          match.winnerId === match.team1Id ? 'font-bold' : ''
                        }`}
                      >
                        <span className="truncate text-sm">
                          {getTeamName(match.team1Id)}
                        </span>
                        <span className="text-sm ml-2">
                          {match.team1Score !== null ? match.team1Score : '-'}
                        </span>
                      </div>
                      <div className="border-t border-gray-100 my-1" />
                      <div
                        className={`flex justify-between items-center py-1 ${
                          match.winnerId === match.team2Id ? 'font-bold' : ''
                        }`}
                      >
                        <span className="truncate text-sm">
                          {getTeamName(match.team2Id)}
                        </span>
                        <span className="text-sm ml-2">
                          {match.team2Score !== null ? match.team2Score : '-'}
                        </span>
                      </div>
                    </CardContent>
                  </Card>
                ))}
              </div>
            </div>
          );
        })}

        {/* Winner column */}
        <div className="flex flex-col">
          <div className="text-sm font-medium text-gray-500 mb-4 text-center">
            {t('brackets.winner')}
          </div>
          <div className="flex flex-col justify-center flex-1">
            {bracketMatches
              .filter((m) => m.roundNumber === numRounds && m.winnerId)
              .map((finalMatch) => (
                <Card key={`winner-${finalMatch.id}`} className="w-48 bg-green-50 border-green-200">
                  <CardContent className="p-3 text-center">
                    <div className="text-lg font-bold text-green-700">
                      {getTeamName(finalMatch.winnerId)}
                    </div>
                  </CardContent>
                </Card>
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
