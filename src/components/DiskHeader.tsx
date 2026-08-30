import { RefreshCw, HardDrive, Monitor, Moon, Sun } from "lucide-react";

import { Button } from "@/components/ui/button";
import { Tooltip, TooltipContent, TooltipTrigger } from "@/components/ui/tooltip";
import { bytes } from "@/lib/format";
import { useStore, selectedBytes, totalBytes, type Theme } from "@/store";

const TEMAS: { valor: Theme; icone: typeof Sun; nome: string }[] = [
  { valor: "system", icone: Monitor, nome: "Acompanha o sistema" },
  { valor: "light", icone: Sun, nome: "Claro" },
  { valor: "dark", icone: Moon, nome: "Escuro" },
];

function SeletorTema() {
  const theme = useStore((s) => s.theme);
  const setTheme = useStore((s) => s.setTheme);

  return (
    <div className="flex items-center gap-0.5 rounded-lg border bg-muted/40 p-0.5">
      {TEMAS.map(({ valor, icone: Icone, nome }) => (
        <Tooltip key={valor}>
          <TooltipTrigger asChild>
            <button
              type="button"
              onClick={() => setTheme(valor)}
              aria-label={nome}
              aria-pressed={theme === valor}
              className={`grid size-7 place-items-center rounded-md transition-colors ${
                theme === valor
                  ? "bg-background text-foreground shadow-sm"
                  : "text-muted-foreground hover:text-foreground"
              }`}
            >
              <Icone className="size-3.5" />
            </button>
          </TooltipTrigger>
          <TooltipContent>{nome}</TooltipContent>
        </Tooltip>
      ))}
    </div>
  );
}

export function DiskHeader() {
  const disk = useStore((s) => s.disk);
  const scanning = useStore((s) => s.scanning);
  const cleaning = useStore((s) => s.cleaning);
  const scan = useStore((s) => s.scan);
  const marcado = useStore(selectedBytes);
  const encontrado = useStore(totalBytes);

  const total = disk?.total ?? 0;
  const usado = disk?.used ?? 0;
  // Três faixas na mesma barra: o que fica, o que você marcou, o que já está livre.
  const pctUsado = total ? ((usado - marcado) / total) * 100 : 0;
  const pctMarcado = total ? (marcado / total) * 100 : 0;

  return (
    <header
      data-tauri-drag-region
      className="border-b bg-card/60 backdrop-blur-xl sticky top-0 z-20"
    >
      {/* Espaço pros semáforos da janela, que ficam sobre o conteúdo. */}
      <div className="mx-auto max-w-5xl px-8 pt-11 pb-6">
        <div className="flex items-end justify-between gap-6">
          <div>
            <div className="flex items-center gap-2 text-muted-foreground text-xs font-medium uppercase tracking-widest">
              <HardDrive className="size-3.5" />
              Macintosh HD
            </div>
            <div className="mt-2 flex items-baseline gap-2.5">
              <span className="text-5xl font-semibold tracking-tight tabular-nums">
                {bytes(disk?.free ?? 0)}
              </span>
              <span className="text-muted-foreground text-lg">livres de {bytes(total)}</span>
            </div>
          </div>

          <div className="flex items-center gap-2">
            <SeletorTema />
            <Button variant="outline" onClick={() => scan()} disabled={scanning || cleaning}>
              <RefreshCw className={scanning ? "animate-spin" : ""} />
              {scanning ? "Analisando…" : "Analisar de novo"}
            </Button>
          </div>
        </div>

        <div className="mt-6 h-3 w-full overflow-hidden rounded-full bg-emerald-500/15 flex">
          <div
            className="h-full bg-foreground/25 transition-[width] duration-500"
            style={{ width: `${pctUsado}%` }}
          />
          <div
            className="h-full bg-primary transition-[width] duration-500"
            style={{
              width: `${pctMarcado}%`,
              backgroundImage:
                "repeating-linear-gradient(45deg,transparent,transparent 4px,rgba(255,255,255,.25) 4px,rgba(255,255,255,.25) 8px)",
            }}
          />
        </div>

        <div className="mt-3 flex flex-wrap items-center gap-x-5 gap-y-1.5 text-xs text-muted-foreground">
          <Legend className="bg-foreground/25" label={`${bytes(usado - marcado)} em uso`} />
          <Legend className="bg-primary" label={`${bytes(marcado)} marcados pra limpar`} />
          <Legend className="bg-emerald-500/40" label={`${bytes(disk?.free ?? 0)} já livres`} />
          <span className="ml-auto">
            {bytes(encontrado)} recuperáveis encontrados no total
          </span>
        </div>
      </div>
    </header>
  );
}

function Legend({ className, label }: { className: string; label: string }) {
  return (
    <span className="inline-flex items-center gap-1.5">
      <span className={`size-2 rounded-full ${className}`} />
      {label}
    </span>
  );
}
