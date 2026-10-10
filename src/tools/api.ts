//! Typed wrappers around the Tauri command surface.
//!
//! Rust is authoritative; this is the only module that talks to it.

import { invoke } from "@tauri-apps/api/core";
import type {
  AppInfo,
  BidOutcome,
  CareerView,
  DbPackInfo,
  Finances,
  TransferListing,
  UpdateState,
  Format,
  MatchInput,
  MatchSnapshot,
  Player,
  SaveSummary,
  TeamSelection,
  TrainingFocus,
  World,
} from "../types";

export const api = {
  getAppInfo: () => invoke<AppInfo>("get_app_info"),
  getWorld: () => invoke<World>("get_world"),
  getDbVersion: () => invoke<string>("get_db_version"),
  /** Databases available at new-game; the fictional default is always first. */
  listDbPacks: () => invoke<DbPackInfo[]>("list_db_packs"),
  /** Swap the active database. Refused while a career is in progress. */
  loadDbPack: (key: string) => invoke<World>("load_db_pack", { key }),
  /** Abandon the career in progress without saving. */
  quitToMenu: () => invoke<void>("quit_to_menu"),
  getSquad: (teamId: number) => invoke<Player[]>("get_squad", { teamId }),
  autoSelect: (teamId: number, format: Format) =>
    invoke<TeamSelection>("auto_select", { teamId, format }),

  // ---- career ----
  startCareer: (args: {
    name: string;
    teamId: number;
    seed: number;
    /** Season number the career opens on; 1 is the 2027 season. */
    firstSeason?: number;
    /** Wipe form/fitness/morale to a clean baseline before kick-off. */
    resetForm?: boolean;
  }) =>
    invoke<CareerView>("start_career", {
      name: args.name,
      teamId: args.teamId,
      seed: args.seed,
      firstSeason: args.firstSeason ?? 1,
      resetForm: args.resetForm ?? false,
    }),
  getCareer: () => invoke<CareerView>("get_career"),
  saveCareer: () => invoke<number>("save_career"),
  listSaves: () => invoke<SaveSummary[]>("list_saves"),
  loadCareer: (id: number) => invoke<CareerView>("load_career", { id }),
  deleteSave: (id: number) => invoke<void>("delete_save", { id }),
  advanceDay: () => invoke<CareerView>("advance_day"),
  advanceOneDay: () => invoke<CareerView>("advance_one_day"),
  advanceToMyMatch: () => invoke<CareerView>("advance_to_my_match"),
  startNewSeason: () => invoke<CareerView>("start_new_season"),
  getTransferPool: () => invoke<TransferListing[]>("get_transfer_pool"),
  bidForPlayer: (playerId: number, offeredWage: number) =>
    invoke<BidOutcome>("bid_for_player", { playerId, offeredWage }),
  releasePlayer: (playerId: number) => invoke<void>("release_player", { playerId }),
  getFinances: () => invoke<Finances>("get_finances"),
  setTrainingFocus: (focus: TrainingFocus) =>
    invoke<CareerView>("set_training_focus", { focus }),
  getUpdateState: () => invoke<UpdateState>("get_update_state"),
  setUpdateSettings: (enabled: boolean, channel?: string) =>
    invoke<UpdateState>("set_update_settings", { enabled, channel }),
  checkLocalUpdate: (manifestPath: string) =>
    invoke<UpdateState>("check_local_update", { manifestPath }),

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
  /// Writes the scorecard to disk and returns its path.
  exportScorecard: () => invoke<string>("export_scorecard"),
};
