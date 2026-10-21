export function formatBytes(bytes: number): string {
  if (!bytes || bytes < 0) return "0 B";
  const units = ["B", "KB", "MB", "GB", "TB"];
  let value = bytes;
  let index = 0;
  while (value >= 1024 && index < units.length - 1) {
    value /= 1024;
    index += 1;
  }
  return `${value >= 100 ? value.toFixed(0) : value.toFixed(1)} ${units[index]}`;
}

export function formatSpeed(bytes: number, elapsedMs: number): string {
  if (elapsedMs <= 0) return "—";
  return `${formatBytes((bytes / elapsedMs) * 1000)}/s`;
}

export function kindLabel(kind: string): string {
  switch (kind) {
    case "release":
      return "正式版";
    case "snapshot":
      return "快照";
    case "old_beta":
      return "远古 Beta";
    case "old_alpha":
      return "远古 Alpha";
    default:
      return kind || "未知";
  }
}

export function formatDate(iso?: string | null): string {
  if (!iso) return "—";
  const date = new Date(iso);
  if (Number.isNaN(date.getTime())) return iso;
  return `${date.getFullYear()}-${String(date.getMonth() + 1).padStart(2, "0")}-${String(
    date.getDate()
  ).padStart(2, "0")}`;
}

export function formatDuration(ms: number): string {
  const total = Math.floor(ms / 1000);
  const minutes = Math.floor(total / 60);
  const seconds = total % 60;
  return minutes > 0 ? `${minutes} 分 ${seconds} 秒` : `${seconds} 秒`;
}
