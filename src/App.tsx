import { useEffect, useMemo, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { AlertTriangle, Check, Loader2, ShieldAlert, Sparkles, Trash2, X } from "lucide-react";

import {
  AlertDialog,
  AlertDialogAction,
  AlertDialogCancel,
  AlertDialogContent,
  AlertDialogDescription,
  AlertDialogFooter,
  AlertDialogHeader,
  AlertDialogTitle,
} from "@/components/ui/alert-dialog";
import { Button } from "@/components/ui/button";
import { TooltipProvider } from "@/components/ui/tooltip";
import { CleaningOverlay } from "@/components/CleaningOverlay";
import { DiskHeader } from "@/components/DiskHeader";
import { TargetList } from "@/components/TargetList";
import { RISK, bytes } from "@/lib/format";
import {
  applyTheme,
  recommendedBytes,
  selectedBytes,
  useStore,
  type Risk,
} from "@/store";

export default function App() {
  const { targets, selected, scanning, cleaning, progress, results } = useStore();
  const scan = useStore((s) => s.scan);
  const clean = useStore((s) => s.clean);
  const setMany = useStore((s) => s.setMany);
  const selectRecommended = useStore((s) => s.selectRecommended);
  const clearSelection = useStore((s) => s.clearSelection);
  const dismissResults = useStore((s) => s.dismissResults);
  const setProgress = useStore((s) => s.setProgress);
  const marcado = useStore(selectedBytes);
  const freedNow = useStore((s) => s.freedNow);
  const theme = useStore((s) => s.theme);
  const [confirmando, setConfirmando] = useState(false);
  const [semAcesso, setSemAcesso] = useState(false);

  // O macOS só aplica a permissão nova depois de reabrir o app.
  useEffect(() => {
    invoke<boolean>("full_disk_access").then((ok) => setSemAcesso(!ok));
  }, []);

  useEffect(() => {
    scan();
    const un = ["scan:progress", "clean:progress"].map((ev) =>
      listen<[number, number, string]>(ev, ({ payload: [done, total, label] }) =>
        setProgress({ done, total, label }),
      ),
    );
    return () => {
      un.forEach((p) => p.then((f) => f()));
    };
  }, []);

  // No modo "sistema" o tema tem que virar junto com o macOS, sem reabrir.
  useEffect(() => {
    applyTheme(theme);
    if (theme !== "system") return;
    const mq = matchMedia("(prefers-color-scheme: dark)");
    const onChange = () => applyTheme("system");
    mq.addEventListener("change", onChange);
    return () => mq.removeEventListener("change", onChange);
  }, [theme]);

  const escolhidos = useMemo(
    () => targets.filter((t) => selected.has(t.id)),
    [targets, selected],
  );
  const perigosos = escolhidos.filter((t) => t.risk === "medium");
  const recomendados = useMemo(() => targets.filter((t) => t.recommended), [targets]);
  const bytesRecomendados = useStore(recommendedBytes);

  const marcarPorRisco = (riscos: Risk[]) =>
    setMany(
      targets.filter((t) => riscos.includes(t.risk)).map((t) => t.id),
      true,
    );

  return (
    <TooltipProvider delayDuration={200}>
      <div className="min-h-screen bg-background text-foreground">
        <DiskHeader />

        <main className="mx-auto max-w-5xl px-8 py-6">
          {semAcesso && (
            <div className="mb-6 flex flex-col gap-3 rounded-xl border border-amber-500/30 bg-amber-500/5 p-4 sm:flex-row sm:items-center">
              <ShieldAlert className="size-5 shrink-0 text-amber-600 dark:text-amber-400" />
              <div className="flex-1 text-sm">
                <p className="font-medium">Scan parcial: sem Acesso Total ao Disco</p>
                <p className="mt-0.5 text-xs text-muted-foreground">
                  Projetos em ~/Documents, o disco do Docker e a mídia do WhatsApp ficaram de
                  fora pra não encher a tela de pedidos de permissão. Libere o Mac Cleaner
                  na lista e reabra o app.
                </p>
              </div>
              <Button variant="secondary" onClick={() => invoke("open_fda_settings")}>
                Abrir Ajustes
              </Button>
            </div>
          )}

          {results && (
            <div className="mb-6 rounded-xl border border-emerald-500/30 bg-emerald-500/5 p-5">
              <div className="flex items-start gap-3">
                <Check className="mt-0.5 size-5 shrink-0 text-emerald-600 dark:text-emerald-400" />
                <div className="flex-1">
                  <p className="font-medium">
                    Liberados {bytes(freedNow)} em {results.length}{" "}
                    {results.length === 1 ? "item" : "itens"}.
                  </p>
                  <ul className="mt-2 space-y-1 text-xs text-muted-foreground">
                    {results.map((r) => (
                      <li key={r.id} className="flex gap-2">
                        <span className="font-mono tabular-nums w-20 shrink-0 text-right">
                          {bytes(r.freed)}
                        </span>
                        <span className="truncate">{r.label}</span>
                        {r.error && (
                          <span className="text-rose-600 dark:text-rose-400">— {r.error}</span>
                        )}
                      </li>
                    ))}
                  </ul>
                </div>
                <Button variant="ghost" size="icon" onClick={dismissResults}>
                  <X />
                </Button>
              </div>
            </div>
          )}

          {targets.length > 0 && (
            <div className="mb-5 flex flex-col gap-3 rounded-xl border bg-muted/30 p-4 sm:flex-row sm:items-center">
              <div className="flex-1">
                <p className="flex items-center gap-2 text-sm font-medium">
                  <Sparkles className="size-4" />
                  Recomendado
                </p>
                <p className="mt-0.5 text-xs text-muted-foreground">
                  {recomendados.length} itens que voltam sozinhos, sem nada pra você refazer
                  depois. Deixa de fora os artefatos por projeto — esses só você sabe quais
                  estão dormindo.
                </p>
              </div>
              <Button onClick={selectRecommended} disabled={!recomendados.length}>
                <Sparkles />
                Marcar recomendados · {bytes(bytesRecomendados)}
              </Button>
            </div>
          )}

          <div className="mb-4 flex flex-wrap items-center gap-2">
            <span className="text-xs font-medium uppercase tracking-widest text-muted-foreground">
              Ou marque por risco
            </span>
            <Button size="sm" variant="secondary" onClick={() => marcarPorRisco(["safe"])}>
              <span className={`size-2 rounded-full ${RISK.safe.dot}`} />
              Tudo que regenera
            </Button>
            <Button size="sm" variant="secondary" onClick={() => marcarPorRisco(["safe", "low"])}>
              <span className={`size-2 rounded-full ${RISK.low.dot}`} />
              Regenera + rebaixa
            </Button>
            {selected.size > 0 && (
              <Button size="sm" variant="ghost" onClick={clearSelection}>
                Desmarcar tudo
              </Button>
            )}
          </div>

          {scanning && targets.length === 0 ? (
            <Vazio
              icon={<Loader2 className="size-6 animate-spin" />}
              titulo="Medindo o disco…"
              texto={progress ? `${progress.done}/${progress.total} · ${progress.label}` : "Só um instante."}
            />
          ) : targets.length === 0 ? (
            <Vazio
              icon={<Sparkles className="size-6" />}
              titulo="Nada pra limpar por aqui."
              texto="Não sobrou nenhum alvo acima de 1 MB. Volte depois de uns builds."
            />
          ) : (
            <TargetList />
          )}

          {/* Espaço só quando a barra fixa existe — senão sobra branco embaixo. */}
          {selected.size > 0 && <div className="h-24" />}
        </main>

        {/* Barra de ação — só aparece quando existe escolha feita. */}
        {selected.size > 0 && !cleaning && (
          <div className="fixed inset-x-0 bottom-0 z-30 border-t bg-card/80 backdrop-blur-xl">
            <div className="mx-auto flex max-w-5xl items-center gap-4 px-8 py-4">
              <div>
                <p className="text-sm font-medium">
                  {selected.size} {selected.size === 1 ? "item marcado" : "itens marcados"}
                </p>
                <p className="text-xs text-muted-foreground">
                  Libera cerca de <span className="font-mono">{bytes(marcado)}</span>
                  {perigosos.length > 0 && (
                    <span className="text-rose-600 dark:text-rose-400">
                      {" "}
                      · {perigosos.length} sem volta
                    </span>
                  )}
                </p>
              </div>
              <Button className="ml-auto" size="lg" onClick={() => setConfirmando(true)}>
                <Trash2 />
                Limpar {bytes(marcado)}
              </Button>
            </div>
          </div>
        )}

        <CleaningOverlay />

        <AlertDialog open={confirmando} onOpenChange={setConfirmando}>
          <AlertDialogContent className="max-w-lg">
            <AlertDialogHeader>
              <AlertDialogTitle>Apagar {bytes(marcado)}?</AlertDialogTitle>
              <AlertDialogDescription asChild>
                <div className="space-y-3">
                  <p>
                    {selected.size} {selected.size === 1 ? "item vai" : "itens vão"} pro além. Não
                    passa pela Lixeira.
                  </p>
                  <ul className="max-h-52 space-y-1 overflow-y-auto rounded-lg border bg-muted/40 p-3 text-xs">
                    {escolhidos.map((t) => (
                      <li key={t.id} className="flex gap-2">
                        <span className={`mt-1.5 size-1.5 shrink-0 rounded-full ${RISK[t.risk].dot}`} />
                        <span className="flex-1 truncate">{t.label}</span>
                        <span className="font-mono tabular-nums">{bytes(t.bytes)}</span>
                      </li>
                    ))}
                  </ul>
                  {perigosos.length > 0 && (
                    <div className="flex gap-2 rounded-lg border border-rose-500/30 bg-rose-500/5 p-3 text-xs text-rose-700 dark:text-rose-400">
                      <AlertTriangle className="mt-0.5 size-4 shrink-0" />
                      <div>
                        <p className="font-medium">
                          {perigosos.length}{" "}
                          {perigosos.length === 1 ? "item não volta" : "itens não voltam"}:
                        </p>
                        <p className="mt-0.5">{perigosos.map((t) => t.label).join(", ")}.</p>
                      </div>
                    </div>
                  )}
                </div>
              </AlertDialogDescription>
            </AlertDialogHeader>
            <AlertDialogFooter>
              <AlertDialogCancel>Cancelar</AlertDialogCancel>
              <AlertDialogAction onClick={() => clean()}>Apagar agora</AlertDialogAction>
            </AlertDialogFooter>
          </AlertDialogContent>
        </AlertDialog>
      </div>
    </TooltipProvider>
  );
}

function Vazio({
  icon,
  titulo,
  texto,
}: {
  icon: React.ReactNode;
  titulo: string;
  texto: string;
}) {
  return (
    <div className="flex flex-col items-center gap-3 rounded-xl border border-dashed py-20 text-center">
      <div className="text-muted-foreground">{icon}</div>
      <p className="font-medium">{titulo}</p>
      <p className="max-w-sm text-sm text-muted-foreground">{texto}</p>
    </div>
  );
}
