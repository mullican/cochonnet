import { useState } from 'react';
import { useTranslation } from 'react-i18next';
import { pdf } from '@react-pdf/renderer';
import { save } from '@tauri-apps/plugin-dialog';
import { writeFile } from '@tauri-apps/plugin-fs';
import { invoke } from '@tauri-apps/api/core';
import { useTournamentStore } from '../../stores/tournamentStore';
import { Button, Card, CardContent, CardHeader, CardTitle } from '../../components/ui';
import { ScoreSheetPDF } from './ScoreSheetPDF';
import { StandingsPDF } from './StandingsPDF';
import { BracketPDF } from './BracketPDF';
import type { QualifyingGame } from '../../types';

interface ExportViewProps {
  tournamentId: string;
}

export function ExportView({ tournamentId: _tournamentId }: ExportViewProps) {
  const { t } = useTranslation();
  const [exporting, setExporting] = useState(false);
  const {
    currentTournament,
    teams,
    qualifyingRounds,
    standings,
    brackets,
    bracketMatches,
  } = useTournamentStore();

  const [error, setError] = useState<string | null>(null);

  // Fetch all games for all rounds
  const fetchAllGames = async (): Promise<QualifyingGame[]> => {
    const allGames: QualifyingGame[] = [];
    for (const round of qualifyingRounds) {
      const games = await invoke<QualifyingGame[]>('get_games_for_round', { roundId: round.id });
      allGames.push(...games);
    }
    return allGames;
  };

  const downloadPDF = async (pdfDocument: Parameters<typeof pdf>[0], defaultFilename: string) => {
    setExporting(true);
    setError(null);
    try {
      const blob = await pdf(pdfDocument).toBlob();
      const arrayBuffer = await blob.arrayBuffer();
      const uint8Array = new Uint8Array(arrayBuffer);

      const filePath = await save({
        defaultPath: defaultFilename,
        filters: [{ name: 'PDF', extensions: ['pdf'] }],
      });

      if (filePath) {
        await writeFile(filePath, uint8Array);
      }
    } catch (err) {
      console.error('Failed to export PDF:', err);
      setError(`Export failed: ${err instanceof Error ? err.message : String(err)}`);
    } finally {
      setExporting(false);
    }
  };

  const handleExportScoreSheets = async () => {
    if (!currentTournament) return;

    setExporting(true);
    setError(null);
    try {
      // Fetch all games for all rounds
      const allGames = await fetchAllGames();

      const doc = (
        <ScoreSheetPDF
          tournament={currentTournament}
          teams={teams}
          rounds={qualifyingRounds}
          games={allGames}
        />
      );

      const blob = await pdf(doc).toBlob();
      const arrayBuffer = await blob.arrayBuffer();
      const uint8Array = new Uint8Array(arrayBuffer);

      const filePath = await save({
        defaultPath: `${currentTournament.name}_score_sheets.pdf`,
        filters: [{ name: 'PDF', extensions: ['pdf'] }],
      });

      if (filePath) {
        await writeFile(filePath, uint8Array);
      }
    } catch (err) {
      console.error('Failed to export PDF:', err);
      setError(`Export failed: ${err instanceof Error ? err.message : String(err)}`);
    } finally {
      setExporting(false);
    }
  };

  const handleExportStandings = async () => {
    if (!currentTournament) return;

    const doc = (
      <StandingsPDF
        tournament={currentTournament}
        teams={teams}
        standings={standings}
      />
    );
    await downloadPDF(doc, `${currentTournament.name}_standings.pdf`);
  };

  const handleExportBrackets = async () => {
    if (!currentTournament) return;

    const doc = (
      <BracketPDF
        tournament={currentTournament}
        teams={teams}
        brackets={brackets}
        matches={bracketMatches}
      />
    );
    await downloadPDF(doc, `${currentTournament.name}_brackets.pdf`);
  };

  const handleExportFullBackup = async () => {
    if (!currentTournament) return;

    setExporting(true);
    setError(null);
    try {
      // Fetch all games for backup
      const allGames = await fetchAllGames();

      const backup = {
        tournament: currentTournament,
        teams,
        qualifyingRounds,
        qualifyingGames: allGames,
        standings,
        brackets,
        bracketMatches,
        exportedAt: new Date().toISOString(),
      };

      const jsonString = JSON.stringify(backup, null, 2);
      const encoder = new TextEncoder();
      const uint8Array = encoder.encode(jsonString);

      const filePath = await save({
        defaultPath: `${currentTournament.name}_backup.json`,
        filters: [{ name: 'JSON', extensions: ['json'] }],
      });

      if (filePath) {
        await writeFile(filePath, uint8Array);
      }
    } catch (err) {
      console.error('Failed to export backup:', err);
      setError(`Export failed: ${err instanceof Error ? err.message : String(err)}`);
    } finally {
      setExporting(false);
    }
  };

  return (
    <div className="space-y-6">
      <h2 className="text-lg font-semibold text-gray-900">{t('export.title')}</h2>

      {error && (
        <div className="rounded-md bg-red-50 p-4 text-sm text-red-700">
          {error}
        </div>
      )}

      <div className="grid gap-4 sm:grid-cols-2">
        <Card>
          <CardHeader>
            <CardTitle>{t('export.scoreSheets')}</CardTitle>
          </CardHeader>
          <CardContent>
            <p className="text-sm text-gray-500 mb-4">
              Export score sheets for all qualifying rounds with court assignments and team pairings.
            </p>
            <Button
              onClick={handleExportScoreSheets}
              disabled={qualifyingRounds.length === 0 || exporting}
            >
              {exporting ? t('common.loading') : t('export.generatePDF')}
            </Button>
          </CardContent>
        </Card>

        <Card>
          <CardHeader>
            <CardTitle>{t('export.standings')}</CardTitle>
          </CardHeader>
          <CardContent>
            <p className="text-sm text-gray-500 mb-4">
              Export current standings with wins, points, and rankings.
            </p>
            <Button
              onClick={handleExportStandings}
              disabled={standings.length === 0 || exporting}
            >
              {exporting ? t('common.loading') : t('export.generatePDF')}
            </Button>
          </CardContent>
        </Card>

        <Card>
          <CardHeader>
            <CardTitle>{t('export.brackets')}</CardTitle>
          </CardHeader>
          <CardContent>
            <p className="text-sm text-gray-500 mb-4">
              Export elimination bracket(s) with seeding and results.
            </p>
            <Button
              onClick={handleExportBrackets}
              disabled={brackets.length === 0 || exporting}
            >
              {exporting ? t('common.loading') : t('export.generatePDF')}
            </Button>
          </CardContent>
        </Card>

        <Card>
          <CardHeader>
            <CardTitle>{t('export.fullBackup')}</CardTitle>
          </CardHeader>
          <CardContent>
            <p className="text-sm text-gray-500 mb-4">
              Download a complete JSON backup of the tournament data.
            </p>
            <Button onClick={handleExportFullBackup} disabled={exporting}>
              {exporting ? t('common.loading') : t('export.downloadJSON')}
            </Button>
          </CardContent>
        </Card>
      </div>
    </div>
  );
}
