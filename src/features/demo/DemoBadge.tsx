import { useTranslation } from 'react-i18next';
import { useDemoStore } from '../../stores/demoStore';
import { comboLabel, useShortcut } from './useShortcut';

/** Arms and disarms demo mode. Rendered once, by the layout. */
export function DemoBadge() {
  const { t } = useTranslation();
  const { armed, toggleArmed, lastAction } = useDemoStore();

  useShortcut('KeyD', toggleArmed);

  if (!armed) return null;

  // Deliberately loud and always on top: demo mode invents scores, so it should
  // be impossible to be in it without noticing - including from the back of a
  // room, on a projector.
  return (
    <div className="fixed bottom-4 end-4 z-50 max-w-xs rounded-md bg-amber-500 px-3 py-2 text-white shadow-lg">
      <div className="text-xs font-bold uppercase tracking-wide">{t('demo.armed')}</div>
      <div className="mt-1 text-xs">
        {t('demo.fillHint', { combo: comboLabel('R') })}
      </div>
      <div className="text-xs opacity-90">
        {t('demo.exitHint', { combo: comboLabel('D') })}
      </div>
      {lastAction && (
        <div className="mt-1 border-t border-white/40 pt-1 text-xs font-medium">
          {lastAction}
        </div>
      )}
    </div>
  );
}
