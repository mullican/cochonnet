import { useEffect, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { invoke } from '@tauri-apps/api/core';
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
import { BracketDisplay } from './BracketDisplay';

interface BracketViewProps {
  tournamentId: string;
}

export function BracketView({ tournamentId }: BracketViewProps) {
  const { t } = useTranslation();
  const {
    brackets,
    qualifyingRounds,
    standings,
    loading,
    fetchBrackets,
    generateBrackets,
    fetchStandings,
  } = useTournamentStore();

  const [selectedBracketId, setSelectedBracketId] = useState<string | null>(null);
  const [regenerateDialogOpen, setRegenerateDialogOpen] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    fetchBrackets(tournamentId);
    fetchStandings(tournamentId);
  }, [tournamentId, fetchBrackets, fetchStandings]);

  useEffect(() => {
    if (brackets.length > 0 && !selectedBracketId) {
      setSelectedBracketId(brackets[0].id);
    }
  }, [brackets, selectedBracketId]);

  const handleGenerateBrackets = async () => {
    setError(null);
    try {
      await generateBrackets(tournamentId);
    } catch (err) {
      console.error('Failed to generate brackets:', err);
      setError(String(err));
    }
  };

  const handleRegenerateBrackets = async () => {
    setError(null);
    try {
      // Delete existing brackets first
      await invoke('delete_brackets', { tournamentId });
    } catch (err) {
      console.error('Failed to delete brackets:', err);
      setError(`Delete failed: ${String(err)}`);
      return;
    }

    try {
      await generateBrackets(tournamentId);
      setRegenerateDialogOpen(false);
      setSelectedBracketId(null);
    } catch (err) {
      console.error('Failed to generate brackets:', err);
      setError(`Generate failed: ${String(err)}`);
    }
  };

  const allRoundsComplete = qualifyingRounds.length > 0 && qualifyingRounds.every((r) => r.isComplete);
  const hasStandings = standings.length > 0;
  const canGenerateBrackets = allRoundsComplete && hasStandings && brackets.length === 0;

  const mainBrackets = brackets.filter((b) => !b.isConsolante);
  const consolanteBrackets = brackets.filter((b) => b.isConsolante);

  return (
    <div className="space-y-6">
      <div className="flex items-center justify-between">
        <h2 className="text-lg font-semibold text-gray-900">{t('brackets.title')}</h2>
        <div className="flex gap-2">
          {canGenerateBrackets && (
            <Button onClick={handleGenerateBrackets} disabled={loading}>
              {t('brackets.generate')}
            </Button>
          )}
          {brackets.length > 0 && (
            <Button variant="secondary" onClick={() => setRegenerateDialogOpen(true)} disabled={loading}>
              {t('brackets.regenerate')}
            </Button>
          )}
        </div>
      </div>

      {error && (
        <Card>
          <CardContent className="py-4 text-center text-red-600">
            {error}
          </CardContent>
        </Card>
      )}

      {brackets.length === 0 && (
        <Card>
          <CardContent className="py-8 text-center text-gray-500">
            {!allRoundsComplete
              ? t('brackets.noBrackets')
              : !hasStandings
              ? 'No standings available. Complete qualifying rounds and ensure standings are calculated.'
              : 'Click "Generate Brackets" to create the elimination brackets.'}
          </CardContent>
        </Card>
      )}

      {brackets.length > 0 && (
        <Tabs value={selectedBracketId || ''}>
          <TabsList>
            {mainBrackets.map((bracket) => (
              <TabsTrigger
                key={bracket.id}
                value={bracket.id}
                onClick={() => setSelectedBracketId(bracket.id)}
              >
                {t('brackets.main')} {bracket.name}
              </TabsTrigger>
            ))}
            {consolanteBrackets.map((bracket) => (
              <TabsTrigger
                key={bracket.id}
                value={bracket.id}
                onClick={() => setSelectedBracketId(bracket.id)}
              >
                {t('brackets.consolante')} {bracket.name}
              </TabsTrigger>
            ))}
          </TabsList>

          {brackets.map((bracket) => (
            <TabsContent key={bracket.id} value={bracket.id} className="mt-4">
              <BracketDisplay
                bracketId={bracket.id}
                bracketSize={bracket.size}
              />
            </TabsContent>
          ))}
        </Tabs>
      )}

      <Dialog open={regenerateDialogOpen} onOpenChange={setRegenerateDialogOpen}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>{t('brackets.regenerate')}</DialogTitle>
            <DialogDescription>
              This will delete all existing brackets and create new ones based on current standings.
              All bracket match results will be lost.
            </DialogDescription>
          </DialogHeader>
          <DialogFooter>
            <Button variant="secondary" onClick={() => setRegenerateDialogOpen(false)}>
              {t('common.cancel')}
            </Button>
            <Button variant="danger" onClick={handleRegenerateBrackets}>
              {t('brackets.regenerate')}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </div>
  );
}
