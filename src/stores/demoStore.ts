import { create } from 'zustand';

interface DemoState {
  /**
   * Whether the score-filling shortcut is live.
   *
   * Nothing persists this - not localStorage, not the database - so it is off
   * at every launch and has to be armed deliberately. That is the point: the
   * fill shortcut writes invented scores, and this app is used to run real
   * tournaments, where a stray three-key press must not be able to find demo
   * mode already switched on.
   */
  armed: boolean;
  toggleArmed: () => void;
  /** What the last fill did, shown in the badge so the mode is never silent. */
  lastAction: string | null;
  report: (action: string) => void;
}

export const useDemoStore = create<DemoState>((set) => ({
  armed: false,
  toggleArmed: () => set((state) => ({ armed: !state.armed, lastAction: null })),
  lastAction: null,
  report: (action: string) => set({ lastAction: action }),
}));
