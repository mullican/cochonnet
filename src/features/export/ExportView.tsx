import { useState, useMemo } from 'react';
import { useTranslation } from 'react-i18next';
import { pdf } from '@react-pdf/renderer';
import { save } from '@tauri-apps/plugin-dialog';
import { writeFile, remove, exists } from '@tauri-apps/plugin-fs';
import { invoke } from '@tauri-apps/api/core';
import { useTournamentStore } from '../../stores/tournamentStore';
import { Button, Card, CardContent, CardHeader, CardTitle } from '../../components/ui';
import { CourtAssignmentsPDF } from './CourtAssignmentsPDF';
import { StandingsPDF } from './StandingsPDF';
import { BracketPDF } from './BracketPDF';
import type { PDFTranslations } from './pdfTranslations';
import type { GameWithTeams, Team } from '../../types';

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

  // Get PDF translations from current language
  const pdfTranslations: PDFTranslations = useMemo(() => ({
    round: t('pdf.round'),
    court: t('pdf.court'),
    vs: t('pdf.vs'),
    champion: t('pdf.champion'),
    winner: t('pdf.winner'),
    tbd: t('pdf.tbd'),
    bye: t('pdf.bye'),
    final: t('pdf.final'),
    semiFinal: t('pdf.semiFinal'),
    quarterFinal: t('pdf.quarterFinal'),
    rank: t('pdf.rank'),
    team: t('pdf.team'),
    wins: t('pdf.wins'),
    losses: t('pdf.losses'),
    pointsFor: t('pdf.pointsFor'),
    pointsAgainst: t('pdf.pointsAgainst'),
    differential: t('pdf.differential'),
    concours: t('pdf.concours'),
    consolante: t('pdf.consolante'),
    standings: t('pdf.standings'),
    standingsAsOf: t('pdf.standingsAsOf'),
    topTeamsAdvance: t('pdf.topTeamsAdvance'),
    legendWins: t('pdf.legendWins'),
    legendLosses: t('pdf.legendLosses'),
    legendPointsFor: t('pdf.legendPointsFor'),
    legendPointsAgainst: t('pdf.legendPointsAgainst'),
    legendDifferential: t('pdf.legendDifferential'),
    legendBuchholz: t('pdf.legendBuchholz'),
    legendFineBuchholz: t('pdf.legendFineBuchholz'),
    legendPointQuotient: t('pdf.legendPointQuotient'),
    tiebreaker: t('pdf.tiebreaker'),
    tiebreakerSwiss: t('pdf.tiebreakerSwiss'),
    tiebreakerPointQuotient: t('pdf.tiebreakerPointQuotient'),
    tiebreakerPanache: t('pdf.tiebreakerPanache'),
    courtAssignments: t('pdf.courtAssignments'),
    buchholz: t('pdf.buchholz'),
    fineBuchholz: t('pdf.fineBuchholz'),
    pointQuotient: t('pdf.pointQuotient'),
    player: t('pdf.player'),
    sittingOut: t('pdf.sittingOut'),
  }), [t]);

  // Helper to write file, removing existing file first if needed
  const writeFileOverwrite = async (path: string, data: Uint8Array) => {
    try {
      if (await exists(path)) {
        await remove(path);
      }
    } catch {
      // Ignore errors checking/removing - file might not exist
    }
    await writeFile(path, data);
  };

  // Fetch all games for all rounds
  const fetchAllGames = async (): Promise<GameWithTeams[]> => {
    const allGames: GameWithTeams[] = [];
    for (const round of qualifyingRounds) {
      const games = await invoke<GameWithTeams[]>('get_games_for_round', { roundId: round.id });
      allGames.push(...games);
    }
    return allGames;
  };

  // Panache rests surplus players each round; they belong on the court sheet even
  // though they have no game. Other formats return nothing here.
  const fetchAllSitouts = async (): Promise<Record<string, Team[]>> => {
    if (currentTournament?.pairingMethod !== 'panache') return {};
    const byRound: Record<string, Team[]> = {};
    for (const round of qualifyingRounds) {
      byRound[round.id] = await invoke<Team[]>('get_sitouts_for_round', { roundId: round.id });
    }
    return byRound;
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
        await writeFileOverwrite(filePath, uint8Array);
      }
    } catch (err) {
      console.error('Failed to export PDF:', err);
      setError(`Export failed: ${err instanceof Error ? err.message : String(err)}`);
    } finally {
      setExporting(false);
    }
  };

  const handleExportCourtAssignments = async () => {
    if (!currentTournament) return;

    setExporting(true);
    setError(null);
    try {
      // Fetch all games for all rounds
      const allGames = await fetchAllGames();
      const allSitouts = await fetchAllSitouts();

      const doc = (
        <CourtAssignmentsPDF
          tournament={currentTournament}
          teams={teams}
          rounds={qualifyingRounds}
          games={allGames}
          sitouts={allSitouts}
          translations={pdfTranslations}
        />
      );

      const blob = await pdf(doc).toBlob();
      const arrayBuffer = await blob.arrayBuffer();
      const uint8Array = new Uint8Array(arrayBuffer);

      const filePath = await save({
        defaultPath: `${currentTournament.name}_court_assignments.pdf`,
        filters: [{ name: 'PDF', extensions: ['pdf'] }],
      });

      if (filePath) {
        await writeFileOverwrite(filePath, uint8Array);
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
        translations={pdfTranslations}
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
        translations={pdfTranslations}
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
        await writeFileOverwrite(filePath, uint8Array);
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
            <CardTitle>{t('export.courtAssignments')}</CardTitle>
          </CardHeader>
          <CardContent>
            <p className="text-sm text-gray-500 mb-4">
              {t('export.courtAssignmentsDescription')}
            </p>
            <Button
              onClick={handleExportCourtAssignments}
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
              {t('export.standingsDescription')}
            </p>
            <Button
              onClick={handleExportStandings}
              disabled={standings.length === 0 || exporting}
            >
              {exporting ? t('common.loading') : t('export.generatePDF')}
            </Button>
          </CardContent>
        </Card>

        {currentTournament?.pairingMethod !== 'panache' && (
          <Card>
            <CardHeader>
              <CardTitle>{t('export.brackets')}</CardTitle>
            </CardHeader>
            <CardContent>
              <p className="text-sm text-gray-500 mb-4">{t('export.bracketsDescription')}</p>
              <Button
                onClick={handleExportBrackets}
                disabled={brackets.length === 0 || exporting}
              >
                {exporting ? t('common.loading') : t('export.generatePDF')}
              </Button>
            </CardContent>
          </Card>
        )}

        <Card>
          <CardHeader>
            <CardTitle>{t('export.fullBackup')}</CardTitle>
          </CardHeader>
          <CardContent>
            <p className="text-sm text-gray-500 mb-4">
              {t('export.fullBackupDescription')}
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
