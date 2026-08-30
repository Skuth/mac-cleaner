import { create } from "zustand";
import { invoke } from "@tauri-apps/api/core";

export type Risk = "safe" | "low" | "medium";

export type Target = {
  id: string;
  group: string;
  label: string;
  detail: string;
  recovery: string;
  risk: Risk;
  bytes: number;
  recommended: boolean;
};

export type Disk = { total: number; free: number; used: number };

export type CleanResult = {
  id: string;
  label: string;
  freed: number;
  error: string | null;
};

type Progress = { done: number; total: number; label: string };

export type Theme = "system" | "light" | "dark";

const THEME_KEY = "mac-cleaner:theme";

/** Liga/desliga a classe `.dark`, que é o que o Tailwind observa. */
export function applyTheme(t: Theme) {
  const escuro =
    t === "dark" ||
    (t === "system" && matchMedia("(prefers-color-scheme: dark)").matches);
  document.documentElement.classList.toggle("dark", escuro);
  // Faz scrollbar e campos nativos acompanharem o tema.
  document.documentElement.style.colorScheme = escuro ? "dark" : "light";
}

type State = {
  targets: Target[];
  disk: Disk | null;
  selected: Set<string>;
  scanning: boolean;
  cleaning: boolean;
  progress: Progress | null;
  results: CleanResult[] | null;
  freedNow: number;
  /** Quanto a limpeza em curso pretende liberar — a seleção é zerada no meio. */
  cleanTarget: number;
  /** Muda a cada scan. Serve de `key` pra remontar a lista com alturas novas. */
  scanId: number;
  theme: Theme;

  setTheme: (t: Theme) => void;
  scan: () => Promise<void>;
  clean: () => Promise<void>;
  toggle: (id: string) => void;
  setMany: (ids: string[], on: boolean) => void;
  selectRecommended: () => void;
  clearSelection: () => void;
  dismissResults: () => void;
  setProgress: (p: Progress | null) => void;
};

export const useStore = create<State>((set, get) => ({
  targets: [],
  disk: null,
  selected: new Set(),
  scanning: false,
  cleaning: false,
  progress: null,
  results: null,
  freedNow: 0,
  cleanTarget: 0,
  scanId: 0,
  theme: (localStorage.getItem(THEME_KEY) as Theme | null) ?? "system",

  setTheme: (theme) => {
    localStorage.setItem(THEME_KEY, theme);
    applyTheme(theme);
    set({ theme });
  },

  scan: async () => {
    set({ scanning: true, progress: null });
    try {
      const [targets, disk] = await Promise.all([
        invoke<Target[]>("scan"),
        invoke<Disk>("disk_usage"),
      ]);
      // Uma seleção antiga apontando pra alvo que sumiu viraria erro no clean.
      const vivos = new Set(targets.map((t) => t.id));
      const selected = new Set([...get().selected].filter((id) => vivos.has(id)));
      set({ targets, disk, selected, scanId: get().scanId + 1 });
    } finally {
      set({ scanning: false, progress: null });
    }
  },

  clean: async () => {
    const { targets, selected } = get();
    const escolhidos = targets.filter((t) => selected.has(t.id));
    const items = escolhidos.map((t) => [t.id, t.label] as [string, string]);
    if (!items.length) return;

    set({
      cleaning: true,
      results: null,
      cleanTarget: escolhidos.reduce((a, t) => a + t.bytes, 0),
      progress: { done: 0, total: items.length, label: escolhidos[0].label },
    });
    try {
      const results = await invoke<CleanResult[]>("clean", { items });
      const freedNow = results.reduce((a, r) => a + r.freed, 0);
      set({ results, freedNow, selected: new Set() });
      await get().scan();
    } finally {
      set({ cleaning: false, progress: null });
    }
  },

  toggle: (id) =>
    set((s) => {
      const next = new Set(s.selected);
      next.has(id) ? next.delete(id) : next.add(id);
      return { selected: next };
    }),

  setMany: (ids, on) =>
    set((s) => {
      const next = new Set(s.selected);
      ids.forEach((id) => (on ? next.add(id) : next.delete(id)));
      return { selected: next };
    }),

  // Substitui a seleção em vez de somar: o botão é um atalho pra um estado
  // conhecido, não mais um acumulador em cima do que já estava marcado.
  selectRecommended: () =>
    set((s) => ({
      selected: new Set(s.targets.filter((t) => t.recommended).map((t) => t.id)),
    })),

  clearSelection: () => set({ selected: new Set() }),
  dismissResults: () => set({ results: null }),
  setProgress: (progress) => set({ progress }),
}));

// --- seletores derivados ---------------------------------------------------

export const selectedBytes = (s: State) =>
  s.targets.reduce((a, t) => (s.selected.has(t.id) ? a + t.bytes : a), 0);

export const totalBytes = (s: State) => s.targets.reduce((a, t) => a + t.bytes, 0);

export const recommendedBytes = (s: State) =>
  s.targets.reduce((a, t) => (t.recommended ? a + t.bytes : a), 0);

/** Agrupa preservando a ordem de tamanho que o backend já devolveu. */
export const byGroup = (targets: Target[]) => {
  const map = new Map<string, Target[]>();
  for (const t of targets) {
    const list = map.get(t.group) ?? [];
    list.push(t);
    map.set(t.group, list);
  }
  return [...map.entries()]
    .map(([group, items]) => ({
      group,
      items,
      bytes: items.reduce((a, t) => a + t.bytes, 0),
    }))
    .sort((a, b) => b.bytes - a.bytes);
};
