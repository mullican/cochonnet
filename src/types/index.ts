export type TeamComposition = 'men' | 'women' | 'mixed' | 'select';
export type TournamentType = 'regional' | 'national' | 'open' | 'club';
export type TournamentFormat = 'single' | 'double' | 'triple';
export type PairingMethod = 'swiss' | 'swissHotel' | 'roundRobin' | 'poolPlay' | 'panache';
export type BracketSize = 4 | 8 | 16 | 32;
/**
 * Page size a document can be printed on. Chosen per document at print time,
 * not stored on the tournament: a deep bracket wants 13x19 on the same day the
 * standings want Letter.
 */
export type PaperSize = 'letter' | 'legal' | 'tabloid' | '13x19' | 'a4' | 'a3';

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
  numberOfCourts: number;
  numberOfQualifyingRounds: number;
  hasConsolante: boolean;
  advanceAll: boolean;
  advanceCount: BracketSize | null;
  bracketSize: BracketSize;
  pairingMethod: PairingMethod;
  regionAvoidance: boolean;
  /** Data URI of the tournament logo, printed top-right on every PDF. */
  logo: string | null;
  createdAt: string;
  updatedAt: string;
}

export interface Umpire {
  id: string;
  tournamentId: string;
  name: string;
}

/**
 * A registered competitor. In panache the roster is individuals, so a Team is one
 * player: `captain` holds the name and `player2` is empty, exactly as singles
 * registration already works.
 */
export interface Team {
  id: string;
  tournamentId: string;
  teamNumber: number;
  captain: string;
  player2: string;
  player3: string | null;
  region: string | null;
  club: string | null;
  /** Panache only: an expert the draw keeps off the same team as other champions. */
  isChampion: boolean;
  /** Pulled out mid-tournament: still listed and still holding past results, but skipped by every later draw. */
  isWithdrawn: boolean;
  createdAt: string;
}

export interface QualifyingRound {
  id: string;
  tournamentId: string;
  roundNumber: number;
  isComplete: boolean;
  /** Panache only: the single championship game played after the qualifying rounds. */
  isFinal: boolean;
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
  /** Panache only: for these games team1Id/team2Id are null and the sides are temporary teams. */
  side1Id: string | null;
  side2Id: string | null;
}

/** One temporary panache team, drawn fresh for a single round. */
export interface PanacheSide {
  id: string;
  teamIndex: number;
  members: Team[];
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
  fineBuchholzScore: number;
  pointQuotient: number;
  isEliminated: boolean;
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
  /** The court was set by hand, so the automatic renumbering leaves it alone. */
  courtIsManual: boolean;
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
  numberOfCourts: number;
  numberOfQualifyingRounds: number;
  hasConsolante: boolean;
  advanceAll: boolean;
  advanceCount: number | null;
  bracketSize: number;
  pairingMethod: PairingMethod;
  regionAvoidance: boolean;
  logo: string | null;
}

export interface TeamFormData {
  teamNumber: string;
  captain: string;
  player2: string;
  player3: string;
  region: string;
  club: string;
  isChampion: boolean;
  isWithdrawn: boolean;
}

// CSV Import
export interface CSVTeamRow {
  number?: string;
  captain: string;
  player2: string;
  player3?: string;
  region?: string;
  club?: string;
  /** Panache roster column; any of "1", "true", "yes", "y", "x" marks a champion. */
  champion?: string;
  /** Panache rosters name the column "name" rather than "captain". */
  name?: string;
}

// Standings with team details
export interface StandingWithTeam extends TeamStanding {
  team: Team;
}

// Game with team details
export interface GameWithTeams extends QualifyingGame {
  team1: Team | null;
  team2: Team | null;
  /** Panache only; null for every other format. */
  side1: PanacheSide | null;
  side2: PanacheSide | null;
}

// Match with team details
export interface MatchWithTeams extends BracketMatch {
  team1: Team | null;
  team2: Team | null;
  winner: Team | null;
}
