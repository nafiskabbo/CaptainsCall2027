//! Shared types mirroring the Rust serde contract (specs: tasks.plan.md).
//!
//! Field names are snake_case to match Rust's default serde output.

export type Format = "test" | "odi50" | "t20";

// Data is snake_case from the Rust `Role` enum (see core/types.rs).
export type Role =
  | "batter"
  | "wicketkeeper"
  | "allrounder"
  | "pace_bowler"
  | "spin_bowler";

export interface Skills {
  bat: number;
  test_bat: number;
  odi_bat: number;
  t20_bat: number;
  defence: number;
  pace: number;
  spin: number;
  odi_bowl: number;
  t20_bowl: number;
  fielding: number;
  keeping: number;
  leadership: number;
}

export interface Player {
  id: number;
  name: string;
  nation_id: number;
  team_id: number | null;
  age: number;
  role: Role;
  batting_style: string;
  bowling_style: string | null;
  skills: Skills;
  potential: Skills;
  form: number;
  fitness: number;
  morale: number;
  experience: number;
}

export interface Team {
  id: number;
  name: string;
  nation_id: number;
  home_ground_id: number;
  budget: number;
  reputation: number;
  squad_ids: number[];
  is_national: boolean;
}

export interface Nation {
  id: number;
  name: string;
  short_name: string;
  strength: number;
  has_test_status: boolean;
}

export interface Ground {
  id: number;
  name: string;
  nation_id: number;
  boundary_m: number;
  batting_friendly: number;
  day_night: boolean;
}

export interface Competition {
  id: number;
  name: string;
  nation_id: number;
  format: Format;
  kind: "league" | "knockout_cup" | "series" | "tour";
  team_ids: number[];
  days_per_match: number;
}

export interface World {
  schema_version: number;
  db_version: string;
  nations: Nation[];
  teams: Team[];
  players: Player[];
  grounds: Ground[];
  competitions: Competition[];
}

export type MatchPhase =
  | "PreMatch"
  | "Innings1"
  | "Innings2"
  | "Innings3"
  | "Innings4"
  | "Complete";

export type DismissalType =
  | "NotOut"
  | "Bowled"
  | "Caught"
  | "Lbw"
  | "RunOut"
  | "Stumped"
  | "CaughtAndBowled"
  | "HitWicket";

export interface Dismissal {
  out_type: DismissalType;
  bowler_id: number | null;
  fielder_id: number | null;
}

export interface BallOutcome {
  runs: number;
  extra_runs: number;
  is_boundary_four: boolean;
  is_boundary_six: boolean;
  extra: "Wide" | "NoBall" | "Bye" | "LegBye" | null;
  dismissal: Dismissal | null;
  strike_changes: boolean;
  commentary: string;
}

export type FieldPreset =
  | "Attacking"
  | "Balanced"
  | "Defensive"
  | "BoundaryRiding"
  | "SpinTrap";

export interface MatchInput {
  batting_aggression: number;
  keep_strike_toggle: boolean;
  hit_and_run_toggle: boolean;
  target_bowler_id: number;
  field_preset: FieldPreset;
}

export interface BattingCard {
  player_id: number;
  name: string;
  runs: number;
  balls: number;
  fours: number;
  sixes: number;
  out: Dismissal | null;
  batting_position: number;
}

export interface BowlingCard {
  player_id: number;
  name: string;
  balls: number;
  runs: number;
  wickets: number;
  maidens: number;
  wides: number;
  no_balls: number;
}

export interface OverSummary {
  over: number;
  runs: number;
  wickets: number;
  bowler_id: number;
}

export interface Innings {
  batting_team_id: number;
  bowling_team_id: number;
  runs: number;
  wickets: number;
  balls: number;
  extras: number;
  batting_cards: BattingCard[];
  bowling_cards: BowlingCard[];
  fow: number[];
  over_summaries: OverSummary[];
  is_declared: boolean;
}

export interface MatchSnapshot {
  match_id: number;
  format: Format;
  ground_id: number;
  home_team_id: number;
  away_team_id: number;
  phase: MatchPhase;
  current_innings: number;
  innings: Innings[];
  striker_id: number | null;
  non_striker_id: number | null;
  bowler_id: number | null;
  toss_winner: number;
  toss_decision: "Bat" | "Bowl";
  seed: number;
  clock: {
    day: number;
    session: number;
    balls_today: number;
    balls_in_session: number;
  };
  recent_balls: BallOutcome[];
  result: { winner_team_id: number | null; summary: string } | null;
}

export interface TeamSelection {
  team_id: number;
  xi_ids: number[];
  batting_order: number[];
  keeper_id: number | null;
}

export interface AppInfo {
  name: string;
  version: string;
  phase: string;
}

export interface FixtureResult {
  home_runs: number;
  away_runs: number;
  home_wickets: number;
  away_wickets: number;
  winner_id: number | null;
  summary: string;
}

export interface Fixture {
  id: number;
  competition_id: number;
  home_id: number;
  away_id: number;
  format: Format;
  ground_id: number;
  day: number;
  round: number;
  /** International stage label ("Group A", "Semi-final", "1st Test"); null for leagues. */
  stage: string | null;
  result: FixtureResult | null;
}

export interface NewsItem {
  day: number;
  headline: string;
}

export interface TableRow {
  team_id: number;
  played: number;
  won: number;
  lost: number;
  tied: number;
  no_result: number;
  points: number;
  runs_for: number;
  runs_against: number;
  wickets_lost: number;
  net_run_rate_x100: number;
}

export interface CareerState {
  id: number | null;
  name: string;
  player_team_id: number;
  player_team_name: string;
  nation_id: number;
  season_day: number;
  db_version: string;
  competition_id: number;
  fixtures: Fixture[];
  news: NewsItem[];
  selections: Record<number, TeamSelection>;
  players: Player[];
  budget: number | null;
  season: number;
  seed: number;
}

export interface CareerView {
  state: CareerState;
  table: TableRow[];
  next_fixture_id: number | null;
  season_complete: boolean;
  international: boolean;
  records: CareerRecordRow[];
}

export interface CareerRecordRow {
  name: string;
  matches: number;
  runs: number;
  average: number | null;
  strikeRate: number;
  wickets: number;
  bowlingAverage: number | null;
  economy: number;
  hundreds: number;
  fifties: number;
  fiveWicketHauls: number;
}

export interface SaveSummary {
  id: number;
  name: string;
  team_name: string;
  season_day: number;
  db_version: string;
  updated_at: string;
}

export interface TransferListing {
  player_id: number;
  name: string;
  age: number;
  role: string;
  from_team: string;
  wage: number;
  reputation: number;
  interested_in_you: boolean;
  value: number;
}

export type BidOutcome =
  | "Accepted"
  | { RejectedTooLow: { wanted: number } }
  | "NotInterested"
  | { InsufficientFunds: { available: number } };

export interface Finances {
  budget: number;
  wageBill: number;
  squadSize: number;
}
