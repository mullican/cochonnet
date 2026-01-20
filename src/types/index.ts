export type TeamComposition = 'men' | 'women' | 'mixed' | 'select';
export type TournamentType = 'regional' | 'national' | 'club';
export type TournamentFormat = 'single' | 'double' | 'triple';
export type DayType = 'single' | 'two';
export type PairingMethod = 'swiss' | 'roundRobin';
export type BracketSize = 4 | 8 | 16 | 32;

export interface Tournament {
  id: string;
  name: string;
  teamComposition: TeamComposition;
  type: TournamentType;
  startDate: string;
  endDate: string;
  director: string;
  headUmpire: string;
  format: TournamentFormat;
  dayType: DayType;
  numberOfCourts: number;
  numberOfQualifyingRounds: number;
  hasConsolante: boolean;
  advanceAll: boolean;
  advanceCount: BracketSize | null;
  bracketSize: BracketSize;
  pairingMethod: PairingMethod;
  regionAvoidance: boolean;
  createdAt: string;
  updatedAt: string;
}

export interface Umpire {
  id: string;
  tournamentId: string;
  name: string;
}

export interface Team {
  id: string;
  tournamentId: string;
  captain: string;
  player2: string;
  player3: string | null;
  region: string | null;
  club: string | null;
  createdAt: string;
}

export interface QualifyingRound {
  id: string;
  tournamentId: string;
  roundNumber: number;
  isComplete: boolean;
  createdAt: string;
}

export interface QualifyingGame {
  id: string;
  roundId: string;
  courtNumber: number;
  team1Id: string | null;
  team2Id: string | null;
  team1Score: number | null;
  team2Score: number | null;
  isBye: boolean;
}

export interface TeamStanding {
  id: string;
  tournamentId: string;
  teamId: string;
  wins: number;
  losses: number;
  pointsFor: number;
  pointsAgainst: number;
  differential: number;
  buchholzScore: number;
  rank: number;
}

export interface Bracket {
  id: string;
  tournamentId: string;
  name: string;
  isConsolante: boolean;
  size: BracketSize;
  isComplete: boolean;
  createdAt: string;
}

export interface BracketMatch {
  id: string;
  bracketId: string;
  roundNumber: number;
  matchNumber: number;
  courtNumber: number | null;
  team1Id: string | null;
  team2Id: string | null;
  team1Score: number | null;
  team2Score: number | null;
  winnerId: string | null;
  nextMatchId: string | null;
  isBye: boolean;
}

export interface PairingHistory {
  id: string;
  tournamentId: string;
  team1Id: string;
  team2Id: string;
  roundId: string;
}

export interface CourtHistory {
  id: string;
  tournamentId: string;
  teamId: string;
  courtNumber: number;
  roundId: string;
}

// Form types
export interface TournamentFormData {
  name: string;
  teamComposition: TeamComposition;
  type: TournamentType;
  startDate: string;
  endDate: string;
  director: string;
  headUmpire: string;
  additionalUmpires: { value: string }[];
  format: TournamentFormat;
  dayType: DayType;
  numberOfCourts: number;
  numberOfQualifyingRounds: number;
  hasConsolante: boolean;
  advanceAll: boolean;
  advanceCount: number | null;
  bracketSize: number;
  pairingMethod: PairingMethod;
  regionAvoidance: boolean;
}

export interface TeamFormData {
  captain: string;
  player2: string;
  player3: string;
  region: string;
  club: string;
}

// CSV Import
export interface CSVTeamRow {
  captain: string;
  player2: string;
  player3?: string;
  region?: string;
  club?: string;
}

// Standings with team details
export interface StandingWithTeam extends TeamStanding {
  team: Team;
}

// Game with team details
export interface GameWithTeams extends QualifyingGame {
  team1: Team | null;
  team2: Team | null;
}

// Match with team details
export interface MatchWithTeams extends BracketMatch {
  team1: Team | null;
  team2: Team | null;
  winner: Team | null;
}
