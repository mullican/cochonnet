import { useEffect, useState } from 'react';
import { useParams, useNavigate } from 'react-router-dom';
import { useTranslation } from 'react-i18next';
import { useTournamentStore } from '../../stores/tournamentStore';
import { TournamentForm } from './TournamentForm';
import type { TournamentFormData } from '../../types';

export function TournamentEdit() {
  const { id } = useParams<{ id: string }>();
  const { t } = useTranslation();
  const navigate = useNavigate();
  const {
    currentTournament,
    loading,
    fetchTournament,
    updateTournament,
    qualifyingRounds,
    brackets,
    fetchQualifyingRounds,
    fetchBrackets,
    umpires,
    fetchUmpires,
  } = useTournamentStore();

  // The form reads its defaults once, when it mounts, so it must not mount
  // until the umpires are in - otherwise it opens with an empty list and
  // saving wipes them.
  const [umpiresLoaded, setUmpiresLoaded] = useState(false);
  // The backend refuses some edits outright - the courts, round count and
  // pairing method of a tournament that is already under way. Those fields are
  // disabled in the form, but a refusal has to be readable if one gets through
  // at all; this used to go to console.error and the save just did nothing.
  const [saveError, setSaveError] = useState<string | null>(null);

  useEffect(() => {
    if (id) {
      setUmpiresLoaded(false);
      fetchTournament(id);
      fetchQualifyingRounds(id);
      fetchBrackets(id);
      fetchUmpires(id).finally(() => setUmpiresLoaded(true));
    }
  }, [id, fetchTournament, fetchQualifyingRounds, fetchBrackets, fetchUmpires]);

  const hasQualifyingRounds = qualifyingRounds.length > 0;
  const hasBrackets = brackets.length > 0;

  const handleSubmit = async (data: TournamentFormData) => {
    if (!id) return;
    setSaveError(null);

    try {
      await updateTournament(id, {
        name: data.name,
        teamComposition: data.teamComposition,
        type: data.type,
        startDate: data.startDate,
        endDate: data.endDate,
        director: data.director,
        headUmpire: data.headUmpire,
        // Always sent, even when empty: that is how a removed umpire is
        // actually removed.
        additionalUmpires: data.additionalUmpires.map((umpire) => umpire.value),
        format: data.format,
        numberOfCourts: data.numberOfCourts,
        numberOfQualifyingRounds: data.numberOfQualifyingRounds,
        hasConsolante: data.hasConsolante,
        advanceAll: data.advanceAll,
        advanceCount: data.advanceCount,
        bracketSize: data.bracketSize,
        pairingMethod: data.pairingMethod,
        regionAvoidance: data.regionAvoidance,
        logo: data.logo,
      } as any);
      navigate(`/tournaments/${id}`);
    } catch (error) {
      setSaveError(String(error));
    }
  };

  if ((loading && !currentTournament) || !umpiresLoaded) {
    return <div className="text-center py-8 text-gray-500">{t('common.loading')}</div>;
  }

  if (!currentTournament) {
    return <div className="text-center py-8 text-gray-500">{t('common.error')}</div>;
  }

  return (
    <div className="space-y-6">
      <div className="flex items-center gap-4">
        <button
          onClick={() => navigate(`/tournaments/${id}`)}
          className="text-gray-500 hover:text-gray-700"
        >
          <svg
            xmlns="http://www.w3.org/2000/svg"
            width="24"
            height="24"
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            strokeWidth="2"
            strokeLinecap="round"
            strokeLinejoin="round"
          >
            <path d="m15 18-6-6 6-6" />
          </svg>
        </button>
        <h1 className="text-2xl font-bold text-gray-900">{t('tournaments.edit')}</h1>
      </div>

      <TournamentForm
        defaultValues={{
          name: currentTournament.name,
          teamComposition: currentTournament.teamComposition,
          type: currentTournament.type,
          startDate: currentTournament.startDate.split('T')[0],
          endDate: currentTournament.endDate.split('T')[0],
          director: currentTournament.director,
          headUmpire: currentTournament.headUmpire,
          additionalUmpires: umpires.map((umpire) => ({ value: umpire.name })),
          format: currentTournament.format,
          numberOfCourts: currentTournament.numberOfCourts,
          numberOfQualifyingRounds: currentTournament.numberOfQualifyingRounds,
          hasConsolante: currentTournament.hasConsolante,
          advanceAll: currentTournament.advanceAll,
          advanceCount: currentTournament.advanceCount,
          bracketSize: currentTournament.bracketSize,
          pairingMethod: currentTournament.pairingMethod,
          regionAvoidance: currentTournament.regionAvoidance,
          logo: currentTournament.logo,
        }}
        onSubmit={handleSubmit}
        onCancel={() => navigate(`/tournaments/${id}`)}
        isLoading={loading}
        hasQualifyingRounds={hasQualifyingRounds}
        hasBrackets={hasBrackets}
        error={saveError}
      />
    </div>
  );
}
