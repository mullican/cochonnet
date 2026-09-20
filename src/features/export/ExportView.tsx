import { useEffect, useState, useMemo } from 'react';
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
import type { BracketMatch, GameWithTeams, Team } from '../../types';

interface ExportViewProps {
  tournamentId: string;
}

/** A document the operator can ask for, and the name it is filed or spooled under. */
interface PdfJob {
  build: () => Parameters<typeof pdf>[0] | Promise<Parameters<typeof pdf>[0]>;
  filename: string;
}

export function ExportView({ tournamentId: _tournamentId }: ExportViewProps) {
  const { t } = useTranslation();
  const [busy, setBusy] = useState(false);
  const {
    currentTournament,
    teams,
    qualifyingRounds,
    standings,
    brackets,
  } = useTournamentStore();

  const [error, setError] = useState<string | null>(null);

  // What this build can actually do. iPadOS has no "save it where you like", so
  // it offers printing in place of export; macOS offers both.
  const [canExport, setCanExport] = useState(true);
  const [canPrint, setCanPrint] = useState(false);

  useEffect(() => {
    invoke<boolean>('file_export_available').then(setCanExport).catch(() => setCanExport(true));
    invoke<boolean>('printing_available').then(setCanPrint).catch(() => setCanPrint(false));
  }, []);

  // Get PDF translations from current language
  const pdfTranslations: PDFTranslations = useMemo(() => ({
    round: t('pdf.round'),
    court: t('pdf.court'),
    vs: t('pdf.vs'),
    courtAbbrev: t('pdf.courtAbbrev'),
    courtLegend: t('pdf.courtLegend'),
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
    nothingToShow: t('pdf.nothingToShow'),
    withdrawn: t('pdf.withdrawn'),
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

  // The store only ever holds the matches of the one bracket being viewed, so
  // an export that reads it straight gets a filled first page and blank ones
  // after. Every bracket has to be loaded here.
  const fetchAllBracketMatches = async (): Promise<BracketMatch[]> => {
    const allMatches: BracketMatch[] = [];
    for (const bracket of brackets) {
      const bracketMatchList = await invoke<BracketMatch[]>('get_matches_for_bracket', {
        bracketId: bracket.id,
      });
      allMatches.push(...bracketMatchList);
    }
    return allMatches;
  };

  const renderBytes = async (job: PdfJob): Promise<Uint8Array> => {
    const blob = await pdf(await job.build()).toBlob();
    return new Uint8Array(await blob.arrayBuffer());
  };

  /** Ask where the PDF should go, then write it there. Desktop only. */
  const exportPdf = async (job: PdfJob) => {
    setBusy(true);
    setError(null);
    try {
      const bytes = await renderBytes(job);
      const filePath = await save({
        defaultPath: job.filename,
        filters: [{ name: 'PDF', extensions: ['pdf'] }],
      });
      if (filePath) {
        await writeFileOverwrite(filePath, bytes);
      }
    } catch (err) {
      console.error('Failed to export PDF:', err);
      setError(t('export.exportFailed', { error: err instanceof Error ? err.message : String(err) }));
    } finally {
      setBusy(false);
    }
  };

  /**
   * Hand the PDF to the system print UI: the standard print panel on macOS,
   * the AirPrint sheet on iPadOS. Both let the operator pick the printer and a
   * page range, so this resolves once the panel is up, not once it has printed
   * - a rejected promise means the panel never opened.
   */
  const printPdf = async (job: PdfJob) => {
    setBusy(true);
    setError(null);
    try {
      const bytes = await renderBytes(job);
      // Sent as a plain number array. These documents top out around 150 KB and
      // this serialises the same way on every platform.
      await invoke('print_pdf', { fileName: job.filename, data: Array.from(bytes) });
    } catch (err) {
      console.error('Failed to print PDF:', err);
      setError(t('export.printFailed', { error: err instanceof Error ? err.message : String(err) }));
    } finally {
      setBusy(false);
    }
  };

  const courtAssignmentsJob = (): PdfJob | null =>
    currentTournament && {
      filename: `${currentTournament.name}_court_assignments.pdf`,
      build: async () => (
        <CourtAssignmentsPDF
          tournament={currentTournament}
          teams={teams}
          rounds={qualifyingRounds}
          games={await fetchAllGames()}
          sitouts={await fetchAllSitouts()}
          translations={pdfTranslations}
        />
      ),
    };

  const standingsJob = (): PdfJob | null =>
    currentTournament && {
      filename: `${currentTournament.name}_standings.pdf`,
      build: () => (
        <StandingsPDF
          tournament={currentTournament}
          teams={teams}
          standings={standings}
          translations={pdfTranslations}
        />
      ),
    };

  const bracketsJob = (): PdfJob | null =>
    currentTournament && {
      filename: `${currentTournament.name}_brackets.pdf`,
      build: async () => (
        <BracketPDF
          tournament={currentTournament}
          teams={teams}
          brackets={brackets}
          matches={await fetchAllBracketMatches()}
          translations={pdfTranslations}
        />
      ),
    };

  const handleExportFullBackup = async () => {
    if (!currentTournament) return;

    setBusy(true);
    setError(null);
    try {
      // Assembled in the backend so it covers every table a tournament touches.
      // The version built here from store state silently dropped umpires,
      // pairing and court history, and the panache tables.
      const jsonString = await invoke<string>('export_tournament_backup', {
        tournamentId: currentTournament.id,
      });
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
      setError(t('export.exportFailed', { error: err instanceof Error ? err.message : String(err) }));
    } finally {
      setBusy(false);
    }
  };

  /** One document, offered through whichever actions this platform supports. */
  const PdfCard = ({
    title,
    description,
    job,
    disabled,
  }: {
    title: string;
    description: string;
    job: () => PdfJob | null;
    disabled: boolean;
  }) => (
    <Card>
      <CardHeader>
        <CardTitle>{title}</CardTitle>
      </CardHeader>
      <CardContent>
        <p className="text-sm text-gray-500 mb-4">{description}</p>
        <div className="flex flex-wrap gap-2">
          {canExport && (
            <Button
              onClick={() => {
                const pdfJob = job();
                if (pdfJob) exportPdf(pdfJob);
              }}
              disabled={disabled || busy}
            >
              {busy ? t('common.loading') : t('export.generatePDF')}
            </Button>
          )}
          {canPrint && (
            <Button
              variant="secondary"
              onClick={() => {
                const pdfJob = job();
                if (pdfJob) printPdf(pdfJob);
              }}
              disabled={disabled || busy}
            >
              {busy ? t('common.loading') : t('export.print')}
            </Button>
          )}
        </div>
      </CardContent>
    </Card>
  );

  return (
    <div className="space-y-6">
      <h2 className="text-lg font-semibold text-gray-900">{t('export.title')}</h2>

      {error && (
        <div className="rounded-md bg-red-50 p-4 text-sm text-red-700">
          {error}
        </div>
      )}

      <div className="grid gap-4 sm:grid-cols-2">
        <PdfCard
          title={t('export.courtAssignments')}
          description={t('export.courtAssignmentsDescription')}
          job={courtAssignmentsJob}
          disabled={qualifyingRounds.length === 0}
        />

        <PdfCard
          title={t('export.standings')}
          description={t('export.standingsDescription')}
          job={standingsJob}
          disabled={standings.length === 0}
        />

        {currentTournament?.pairingMethod !== 'panache' && (
          <PdfCard
            title={t('export.brackets')}
            description={t('export.bracketsDescription')}
            job={bracketsJob}
            disabled={brackets.length === 0}
          />
        )}

        {/* A backup is a file, not a document: nothing to print, and nowhere to
            put it on a platform with no save dialog. */}
        {canExport && (
          <Card>
            <CardHeader>
              <CardTitle>{t('export.fullBackup')}</CardTitle>
            </CardHeader>
            <CardContent>
              <p className="text-sm text-gray-500 mb-4">
                {t('export.fullBackupDescription')}
              </p>
              <Button onClick={handleExportFullBackup} disabled={busy}>
                {busy ? t('common.loading') : t('export.downloadJSON')}
              </Button>
            </CardContent>
          </Card>
        )}
      </div>
    </div>
  );
}
