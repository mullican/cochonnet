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

export function TeamsList({ tournamentId }: TeamsListProps) {
  const { t } = useTranslation();
  const fileInputRef = useRef<HTMLInputElement>(null);
  const { teams, qualifyingRounds, loading, fetchTeams, createTeam, updateTeam, deleteTeam, deleteAllTeams, importTeams, fetchQualifyingRounds } = useTournamentStore();

  const [addDialogOpen, setAddDialogOpen] = useState(false);
  const [editDialogOpen, setEditDialogOpen] = useState(false);
  const [deleteDialogOpen, setDeleteDialogOpen] = useState(false);
  const [deleteAllDialogOpen, setDeleteAllDialogOpen] = useState(false);
  const [selectedTeam, setSelectedTeam] = useState<Team | null>(null);
  const [importError, setImportError] = useState<string | null>(null);
  const [importSuccess, setImportSuccess] = useState<string | null>(null);
  const [deleteAllError, setDeleteAllError] = useState<string | null>(null);

  useEffect(() => {
    fetchTeams(tournamentId);
    fetchQualifyingRounds(tournamentId);
  }, [tournamentId, fetchTeams, fetchQualifyingRounds]);

  const hasRounds = qualifyingRounds.length > 0;
  const canDeleteAllTeams = teams.length > 0 && !hasRounds;

  const handleAddTeam = async (data: TeamFormData) => {
    try {
      await createTeam({
        tournamentId,
        captain: data.captain,
        player2: data.player2,
        player3: data.player3 || null,
        region: data.region || null,
        club: data.club || null,
      });
      setAddDialogOpen(false);
    } catch (error) {
      console.error('Failed to add team:', error);
    }
  };

  const handleEditTeam = async (data: TeamFormData) => {
    if (!selectedTeam) return;
    try {
      await updateTeam(selectedTeam.id, {
        tournamentId,
        captain: data.captain,
        player2: data.player2,
        player3: data.player3 || null,
        region: data.region || null,
        club: data.club || null,
      });
      setEditDialogOpen(false);
      setSelectedTeam(null);
    } catch (error) {
      console.error('Failed to update team:', error);
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
            captain: row.captain || '',
            player2: row.player2 || '',
            player3: row.player3 || null,
            region: row.region || null,
            club: row.club || null,
          }));

          // Validate required fields
          const invalidTeams = teamsData.filter((t) => !t.captain || !t.player2);
          if (invalidTeams.length > 0) {
            setImportError('Some teams are missing required fields (captain, player2)');
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
    const template = 'captain,player2,player3,region,club\nJohn Doe,Jane Smith,Bob Wilson,North,Club A\n';
    const blob = new Blob([template], { type: 'text/csv' });
    const url = URL.createObjectURL(blob);
    const a = document.createElement('a');
    a.href = url;
    a.download = 'teams_template.csv';
    a.click();
    URL.revokeObjectURL(url);
  };

  return (
    <div className="space-y-4">
      <div className="flex items-center justify-between">
        <h2 className="text-lg font-semibold text-gray-900">{t('teams.title')}</h2>
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
          <Button size="sm" onClick={() => setAddDialogOpen(true)}>
            {t('teams.add')}
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
            <p className="text-gray-500">{t('teams.noTeams')}</p>
          </CardContent>
        </Card>
      ) : (
        <Card>
          <Table>
            <TableHeader>
              <TableRow>
                <TableHead>{t('teams.captain')}</TableHead>
                <TableHead>{t('teams.player2')}</TableHead>
                <TableHead>{t('teams.player3')}</TableHead>
                <TableHead>{t('teams.region')}</TableHead>
                <TableHead>{t('teams.club')}</TableHead>
                <TableHead className="w-24">{t('common.actions')}</TableHead>
              </TableRow>
            </TableHeader>
            <TableBody>
              {teams.map((team) => (
                <TableRow key={team.id}>
                  <TableCell className="font-medium">{team.captain}</TableCell>
                  <TableCell>{team.player2}</TableCell>
                  <TableCell>{team.player3 || '-'}</TableCell>
                  <TableCell>{team.region || '-'}</TableCell>
                  <TableCell>{team.club || '-'}</TableCell>
                  <TableCell>
                    <div className="flex gap-1">
                      <Button
                        variant="ghost"
                        size="sm"
                        onClick={() => {
                          setSelectedTeam(team);
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
          <TeamForm
            onSubmit={handleAddTeam}
            onCancel={() => setAddDialogOpen(false)}
          />
        </DialogContent>
      </Dialog>

      {/* Edit Team Dialog */}
      <Dialog open={editDialogOpen} onOpenChange={setEditDialogOpen}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>{t('teams.edit')}</DialogTitle>
          </DialogHeader>
          {selectedTeam && (
            <TeamForm
              defaultValues={{
                captain: selectedTeam.captain,
                player2: selectedTeam.player2,
                player3: selectedTeam.player3 || '',
                region: selectedTeam.region || '',
                club: selectedTeam.club || '',
              }}
              onSubmit={handleEditTeam}
              onCancel={() => {
                setEditDialogOpen(false);
                setSelectedTeam(null);
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
