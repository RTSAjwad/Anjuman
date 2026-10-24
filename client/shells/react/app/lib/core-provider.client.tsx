import {
  createContext,
  useCallback,
  useContext,
  useEffect,
  useMemo,
  useRef,
  useState,
} from "react";
import type { ReactNode } from "react";
import { Core } from "./core.client";
import type { Event, ViewModel } from "shared_types/app";

// Single source of truth: the latest `ViewModel` from the core. Components read
// `view` and forward `Event`s via `update` — the Crux unidirectional invariant.
// No component-local mirrored state that can drift from the core.
interface CoreContextValue {
  view: ViewModel | null;
  update: (event: Event) => void;
}

const CoreContext = createContext<CoreContextValue | null>(null);

export function CoreProvider({ children }: { children: ReactNode }) {
  const [view, setView] = useState<ViewModel | null>(null);
  const coreRef = useRef<Core | null>(null);

  // Hold the Core in a ref so its identity is stable; it drives `setView`.
  if (coreRef.current === null) {
    coreRef.current = new Core(setView);
  }
  const core = coreRef.current;

  useEffect(() => {
    void core.initialize().then(() => {
      // First render arrives via the Core's first `view()` call; ensure state is
      // populated even if no event fires (initialize already calls setState).
      setView(core.view());
    });
  }, [core]);

  const update = useCallback(
    (event: Event) => {
      core.update(event);
    },
    [core],
  );

  const value = useMemo(() => ({ view, update }), [view, update]);

  return <CoreContext.Provider value={value}>{children}</CoreContext.Provider>;
}

// eslint-disable-next-line react-refresh/only-export-components
export function useCore(): CoreContextValue {
  const ctx = useContext(CoreContext);
  if (ctx === null) {
    throw new Error("useCore must be used within a CoreProvider");
  }
  return ctx;
}
