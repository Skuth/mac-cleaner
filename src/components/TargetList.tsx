import { CornerDownRight, Sparkles } from "lucide-react";

import {
  Accordion,
  AccordionContent,
  AccordionItem,
  AccordionTrigger,
} from "@/components/ui/accordion";
import { Checkbox } from "@/components/ui/checkbox";
import { Tooltip, TooltipContent, TooltipTrigger } from "@/components/ui/tooltip";
import { RISK, bytes } from "@/lib/format";
import { byGroup, useStore, type Target } from "@/store";

export function TargetList() {
  const targets = useStore((s) => s.targets);
  const scanId = useStore((s) => s.scanId);
  const grupos = byGroup(targets);

  return (
    // key por scan: o Radix mede a altura do conteúdo na montagem e guarda numa
    // CSS var. Quando itens somem depois da limpeza a medida fica velha e o
    // grupo mantém a altura antiga — o espaço em branco. Remontar remede tudo,
    // e de quebra deixa aberto qualquer grupo que só apareceu agora.
    <Accordion
      key={scanId}
      type="multiple"
      defaultValue={grupos.map((g) => g.group)}
      className="divide-y rounded-xl border bg-card"
    >
      {grupos.map((g) => (
        <AccordionItem key={g.group} value={g.group} className="border-b-0 px-1">
          <GroupHeader group={g.group} items={g.items} bytes={g.bytes} />
          <AccordionContent className="pb-2">
            <div className="divide-y divide-border/60">
              {g.items.map((t) => (
                <Row key={t.id} target={t} />
              ))}
            </div>
          </AccordionContent>
        </AccordionItem>
      ))}
    </Accordion>
  );
}

function GroupHeader({
  group,
  items,
  bytes: total,
}: {
  group: string;
  items: Target[];
  bytes: number;
}) {
  const selected = useStore((s) => s.selected);
  const setMany = useStore((s) => s.setMany);
  const marcados = items.filter((t) => selected.has(t.id)).length;
  const todos = marcados === items.length;

  return (
    <div className="flex items-center gap-3 px-4">
      <Checkbox
        checked={todos ? true : marcados > 0 ? "indeterminate" : false}
        onCheckedChange={() => setMany(items.map((t) => t.id), !todos)}
        onClick={(e) => e.stopPropagation()}
        aria-label={`Marcar tudo em ${group}`}
      />
      <AccordionTrigger className="flex-1 py-4 hover:no-underline">
        <div className="flex flex-1 items-center gap-3 pr-3">
          <span className="font-medium">{group}</span>
          <span className="text-muted-foreground text-xs">
            {items.length} {items.length === 1 ? "item" : "itens"}
            {marcados > 0 && ` · ${marcados} marcado${marcados > 1 ? "s" : ""}`}
          </span>
          <span className="ml-auto font-mono text-sm tabular-nums text-muted-foreground">
            {bytes(total)}
          </span>
        </div>
      </AccordionTrigger>
    </div>
  );
}

function Row({ target: t }: { target: Target }) {
  const marcado = useStore((s) => s.selected.has(t.id));
  const toggle = useStore((s) => s.toggle);
  const risco = RISK[t.risk];

  return (
    <label
      className={`flex cursor-pointer gap-3 px-4 py-3 transition-colors ${
        marcado ? "bg-primary/5" : "hover:bg-muted/40"
      }`}
    >
      <Checkbox checked={marcado} onCheckedChange={() => toggle(t.id)} className="mt-1" />

      <div className="min-w-0 flex-1">
        <div className="flex items-baseline gap-2">
          <span className="truncate font-medium text-sm">{t.label}</span>
          <Tooltip>
            <TooltipTrigger asChild>
              <span
                className={`shrink-0 rounded-full border px-2 py-0.5 text-[10px] font-medium ${risco.chip}`}
              >
                {risco.label}
              </span>
            </TooltipTrigger>
            <TooltipContent>{risco.hint}</TooltipContent>
          </Tooltip>
          {t.recommended && (
            <Tooltip>
              <TooltipTrigger asChild>
                <Sparkles className="size-3 shrink-0 text-muted-foreground" />
              </TooltipTrigger>
              <TooltipContent>Entra no botão "Marcar recomendados".</TooltipContent>
            </Tooltip>
          )}
          <span className="ml-auto shrink-0 font-mono text-sm tabular-nums">{bytes(t.bytes)}</span>
        </div>

        <p className="mt-1 text-xs leading-relaxed text-muted-foreground break-words">{t.detail}</p>
        <p className="mt-1 flex items-start gap-1.5 text-xs text-muted-foreground/70">
          <CornerDownRight className="mt-0.5 size-3 shrink-0" />
          <span>{t.recovery}</span>
        </p>
      </div>
    </label>
  );
}
