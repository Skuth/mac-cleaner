import { Trash2 } from "lucide-react";

import { bytes } from "@/lib/format";
import { useStore } from "@/store";

/**
 * Tela cheia enquanto apaga. Existe porque um item grande (um AVD de 12 GB)
 * trava a porcentagem por um minuto inteiro — sem animação contínua a janela
 * parece congelada.
 */
export function CleaningOverlay() {
  const cleaning = useStore((s) => s.cleaning);
  const scanning = useStore((s) => s.scanning);
  const progress = useStore((s) => s.progress);
  const alvo = useStore((s) => s.cleanTarget);

  if (!cleaning) return null;

  const total = progress?.total ?? 0;
  const done = progress?.done ?? 0;
  const pct = total ? (done / total) * 100 : 0;
  // Depois do último item o backend remede o disco; sem isso a tela some antes.
  const remedindo = scanning || done >= total;

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-background/80 backdrop-blur-md">
      <div className="mc-entrada w-full max-w-md px-8 text-center">
        <div className="relative mx-auto grid size-28 place-items-center">
          {[0, 0.55, 1.1].map((delay) => (
            <span
              key={delay}
              className="mc-onda absolute size-20 rounded-full border-2 border-primary/40"
              style={{ animationDelay: `${delay}s` }}
            />
          ))}

          {/* Poeira levantada pela vassoura. */}
          {[-22, -8, 6, 20].map((x, i) => (
            <span
              key={x}
              className="mc-particula absolute size-1.5 rounded-full bg-primary/60"
              style={{ left: `calc(50% + ${x}px)`, animationDelay: `${i * 0.28}s` }}
            />
          ))}

          <span className="relative grid size-16 place-items-center rounded-2xl border bg-card shadow-sm">
            <Trash2 className="size-7 text-primary" />
          </span>
        </div>

        <p className="mt-6 text-lg font-semibold">
          {remedindo ? "Conferindo o que sobrou…" : "Apagando…"}
        </p>
        <p className="mt-1 h-5 truncate text-sm text-muted-foreground">
          {remedindo ? "Remedindo o disco" : progress?.label}
        </p>

        <div className="relative mt-6 h-2 overflow-hidden rounded-full bg-muted">
          <div
            className="h-full rounded-full bg-primary transition-[width] duration-500 ease-out"
            style={{ width: `${remedindo ? 100 : pct}%` }}
          />
          {/* O brilho corre sempre, mesmo com a barra parada. */}
          <div className="absolute inset-0 overflow-hidden rounded-full">
            <div className="mc-shimmer h-full w-1/3 bg-gradient-to-r from-transparent via-white/40 to-transparent" />
          </div>
        </div>

        <p className="mt-3 font-mono text-xs tabular-nums text-muted-foreground">
          {done}/{total} · liberando até {bytes(alvo)}
        </p>

        <p className="mt-6 text-xs text-muted-foreground/70">
          Não feche a janela. Itens grandes levam alguns minutos.
        </p>
      </div>
    </div>
  );
}
