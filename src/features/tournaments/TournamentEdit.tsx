import { useEffect } from 'react';
import { useParams, useNavigate } from 'react-router-dom';
import { useTranslation } from 'react-i18next';
import { useTournamentStore } from '../../stores/tournamentStore';
import { TournamentForm } from './TournamentForm';
import type { TournamentFormData } from '../../types';

export function TournamentEdit() {
  const { id } = useParams<{ id: string }>();
  const { t } = useTranslation();
  const navigate = useNavigate();
  const { currentTournament, loading, fetchTournament, updateTournament } = useTournamentStore();

  useEffect(() => {
    if (id) {
      fetchTournament(id);
    }
  }, [id, fetchTournament]);

  const handleSubmit = async (data: TournamentFormData) => {
    if (!id) return;

    try {
      await updateTournament(id, {
        name: data.name,
        teamComposition: data.teamComposition,
        type: data.type,
        startDate: data.startDate,
        endDate: data.endDate,
        director: data.director,
        headUmpire: data.headUmpire,
        format: data.format,
        dayType: data.dayType,
        numberOfCourts: data.numberOfCourts,
        numberOfQualifyingRounds: data.numberOfQualifyingRounds,
        hasConsolante: data.hasConsolante,
        advanceAll: data.advanceAll,
        advanceCount: data.advanceCount,
        bracketSize: data.bracketSize,
        pairingMethod: data.pairingMethod,
        regionAvoidance: data.regionAvoidance,
      } as any);
      navigate(`/tournaments/${id}`);
    } catch (error) {
      console.error('Failed to update tournament:', error);
    }
  };

  if (loading && !currentTournament) {
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
          additionalUmpires: [],
          format: currentTournament.format,
          dayType: currentTournament.dayType,
          numberOfCourts: currentTournament.numberOfCourts,
          numberOfQualifyingRounds: currentTournament.numberOfQualifyingRounds,
          hasConsolante: currentTournament.hasConsolante,
          advanceAll: currentTournament.advanceAll,
          advanceCount: currentTournament.advanceCount,
          bracketSize: currentTournament.bracketSize,
          pairingMethod: currentTournament.pairingMethod,
          regionAvoidance: currentTournament.regionAvoidance,
        }}
        onSubmit={handleSubmit}
        onCancel={() => navigate(`/tournaments/${id}`)}
        isLoading={loading}
      />
    </div>
  );
}
