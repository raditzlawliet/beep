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

// Ask before a request reads OS environment variables through $processEnv.
export async function confirmProcessEnv(names: string[]): Promise<boolean> {
  const list = names.map((n) => `- ${n}`).join("\n");
  const result = await message(
    `This request reads environment variables with $processEnv:\n\n${list}\n\nTheir values may be sent to the server. Allow for this session?`,
    {
      title: "Allow Environment Variables",
      kind: "warning",
      buttons: { ok: "Allow", cancel: "Cancel" },
    },
  );
  return result === "Allow";
}
