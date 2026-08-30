import type { Risk } from "@/store";

/** Bytes em unidade binária, no padrão que o Finder mostra. */
export function bytes(n: number): string {
  if (n <= 0) return "0 B";
  const units = ["B", "KB", "MB", "GB", "TB"];
  const i = Math.min(Math.floor(Math.log(n) / Math.log(1024)), units.length - 1);
  const v = n / 1024 ** i;
  return `${v.toFixed(v >= 100 || i === 0 ? 0 : 1).replace(".", ",")} ${units[i]}`;
}

export const RISK: Record<Risk, { label: string; hint: string; dot: string; chip: string }> = {
  safe: {
    label: "Regenera",
    hint: "Volta sozinho, sem você fazer nada.",
    dot: "bg-emerald-500",
    chip: "border-emerald-500/30 bg-emerald-500/10 text-emerald-700 dark:text-emerald-400",
  },
  low: {
    label: "Rebaixa",
    hint: "Volta com um comando ou um download.",
    dot: "bg-amber-500",
    chip: "border-amber-500/30 bg-amber-500/10 text-amber-700 dark:text-amber-400",
  },
  medium: {
    label: "Some",
    hint: "Não volta, ou custa caro recuperar. Leia antes de marcar.",
    dot: "bg-rose-500",
    chip: "border-rose-500/30 bg-rose-500/10 text-rose-700 dark:text-rose-400",
  },
};
