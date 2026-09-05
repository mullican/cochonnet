import { useEffect, useState, useRef } from 'react';
import { useTranslation } from 'react-i18next';
import Papa from 'papaparse';
import { useTournamentStore } from '../../stores/tournamentStore';
import {
  Button,
  Card,
  CardContent,
  Table,
  TableHeader,
  TableBody,
  TableRow,
  TableHead,
  TableCell,
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
  DialogFooter,
} from '../../components/ui';
import { TeamForm } from './TeamForm';
import type { Team, TeamFormData, CSVTeamRow } from '../../types';

interface TeamsListProps {
  tournamentId: string;
}

/** Reads the panache roster's champion column, which operators fill in freehand. */
function parseChampionFlag(value: string | undefined): boolean {
  if (!value) return false;
  return ['1', 'true', 'yes', 'y', 'x'].includes(value.trim().toLowerCase());
}

export function TeamsList({ tournamentId }: TeamsListProps) {
  const { t } = useTranslation();
  const fileInputRef = useRef<HTMLInputElement>(null);
  const { teams, qualifyingRounds, currentTournament, loading, fetchTeams, createTeam, updateTeam, deleteTeam, deleteAllTeams, importTeams, fetchQualifyingRounds, setTeamChampion } = useTournamentStore();

  const [addDialogOpen, setAddDialogOpen] = useState(false);
  const [editDialogOpen, setEditDialogOpen] = useState(false);
  const [deleteDialogOpen, setDeleteDialogOpen] = useState(false);
  const [deleteAllDialogOpen, setDeleteAllDialogOpen] = useState(false);
  const [selectedTeam, setSelectedTeam] = useState<Team | null>(null);
  const [importError, setImportError] = useState<string | null>(null);
  const [importSuccess, setImportSuccess] = useState<string | null>(null);
  const [deleteAllError, setDeleteAllError] = useState<string | null>(null);
  const [teamFormError, setTeamFormError] = useState<string | null>(null);

  useEffect(() => {
    fetchTeams(tournamentId);
    fetchQualifyingRounds(tournamentId);
  }, [tournamentId, fetchTeams, fetchQualifyingRounds]);

  const hasRounds = qualifyingRounds.length > 0;
  const canDeleteAllTeams = teams.length > 0 && !hasRounds;

  // Panache registers individuals: one name per row, no partners, plus the
  // champion flag that drives the draw's expert-spreading constraints.
  const isPanache = currentTournament?.pairingMethod === 'panache';
  const showPlayer3 = !isPanache && currentTournament?.format === 'triple';
  const showPlayer2 = !isPanache;

  const nextTeamNumber = teams.length > 0 ? Math.max(...teams.map((t) => t.teamNumber)) + 1 : 1;

  const handleAddTeam = async (data: TeamFormData) => {
    setTeamFormError(null);
    try {
      await createTeam({
        tournamentId,
        teamNumber: parseInt(data.teamNumber, 10),
        captain: data.captain,
        player2: data.player2,
        player3: data.player3 || null,
        region: data.region || null,
        club: data.club || null,
        isChampion: data.isChampion,
      });
      setAddDialogOpen(false);
    } catch (error) {
      console.error('Failed to add team:', error);
      setTeamFormError(String(error));
    }
  };

  const handleEditTeam = async (data: TeamFormData) => {
    if (!selectedTeam) return;
    setTeamFormError(null);
    try {
      await updateTeam(selectedTeam.id, {
        tournamentId,
        teamNumber: parseInt(data.teamNumber, 10),
        captain: data.captain,
        player2: data.player2,
        player3: data.player3 || null,
        region: data.region || null,
        club: data.club || null,
        isChampion: data.isChampion,
      });
      setEditDialogOpen(false);
      setSelectedTeam(null);
    } catch (error) {
      console.error('Failed to update team:', error);
      setTeamFormError(String(error));
    }
  };

  const handleDeleteTeam = async () => {
    if (!selectedTeam) return;
    try {
      await deleteTeam(selectedTeam.id);
      setDeleteDialogOpen(false);
      setSelectedTeam(null);
    } catch (error) {
      console.error('Failed to delete team:', error);
    }
  };

  const handleDeleteAllTeams = async () => {
    setDeleteAllError(null);
    try {
      await deleteAllTeams(tournamentId);
      setDeleteAllDialogOpen(false);
    } catch (error) {
      setDeleteAllError(String(error));
    }
  };

  const handleFileChange = (event: React.ChangeEvent<HTMLInputElement>) => {
    const file = event.target.files?.[0];
    if (!file) return;

    setImportError(null);
    setImportSuccess(null);

    Papa.parse<CSVTeamRow>(file, {
      header: true,
      skipEmptyLines: true,
      complete: async (results) => {
        try {
          const teamsData = results.data.map((row) => ({
            tournamentId,
            teamNumber: row.number ? parseInt(row.number, 10) : undefined,
            // A panache roster names the column "name"; team rosters use "captain".
            captain: (isPanache ? row.name || row.captain : row.captain) || '',
            player2: isPanache ? '' : row.player2 || '',
            player3: isPanache ? null : row.player3 || null,
            region: row.region || null,
            club: row.club || null,
            isChampion: isPanache ? parseChampionFlag(row.champion) : false,
          }));

          // A second player is only required by formats that actually have one.
          // Panache and singles rosters are one name per row.
          const requiresPartner = !isPanache && currentTournament?.format !== 'single';
          const invalidTeams = teamsData.filter(
            (team) => !team.captain || (requiresPartner && !team.player2)
          );
          if (invalidTeams.length > 0) {
            setImportError(
              isPanache
                ? t('teams.importMissingName')
                : requiresPartner
                  ? t('teams.importMissingFields')
                  : t('teams.importMissingName')
            );
            return;
          }

          const count = await importTeams(tournamentId, teamsData);
          setImportSuccess(t('teams.importSuccess', { count }));
        } catch (error) {
          setImportError(t('teams.importError', { error: String(error) }));
        }
      },
      error: (error) => {
        setImportError(t('teams.importError', { error: error.message }));
      },
    });

    // Reset file input
    if (fileInputRef.current) {
      fileInputRef.current.value = '';
    }
  };

  const downloadTemplate = () => {
    // Panache rosters are individuals, so the template is a player list with the
    // champion flag rather than a captain/partner pairing.
    const template = isPanache
      ? 'number,name,region,club,champion\n1,John Doe,North,Club A,\n2,Jane Smith,South,Club B,yes\n'
      : 'number,captain,player2,player3,region,club\n1,John Doe,Jane Smith,Bob Wilson,North,Club A\n';
    const blob = new Blob([template], { type: 'text/csv' });
    const url = URL.createObjectURL(blob);
    const a = document.createElement('a');
    a.href = url;
    a.download = isPanache ? 'players_template.csv' : 'teams_template.csv';
    a.click();
    URL.revokeObjectURL(url);
  };

  return (
    <div className="space-y-4">
      <div className="flex items-center justify-between">
        <h2 className="text-lg font-semibold text-gray-900">
          {isPanache ? t('teams.playersTitle') : t('teams.title')}
        </h2>
        <div className="flex gap-2">
          {canDeleteAllTeams && (
            <Button
              variant="danger"
              size="sm"
              onClick={() => setDeleteAllDialogOpen(true)}
            >
              {t('teams.deleteAll')}
            </Button>
          )}
          <Button variant="secondary" size="sm" onClick={downloadTemplate}>
            {t('teams.downloadTemplate')}
          </Button>
          <input
            ref={fileInputRef}
            type="file"
            accept=".csv"
            className="hidden"
            onChange={handleFileChange}
          />
          <Button
            variant="secondary"
            size="sm"
            onClick={() => fileInputRef.current?.click()}
          >
            {t('teams.importCSV')}
          </Button>
          <Button size="sm" onClick={() => { setTeamFormError(null); setAddDialogOpen(true); }}>
            {isPanache ? t('teams.addPlayer') : t('teams.add')}
          </Button>
        </div>
      </div>

      {importError && (
        <div className="rounded-md bg-red-50 p-4 text-sm text-red-700">
          {importError}
        </div>
      )}

      {importSuccess && (
        <div className="rounded-md bg-green-50 p-4 text-sm text-green-700">
          {importSuccess}
        </div>
      )}

      {loading ? (
        <div className="text-center py-8 text-gray-500">{t('common.loading')}</div>
      ) : teams.length === 0 ? (
        <Card>
          <CardContent className="py-12 text-center">
            <p className="text-gray-500">{isPanache ? t('teams.noPlayers') : t('teams.noTeams')}</p>
          </CardContent>
        </Card>
      ) : (
        <Card className="overflow-hidden">
          <Table>
            <TableHeader>
              <TableRow>
                <TableHead className="w-20">{t('teams.teamNumber')}</TableHead>
                <TableHead>{isPanache ? t('teams.player') : t('teams.captain')}</TableHead>
                {showPlayer2 && <TableHead>{t('teams.player2')}</TableHead>}
                {showPlayer3 && <TableHead>{t('teams.player3')}</TableHead>}
                {isPanache && <TableHead className="w-28">{t('teams.champion')}</TableHead>}
                <TableHead>{t('teams.region')}</TableHead>
                <TableHead>{t('teams.club')}</TableHead>
                <TableHead className="w-24">{t('common.actions')}</TableHead>
              </TableRow>
            </TableHeader>
            <TableBody>
              {[...teams].sort((a, b) => a.teamNumber - b.teamNumber).map((team) => (
                <TableRow key={team.id}>
                  <TableCell className="font-medium">{team.teamNumber}</TableCell>
                  <TableCell className="font-medium">{team.captain}</TableCell>
                  {showPlayer2 && <TableCell>{team.player2}</TableCell>}
                  {showPlayer3 && <TableCell>{team.player3 || '-'}</TableCell>}
                  {isPanache && (
                    <TableCell>
                      <label className="flex items-center gap-2">
                        <input
                          type="checkbox"
                          checked={team.isChampion}
                          onChange={(e) => setTeamChampion(team.id, e.target.checked)}
                          className="h-4 w-4 rounded border-gray-300 text-primary-600 focus:ring-primary-500"
                        />
                        {team.isChampion && (
                          <span className="text-xs font-medium text-amber-700">
                            {t('teams.champion')}
                          </span>
                        )}
                      </label>
                    </TableCell>
                  )}
                  <TableCell>{team.region || '-'}</TableCell>
                  <TableCell>{team.club || '-'}</TableCell>
                  <TableCell>
                    <div className="flex gap-1">
                      <Button
                        variant="ghost"
                        size="sm"
                        onClick={() => {
                          setSelectedTeam(team);
                          setTeamFormError(null);
                          setEditDialogOpen(true);
                        }}
                      >
                        {t('common.edit')}
                      </Button>
                      {!hasRounds && (
                        <Button
                          variant="ghost"
                          size="sm"
                          onClick={() => {
                            setSelectedTeam(team);
                            setDeleteDialogOpen(true);
                          }}
                        >
                          {t('common.delete')}
                        </Button>
                      )}
                    </div>
                  </TableCell>
                </TableRow>
              ))}
            </TableBody>
          </Table>
        </Card>
      )}

      {/* Add Team Dialog */}
      <Dialog open={addDialogOpen} onOpenChange={setAddDialogOpen}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>{t('teams.add')}</DialogTitle>
          </DialogHeader>
          {teamFormError && (
            <div className="rounded-md bg-red-50 p-3 text-sm text-red-700">
              {teamFormError}
            </div>
          )}
          <TeamForm
            defaultValues={{ teamNumber: String(nextTeamNumber) }}
            showPlayer3={showPlayer3}
            isPanache={isPanache}
            onSubmit={handleAddTeam}
            onCancel={() => {
              setAddDialogOpen(false);
              setTeamFormError(null);
            }}
          />
        </DialogContent>
      </Dialog>

      {/* Edit Team Dialog */}
      <Dialog open={editDialogOpen} onOpenChange={setEditDialogOpen}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>{t('teams.edit')}</DialogTitle>
          </DialogHeader>
          {teamFormError && (
            <div className="rounded-md bg-red-50 p-3 text-sm text-red-700">
              {teamFormError}
            </div>
          )}
          {selectedTeam && (
            <TeamForm
              defaultValues={{
                teamNumber: String(selectedTeam.teamNumber),
                captain: selectedTeam.captain,
                player2: selectedTeam.player2,
                player3: selectedTeam.player3 || '',
                region: selectedTeam.region || '',
                club: selectedTeam.club || '',
                isChampion: selectedTeam.isChampion,
              }}
              showPlayer3={showPlayer3}
              isPanache={isPanache}
              onSubmit={handleEditTeam}
              onCancel={() => {
                setEditDialogOpen(false);
                setSelectedTeam(null);
                setTeamFormError(null);
              }}
            />
          )}
        </DialogContent>
      </Dialog>

      {/* Delete Confirmation Dialog */}
      <Dialog open={deleteDialogOpen} onOpenChange={setDeleteDialogOpen}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>{t('common.delete')}</DialogTitle>
          </DialogHeader>
          <p className="text-sm text-gray-500">{t('teams.deleteConfirm')}</p>
          <DialogFooter>
            <Button
              variant="secondary"
              onClick={() => {
                setDeleteDialogOpen(false);
                setSelectedTeam(null);
              }}
            >
              {t('common.cancel')}
            </Button>
            <Button variant="danger" onClick={handleDeleteTeam}>
              {t('common.delete')}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>

      {/* Delete All Teams Confirmation Dialog */}
      <Dialog open={deleteAllDialogOpen} onOpenChange={setDeleteAllDialogOpen}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>{t('teams.deleteAll')}</DialogTitle>
          </DialogHeader>
          <p className="text-sm text-gray-500">{t('teams.deleteAllConfirm')}</p>
          {deleteAllError && (
            <div className="rounded-md bg-red-50 p-3 text-sm text-red-700">
              {deleteAllError}
            </div>
          )}
          <DialogFooter>
            <Button
              variant="secondary"
              onClick={() => {
                setDeleteAllDialogOpen(false);
                setDeleteAllError(null);
              }}
            >
              {t('common.cancel')}
            </Button>
            <Button variant="danger" onClick={handleDeleteAllTeams} disabled={loading}>
              {t('common.delete')}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </div>
  );
}
