// Files and folders added to a project: chosen in the dialog or given (dropped, say), then linked from where they are or copied in, as asked.

import { invoke } from "@tauri-apps/api/core";
import { message, open } from "@tauri-apps/plugin-dialog";

const LINK = "Link";
const COPY = "Copy into Project";

/** Files, or folders, to choose; then, linked or copied (into `folder`, the project's folder if null), as asked.
 *  Nothing is added when either dialog is cancelled. */
export async function addFiles(directory: boolean, folder: string | null) {
  const picked = await open({ multiple: true, directory, title: directory ? "Add Folders to the Project" : "Add Files to the Project" });
  if (picked?.length) await addPaths(picked, folder);
}

/** The paths linked or copied (into `folder`, the project's folder if null), as asked: what was done, or null
 *  when the dialog is cancelled and nothing is. */
export async function addPaths(paths: string[], folder: string | null): Promise<"link" | "copy" | null> {
  const what = paths.length === 1 ? `“${paths[0].slice(paths[0].lastIndexOf("/") + 1)}”` : `${paths.length} items`;
  const choice = await message(`${what}: link to where ${paths.length === 1 ? "it is" : "they are"}, or copy into the project?`, {
    title: "Add to the Project",
    buttons: { yes: LINK, no: COPY, cancel: "Cancel" },
  });
  if (choice !== LINK && choice !== COPY) return null;
  const copy = choice === COPY;
  await invoke("add_files", { paths, copy, folder });
  return copy ? "copy" : "link";
}
