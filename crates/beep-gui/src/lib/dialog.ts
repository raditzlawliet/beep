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
