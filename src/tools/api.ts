//! Typed wrappers around the Tauri command surface.
//!
//! Rust is authoritative; this is the only module that talks to it.

import { invoke } from "@tauri-apps/api/core";
import type {
  AppInfo,
  BidOutcome,
  CareerView,
  Finances,
  TransferListing,
  Format,
  MatchInput,
  MatchSnapshot,
  Player,
  SaveSummary,
  TeamSelection,
  World,
} from "../types";

export const api = {
  getAppInfo: () => invoke<AppInfo>("get_app_info"),
  getWorld: () => invoke<World>("get_world"),
  getDbVersion: () => invoke<string>("get_db_version"),
  getSquad: (teamId: number) => invoke<Player[]>("get_squad", { teamId }),
  autoSelect: (teamId: number, format: Format) =>
    invoke<TeamSelection>("auto_select", { teamId, format }),

  // ---- career ----
  startCareer: (args: { name: string; teamId: number; seed: number }) =>
    invoke<CareerView>("start_career", {
      name: args.name,
      teamId: args.teamId,
      seed: args.seed,
    }),
  getCareer: () => invoke<CareerView>("get_career"),
  saveCareer: () => invoke<number>("save_career"),
  listSaves: () => invoke<SaveSummary[]>("list_saves"),
  loadCareer: (id: number) => invoke<CareerView>("load_career", { id }),
  deleteSave: (id: number) => invoke<void>("delete_save", { id }),
  advanceDay: () => invoke<CareerView>("advance_day"),
  startNewSeason: () => invoke<CareerView>("start_new_season"),
  getTransferPool: () => invoke<TransferListing[]>("get_transfer_pool"),
  bidForPlayer: (playerId: number, offeredWage: number) =>
    invoke<BidOutcome>("bid_for_player", { playerId, offeredWage }),
  releasePlayer: (playerId: number) => invoke<void>("release_player", { playerId }),
  getFinances: () => invoke<Finances>("get_finances"),

  // ---- selection ----
  getSelection: (fixtureId: number) =>
    invoke<TeamSelection>("get_selection", { fixtureId }),
  setSelection: (fixtureId: number, selection: TeamSelection) =>
    invoke<void>("set_selection", { fixtureId, selection }),

  // ---- live match ----
  startFixture: (fixtureId: number) => invoke<MatchSnapshot>("start_fixture", { fixtureId }),
  getMatch: () => invoke<MatchSnapshot | null>("get_match"),
  setMatchInput: (input: MatchInput) => invoke<void>("set_match_input", { input }),
  /** Advance `count` deliveries and return the new snapshot. */
  simMatch: (count: number) => invoke<MatchSnapshot>("sim_match", { count }),
  /** Fast-forward to the end of the current innings. */
  simInnings: () => invoke<MatchSnapshot>("sim_innings"),
  exportScorecard: () => invoke<string>("export_scorecard"),
};
