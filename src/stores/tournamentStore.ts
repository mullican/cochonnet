import { create } from 'zustand';
import type { Tournament, Team, QualifyingRound, QualifyingGame, TeamStanding, Bracket, BracketMatch } from '../types';
import { invoke } from '@tauri-apps/api/core';

interface TournamentState {
  tournaments: Tournament[];
  currentTournament: Tournament | null;
  teams: Team[];
  qualifyingRounds: QualifyingRound[];
  qualifyingGames: QualifyingGame[];
  standings: TeamStanding[];
  brackets: Bracket[];
  bracketMatches: BracketMatch[];
  loading: boolean;
  error: string | null;

  // Tournament actions
  fetchTournaments: () => Promise<void>;
  fetchTournament: (id: string) => Promise<void>;
  createTournament: (data: Partial<Tournament>) => Promise<Tournament>;
  updateTournament: (id: string, data: Partial<Tournament>) => Promise<void>;
  deleteTournament: (id: string) => Promise<void>;
  setCurrentTournament: (tournament: Tournament | null) => void;

  // Team actions
  fetchTeams: (tournamentId: string) => Promise<void>;
  createTeam: (data: Partial<Team>) => Promise<Team>;
  updateTeam: (id: string, data: Partial<Team>) => Promise<void>;
  deleteTeam: (id: string) => Promise<void>;
  importTeams: (tournamentId: string, teams: Partial<Team>[]) => Promise<number>;

  // Qualifying round actions
  fetchQualifyingRounds: (tournamentId: string) => Promise<void>;
  generatePairings: (tournamentId: string) => Promise<QualifyingRound>;
  generateAllQualifyingRounds: (tournamentId: string) => Promise<QualifyingRound[]>;
  fetchGamesForRound: (roundId: string) => Promise<void>;
  updateGameScore: (gameId: string, team1Score: number, team2Score: number) => Promise<void>;
  completeRound: (roundId: string) => Promise<void>;

  // Standings actions
  fetchStandings: (tournamentId: string) => Promise<void>;

  // Bracket actions
  fetchBrackets: (tournamentId: string) => Promise<void>;
  generateBrackets: (tournamentId: string) => Promise<void>;
  fetchMatchesForBracket: (bracketId: string) => Promise<void>;
  updateMatchScore: (matchId: string, team1Score: number, team2Score: number) => Promise<void>;

  // Utility
  clearError: () => void;
}

export const useTournamentStore = create<TournamentState>((set, get) => ({
  tournaments: [],
  currentTournament: null,
  teams: [],
  qualifyingRounds: [],
  qualifyingGames: [],
  standings: [],
  brackets: [],
  bracketMatches: [],
  loading: false,
  error: null,

  // Tournament actions
  fetchTournaments: async () => {
    set({ loading: true, error: null });
    try {
      const tournaments = await invoke<Tournament[]>('get_tournaments');
      set({ tournaments, loading: false });
    } catch (error) {
      set({ error: String(error), loading: false });
    }
  },

  fetchTournament: async (id: string) => {
    set({ loading: true, error: null });
    try {
      const tournament = await invoke<Tournament>('get_tournament', { id });
      set({ currentTournament: tournament, loading: false });
    } catch (error) {
      set({ error: String(error), loading: false });
    }
  },

  createTournament: async (data: Partial<Tournament>) => {
    set({ loading: true, error: null });
    try {
      const tournament = await invoke<Tournament>('create_tournament', { data });
      set((state) => ({
        tournaments: [...state.tournaments, tournament],
        loading: false,
      }));
      return tournament;
    } catch (error) {
      set({ error: String(error), loading: false });
      throw error;
    }
  },

  updateTournament: async (id: string, data: Partial<Tournament>) => {
    set({ loading: true, error: null });
    try {
      await invoke('update_tournament', { id, data });
      set((state) => ({
        tournaments: state.tournaments.map((t) =>
          t.id === id ? { ...t, ...data } : t
        ),
        currentTournament:
          state.currentTournament?.id === id
            ? { ...state.currentTournament, ...data }
            : state.currentTournament,
        loading: false,
      }));
    } catch (error) {
      set({ error: String(error), loading: false });
      throw error;
    }
  },

  deleteTournament: async (id: string) => {
    set({ loading: true, error: null });
    try {
      await invoke('delete_tournament', { id });
      set((state) => ({
        tournaments: state.tournaments.filter((t) => t.id !== id),
        currentTournament:
          state.currentTournament?.id === id ? null : state.currentTournament,
        loading: false,
      }));
    } catch (error) {
      set({ error: String(error), loading: false });
      throw error;
    }
  },

  setCurrentTournament: (tournament: Tournament | null) => {
    set({ currentTournament: tournament });
  },

  // Team actions
  fetchTeams: async (tournamentId: string) => {
    set({ loading: true, error: null });
    try {
      const teams = await invoke<Team[]>('get_teams', { tournamentId });
      set({ teams, loading: false });
    } catch (error) {
      set({ error: String(error), loading: false });
    }
  },

  createTeam: async (data: Partial<Team>) => {
    set({ loading: true, error: null });
    try {
      const team = await invoke<Team>('create_team', { data });
      set((state) => ({
        teams: [...state.teams, team],
        loading: false,
      }));
      return team;
    } catch (error) {
      set({ error: String(error), loading: false });
      throw error;
    }
  },

  updateTeam: async (id: string, data: Partial<Team>) => {
    set({ loading: true, error: null });
    try {
      await invoke('update_team', { id, data });
      set((state) => ({
        teams: state.teams.map((t) => (t.id === id ? { ...t, ...data } : t)),
        loading: false,
      }));
    } catch (error) {
      set({ error: String(error), loading: false });
      throw error;
    }
  },

  deleteTeam: async (id: string) => {
    set({ loading: true, error: null });
    try {
      await invoke('delete_team', { id });
      set((state) => ({
        teams: state.teams.filter((t) => t.id !== id),
        loading: false,
      }));
    } catch (error) {
      set({ error: String(error), loading: false });
      throw error;
    }
  },

  importTeams: async (tournamentId: string, teams: Partial<Team>[]) => {
    set({ loading: true, error: null });
    try {
      const count = await invoke<number>('import_teams', { tournamentId, teams });
      await get().fetchTeams(tournamentId);
      set({ loading: false });
      return count;
    } catch (error) {
      set({ error: String(error), loading: false });
      throw error;
    }
  },

  // Qualifying round actions
  fetchQualifyingRounds: async (tournamentId: string) => {
    set({ loading: true, error: null });
    try {
      const rounds = await invoke<QualifyingRound[]>('get_qualifying_rounds', {
        tournamentId,
      });
      set({ qualifyingRounds: rounds, loading: false });
    } catch (error) {
      set({ error: String(error), loading: false });
    }
  },

  generatePairings: async (tournamentId: string) => {
    set({ loading: true, error: null });
    try {
      const round = await invoke<QualifyingRound>('generate_pairings', {
        tournamentId,
      });
      set((state) => ({
        qualifyingRounds: [...state.qualifyingRounds, round],
        loading: false,
      }));
      return round;
    } catch (error) {
      set({ error: String(error), loading: false });
      throw error;
    }
  },

  generateAllQualifyingRounds: async (tournamentId: string) => {
    set({ loading: true, error: null });
    try {
      const rounds = await invoke<QualifyingRound[]>('generate_all_qualifying_rounds', {
        tournamentId,
      });
      set((state) => ({
        qualifyingRounds: [...state.qualifyingRounds, ...rounds],
        loading: false,
      }));
      return rounds;
    } catch (error) {
      set({ error: String(error), loading: false });
      throw error;
    }
  },

  fetchGamesForRound: async (roundId: string) => {
    set({ loading: true, error: null });
    try {
      const games = await invoke<QualifyingGame[]>('get_games_for_round', {
        roundId,
      });
      set({ qualifyingGames: games, loading: false });
    } catch (error) {
      set({ error: String(error), loading: false });
    }
  },

  updateGameScore: async (
    gameId: string,
    team1Score: number,
    team2Score: number
  ) => {
    set({ loading: true, error: null });
    try {
      await invoke('update_game_score', { gameId, team1Score, team2Score });
      set((state) => ({
        qualifyingGames: state.qualifyingGames.map((g) =>
          g.id === gameId ? { ...g, team1Score, team2Score } : g
        ),
        loading: false,
      }));
    } catch (error) {
      set({ error: String(error), loading: false });
      throw error;
    }
  },

  completeRound: async (roundId: string) => {
    set({ loading: true, error: null });
    try {
      await invoke('complete_round', { roundId });
      set((state) => ({
        qualifyingRounds: state.qualifyingRounds.map((r) =>
          r.id === roundId ? { ...r, isComplete: true } : r
        ),
        loading: false,
      }));
    } catch (error) {
      set({ error: String(error), loading: false });
      throw error;
    }
  },

  // Standings actions
  fetchStandings: async (tournamentId: string) => {
    set({ loading: true, error: null });
    try {
      const standings = await invoke<TeamStanding[]>('get_standings', {
        tournamentId,
      });
      set({ standings, loading: false });
    } catch (error) {
      set({ error: String(error), loading: false });
    }
  },

  // Bracket actions
  fetchBrackets: async (tournamentId: string) => {
    set({ loading: true, error: null });
    try {
      const brackets = await invoke<Bracket[]>('get_brackets', { tournamentId });
      set({ brackets, loading: false });
    } catch (error) {
      set({ error: String(error), loading: false });
    }
  },

  generateBrackets: async (tournamentId: string) => {
    set({ loading: true, error: null });
    try {
      await invoke('generate_brackets', { tournamentId });
      await get().fetchBrackets(tournamentId);
      set({ loading: false });
    } catch (error) {
      set({ error: String(error), loading: false });
      throw error;
    }
  },

  fetchMatchesForBracket: async (bracketId: string) => {
    set({ loading: true, error: null });
    try {
      const matches = await invoke<BracketMatch[]>('get_matches_for_bracket', {
        bracketId,
      });
      set({ bracketMatches: matches, loading: false });
    } catch (error) {
      set({ error: String(error), loading: false });
    }
  },

  updateMatchScore: async (
    matchId: string,
    team1Score: number,
    team2Score: number
  ) => {
    set({ loading: true, error: null });
    try {
      await invoke('update_match_score', { matchId, team1Score, team2Score });
      set((state) => ({
        bracketMatches: state.bracketMatches.map((m) =>
          m.id === matchId
            ? {
                ...m,
                team1Score,
                team2Score,
                winnerId: team1Score > team2Score ? m.team1Id : m.team2Id,
              }
            : m
        ),
        loading: false,
      }));
    } catch (error) {
      set({ error: String(error), loading: false });
      throw error;
    }
  },

  clearError: () => set({ error: null }),
}));
