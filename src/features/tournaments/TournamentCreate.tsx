import { useNavigate } from 'react-router-dom';
import { useTranslation } from 'react-i18next';
import { useTournamentStore } from '../../stores/tournamentStore';
import { TournamentForm } from './TournamentForm';
import type { TournamentFormData } from '../../types';

export function TournamentCreate() {
  const { t } = useTranslation();
  const navigate = useNavigate();
  const { createTournament, loading } = useTournamentStore();

  const handleSubmit = async (data: TournamentFormData) => {
    try {
      const tournament = await createTournament({
        name: data.name,
        teamComposition: data.teamComposition,
        type: data.type,
        startDate: data.startDate,
        endDate: data.endDate,
        director: data.director,
        headUmpire: data.headUmpire,
        format: data.format,
        numberOfCourts: data.numberOfCourts,
        numberOfQualifyingRounds: data.numberOfQualifyingRounds,
        hasConsolante: data.hasConsolante,
        advanceAll: data.advanceAll,
        advanceCount: data.advanceCount,
        bracketSize: data.bracketSize,
        pairingMethod: data.pairingMethod,
        regionAvoidance: data.regionAvoidance,
      } as any);
      navigate(`/tournaments/${tournament.id}`);
    } catch (error) {
      console.error('Failed to create tournament:', error);
    }
  };

  return (
    <div className="space-y-6">
      <div className="flex items-center gap-4">
        <button
          onClick={() => navigate('/')}
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
        <h1 className="text-2xl font-bold text-gray-900">{t('tournaments.create')}</h1>
      </div>

      <TournamentForm
        onSubmit={handleSubmit}
        onCancel={() => navigate('/')}
        isLoading={loading}
      />
    </div>
  );
}
