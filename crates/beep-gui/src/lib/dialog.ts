import { message } from "@tauri-apps/plugin-dialog";

export type UnsavedChoice = "save" | "dont_save" | "cancel";

// Show a native confirmation for unsaved tab changes.
export async function confirmUnsavedClose(
  label: string,
): Promise<UnsavedChoice> {
  const result = await message(
    `"${label}" has unsaved changes. Save before closing?`,
    {
      title: "Unsaved Changes",
      kind: "warning",
      buttons: { yes: "Save", no: "Don't Save", cancel: "Cancel" },
    },
  );
  if (result === "Save") return "save";
  if (result === "Don't Save") return "dont_save";
  return "cancel";
}

export type OverwriteChoice = "overwrite" | "discard" | "cancel";

// Show a native confirmation when a file changed on disk.
export async function confirmExternalOverwrite(
  label: string,
): Promise<OverwriteChoice> {
  const result = await message(
    `"${label}" has changed on disk since you started editing. Overwrite it?`,
    {
      title: "File Changed on Disk",
      kind: "warning",
      buttons: { yes: "Overwrite", no: "Discard", cancel: "Cancel" },
    },
  );
  if (result === "Overwrite") return "overwrite";
  if (result === "Discard") return "discard";
  return "cancel";
}

// Ask before a request reads OS environment variables through process.env (.env values never ask).
export async function confirmProcessEnv(names: string[]): Promise<boolean> {
  const list = names.map((n) => `- ${n}`).join("\n");
  const result = await message(
    `This request reads OS environment variables (process.env, not in .env):\n\n${list}\n\nTheir values may be sent to the server. Allow for this session?`,
    {
      title: "Allow Environment Variables",
      kind: "warning",
      buttons: { ok: "Allow", cancel: "Cancel" },
    },
  );
  return result === "Allow";
}

export type DeleteEnvChoice = "file" | "all" | "cancel";

// Ask how much of an environment to delete: the file being edited, or every folder level.
export async function confirmDeleteEnv(name: string, file: string, count: number): Promise<DeleteEnvChoice> {
  const result = await message(
    `Delete environment "${name}"?\n\nThis file: ${file}\nAll files: ${count} file(s) of this environment in every folder.`,
    {
      title: "Delete Environment",
      kind: "warning",
      buttons: { yes: "This file", no: "All files", cancel: "Cancel" },
    },
  );
  if (result === "This file") return "file";
  if (result === "All files") return "all";
  return "cancel";
}
