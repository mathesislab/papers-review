// The settings an editor's own bar changes, for every editor of the app.

import { invoke } from "@tauri-apps/api/core";
import type { Settings } from "./types";

/** The line numbers of the editors shown, or hidden. */
export async function toggleLineNumbers() {
  const settings = await invoke<Settings>("settings");
  await invoke("set_settings", { settings: { ...settings, line_numbers: !settings.line_numbers } });
}
