import { useEffect, useRef } from 'react';

/** The app ships on macOS, Windows and Linux, so the modifier is either. */
const isMac = typeof navigator !== 'undefined' && /Mac/i.test(navigator.userAgent);

/** How to write a Mod+Shift+<key> combo for this platform, for on-screen hints. */
export function comboLabel(letter: string): string {
  return isMac ? `⇧⌘${letter}` : `Ctrl+Shift+${letter}`;
}

/**
 * Fires `handler` on Mod+Shift+<code>, where Mod is Cmd or Ctrl.
 *
 * Matching is on `event.code` - the physical key - so it behaves the same on
 * the AZERTY keyboards this app's French half is used on.
 */
export function useShortcut(code: string, handler: () => void, enabled = true) {
  // Keeps the listener from being torn down and rebuilt on every render just
  // because the handler closure is new.
  const latest = useRef(handler);
  latest.current = handler;

  useEffect(() => {
    if (!enabled) return;

    const onKeyDown = (event: KeyboardEvent) => {
      if (!(event.metaKey || event.ctrlKey) || !event.shiftKey) return;
      if (event.code !== code) return;
      event.preventDefault();
      latest.current();
    };

    window.addEventListener('keydown', onKeyDown);
    return () => window.removeEventListener('keydown', onKeyDown);
  }, [code, enabled]);
}
