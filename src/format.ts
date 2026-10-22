export const date = (value: string | null) =>
  value
    ? new Date(value).toLocaleString(undefined, {
        dateStyle: "medium",
        timeStyle: "short",
      })
    : "Not observed";
export const size = (bytes: number) =>
  bytes < 1024
    ? `${bytes} B`
    : bytes < 1048576
      ? `${(bytes / 1024).toFixed(1)} KB`
      : `${(bytes / 1048576).toFixed(1)} MB`;
// Canonical Windows paths use an extended prefix; present a normal Explorer path.
export function displayPath(path: string): string {
  if (path.startsWith("\\\\?\\UNC\\")) return "\\\\" + path.slice(8);
  return path.startsWith("\\\\?\\") ? path.slice(4) : path;
}

// Filename versions belong to the author; snapshot numbers are save revisions.
export function versionName(
  filename: string | undefined,
  revision: number,
): string {
  const stem = filename?.replace(/\.[^.]+$/, "") ?? "";
  const label = stem.match(
    /(?:^|[\s_(-])v(?:ersion)?[\s_-]*(\d+(?:\.\d+)*)\)?(?:[\s_-]+(?:(?:19|20)\d{2}[-_]\d{2}[-_]\d{2}|(?:19|20)\d{6}))*$/i,
  );
  return label ? `v${label[1]}` : `Saved revision ${revision}`;
}
