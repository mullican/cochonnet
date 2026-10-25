import { useEffect, useState } from 'react';
import { useTranslation } from 'react-i18next';
import type { TFunction } from 'i18next';
import { useTournamentStore } from '../../stores/tournamentStore';
import {
  Button,
  Card,
  CardContent,
  CardHeader,
  CardTitle,
  Input,
  Select,
  SelectItem,
} from '../../components/ui';
import type { CourtDemand, CourtPin, CourtPlan } from '../../types';
import { parseCourtList, type CourtListError } from './courtRanges';

interface CourtPlanViewProps {
  tournamentId: string;
}

const EMPTY_PLAN: CourtPlan = {
  auxiliary: '',
  spreadAuxiliary: true,
  qualifyingRounds: [],
  eliminationRounds: [],
  pins: [],
};

type RowKey = 'qualifyingRounds' | 'eliminationRounds';

/** Final, semifinals, quarterfinals: the rounds a featured court is chosen for. */
const PIN_STAGES = [0, 1, 2];

/** Pads or trims a row list to the rows on screen. */
const fitRows = (rows: string[], count: number) =>
  Array.from({ length: count }, (_, i) => rows[i] ?? '');

/** A pin's courts, one per match in its round: one for a final, two for semifinals. */
const pinSlots = (pin: CourtPin) =>
  Array.from({ length: 1 << pin.stage }, (_, j) => (pin.courts.split(',')[j] ?? '').trim());

const samePin = (a: CourtPin, b: CourtPin) =>
  a.bracket.trim().toUpperCase() === b.bracket.trim().toUpperCase() && a.stage === b.stage;

/** "Semi-Final" for a match `fromEnd` rounds before its bracket's last. */
function stageName(t: TFunction, fromEnd: number): string {
  if (fromEnd === 0) return t('brackets.final');
  if (fromEnd === 1) return t('brackets.semiFinal');
  return t('brackets.quarterFinal');
}

function describeError(t: TFunction, error: CourtListError): string {
  return t(`courts.errors.${error.key}`, error as unknown as Record<string, unknown>);
}

export function CourtPlanView({ tournamentId }: CourtPlanViewProps) {
  const { t } = useTranslation();
  const { currentTournament, fetchCourtPlan, fetchCourtDemand, saveCourtPlan } =
    useTournamentStore();

  const [plan, setPlan] = useState<CourtPlan | null>(null);
  const [demand, setDemand] = useState<CourtDemand | null>(null);
  const [savedJson, setSavedJson] = useState('');
  const [loadError, setLoadError] = useState<string | null>(null);
  const [saveError, setSaveError] = useState<string | null>(null);
  const [saving, setSaving] = useState(false);
  const [justSaved, setJustSaved] = useState(false);

  const isPanache = currentTournament?.pairingMethod === 'panache';
  const numberOfCourts = currentTournament?.numberOfCourts ?? 0;
  const qualifyingCount = currentTournament?.numberOfQualifyingRounds ?? 0;
  // A wave past the last row keeps that row, so a short guess costs nothing.
  const eliminationCount = isPanache
    ? 0
    : demand?.waveCount || Math.max(1, Math.log2(currentTournament?.bracketSize ?? 16));

  useEffect(() => {
    let cancelled = false;
    Promise.all([fetchCourtPlan(tournamentId), fetchCourtDemand(tournamentId)])
      .then(([loaded, needed]) => {
        if (cancelled) return;
        const merged = { ...EMPTY_PLAN, ...loaded };
        setPlan(merged);
        setSavedJson(JSON.stringify(merged));
        setDemand(needed);
      })
      .catch((error) => !cancelled && setLoadError(String(error)));
    return () => {
      cancelled = true;
    };
  }, [tournamentId, fetchCourtPlan, fetchCourtDemand]);

  if (loadError) {
    return <div className="rounded-md bg-red-50 p-4 text-sm text-red-700">{loadError}</div>;
  }
  if (!plan || !demand || !currentTournament) {
    return <div className="text-center py-8 text-gray-500">{t('common.loading')}</div>;
  }

  // The rows on screen. What is saved is exactly this, so a row the operator
  // cannot see can never quietly steer a draw.
  const qualifyingRows = fitRows(plan.qualifyingRounds, qualifyingCount);
  const eliminationRows = fitRows(plan.eliminationRounds, eliminationCount);
  const toSave: CourtPlan = {
    ...plan,
    qualifyingRounds: qualifyingRows,
    eliminationRounds: isPanache ? [] : eliminationRows,
    pins: isPanache ? [] : plan.pins,
  };

  const parse = (text: string) => parseCourtList(text, numberOfCourts);
  const bracketOptions = demand.brackets;
  // A new row starts on the first match not featured yet, not on a duplicate.
  const nextPin = bracketOptions
    .flatMap((bracket) => PIN_STAGES.map((stage) => ({ bracket, stage, courts: '' })))
    .find((candidate) => !plan.pins.some((p) => samePin(p, candidate)));
  const isInvalid = (text: string) => 'error' in parse(text);

  // What is wrong with each of a pin's courts, if anything. A blank one leaves
  // a match with nowhere to go.
  const slotErrors = (slots: string[]): (string | null)[] =>
    slots.map((slot, j) => {
      if (slot === '') return t('courts.pinNeedsCourts');
      const result = parse(slot);
      if ('error' in result) return describeError(t, result.error);
      if (slots.findIndex((other) => Number(other) === Number(slot)) < j) {
        return t('courts.courtTwice', { court: Number(slot) });
      }
      return null;
    });
  // The courts each row offers with its blanks filled in, as `ResolvedPlan`
  // reads them: one timeline from qualifying round 1 to the last wave, a blank
  // row being the row before, a blank first row the whole venue.
  let current = Array.from({ length: numberOfCourts }, (_, i) => i + 1);
  const offered = [...qualifyingRows, ...eliminationRows].map((row) => {
    const result = parse(row);
    if ('courts' in result && result.courts.length > 0) current = result.courts;
    return current;
  });
  // A row that offers fewer courts than its round has games, as
  // `check_court_capacity` refuses it. A featured court counts in its wave
  // even when the row leaves it out, because the draw offers it too.
  const shortfall = (key: RowKey, i: number): { games: number; count: number } | null => {
    const courts = new Set(offered[key === 'qualifyingRounds' ? i : qualifyingCount + i]);
    let games = demand.qualifying[i] ?? 0;
    if (key === 'eliminationRounds') {
      const wave = demand.elimination[i];
      games = wave?.matches ?? 0;
      for (const stage of wave?.stages ?? []) {
        const pin = toSave.pins.find((p) => samePin(p, { ...stage, courts: '' }));
        const pinned = pin ? parse(pin.courts) : null;
        if (pinned && 'courts' in pinned) pinned.courts.forEach((court) => courts.add(court));
      }
    }
    return games > courts.size ? { games, count: courts.size } : null;
  };

  // The later of two rows for the same match is the one marked.
  const isRepeatedPin = (i: number) => plan.pins.findIndex((p) => samePin(p, plan.pins[i])) < i;
  const auxiliary = parse(plan.auxiliary);
  const auxiliaryCount = 'courts' in auxiliary ? auxiliary.courts.length : 0;
  // With every court auxiliary there is no main court to prefer.
  const noMainCourts = auxiliaryCount > 0 && auxiliaryCount >= numberOfCourts;

  // The same checks `validate_court_plan` makes on save, so Save is only
  // offered for a plan the backend will take.
  // Every one of these is marked on its own field below.
  const fieldErrors =
    noMainCourts ||
    toSave.pins.some(
      (p, i) =>
        p.bracket.trim() === '' || isRepeatedPin(i) || slotErrors(pinSlots(p)).some(Boolean)
    ) ||
    [plan.auxiliary, ...toSave.qualifyingRounds, ...toSave.eliminationRounds].some(isInvalid) ||
    qualifyingRows.some((_, i) => shortfall('qualifyingRounds', i)) ||
    eliminationRows.some((_, i) => shortfall('eliminationRounds', i));
  const dirty = JSON.stringify(toSave) !== savedJson;

  const update = (changes: Partial<CourtPlan>) => {
    setPlan({ ...plan, ...changes });
    setJustSaved(false);
    setSaveError(null);
  };
  const setRow = (key: RowKey, rows: string[], i: number, value: string) => {
    const next = [...rows];
    next[i] = value;
    update({ [key]: next });
  };
  const setPin = (i: number, changes: Partial<CourtPin>) => {
    update({ pins: plan.pins.map((pin, j) => (j === i ? { ...pin, ...changes } : pin)) });
  };

  const handleSave = async () => {
    setSaving(true);
    setSaveError(null);
    try {
      await saveCourtPlan(tournamentId, toSave);
      setPlan(toSave);
      setSavedJson(JSON.stringify(toSave));
      setJustSaved(true);
    } catch (error) {
      setSaveError(String(error));
    } finally {
      setSaving(false);
    }
  };

  /** The line under a court list: what it resolves to, or what is wrong with it. */
  const preview = (text: string, blank: string) => {
    if (text.trim() === '') return <span className="text-gray-400">{blank}</span>;
    const result = parse(text);
    if ('error' in result) {
      return <span className="text-red-600">{describeError(t, result.error)}</span>;
    }
    return (
      <span className="text-gray-500">
        {t('courts.courtCount', { count: result.courts.length })}
      </span>
    );
  };

  const roundRow = (key: RowKey, rows: string[], i: number, label: string) => {
    const isFirst = i === 0 && (key === 'qualifyingRounds' || qualifyingCount === 0);
    const blank = isFirst ? t('courts.allCourts') : t('courts.sameAsAbove');
    const short = isInvalid(rows[i]) ? null : shortfall(key, i);
    return (
      <div key={`${key}-${i}`} className="grid gap-2 sm:grid-cols-[14rem_1fr_10rem] sm:items-center">
        <label htmlFor={`${key}-${i}`} className="text-sm text-gray-700">
          {label}
        </label>
        <Input
          id={`${key}-${i}`}
          value={rows[i]}
          placeholder={blank}
          aria-invalid={isInvalid(rows[i]) || short !== null}
          onChange={(e) => setRow(key, rows, i, e.target.value)}
        />
        <div className="text-xs">
          {short ? (
            <span className="text-red-600">{t('courts.tooFewCourts', short)}</span>
          ) : (
            preview(rows[i], blank)
          )}
        </div>
      </div>
    );
  };

  return (
    <div className="space-y-6">
      <p className="text-sm text-gray-600">{t('courts.intro')}</p>

      <Card>
        <CardHeader>
          <CardTitle>{t('courts.auxiliaryTitle')}</CardTitle>
        </CardHeader>
        <CardContent className="space-y-4">
          <p className="text-sm text-gray-600">{t('courts.auxiliaryDescription')}</p>
          <div className="grid gap-2 sm:grid-cols-[14rem_1fr_10rem] sm:items-center">
            <label htmlFor="auxiliary" className="text-sm text-gray-700">
              {t('courts.auxiliaryCourts')}
            </label>
            <Input
              id="auxiliary"
              value={plan.auxiliary}
              placeholder={t('courts.auxiliaryPlaceholder')}
              aria-invalid={'error' in auxiliary || noMainCourts}
              onChange={(e) => update({ auxiliary: e.target.value })}
            />
            <div className="text-xs">
              {'error' in auxiliary ? (
                <span className="text-red-600">{describeError(t, auxiliary.error)}</span>
              ) : noMainCourts ? (
                <span className="text-red-600">{t('courts.noMainCourts')}</span>
              ) : auxiliaryCount > 0 ? (
                <span className="text-gray-500">
                  {t('courts.auxiliarySplit', {
                    auxiliary: auxiliaryCount,
                    main: numberOfCourts - auxiliaryCount,
                  })}
                </span>
              ) : (
                <span className="text-gray-400">{t('courts.noAuxiliary')}</span>
              )}
            </div>
          </div>
          <div className="flex items-center gap-2">
            <input
              type="checkbox"
              id="spreadAuxiliary"
              checked={plan.spreadAuxiliary}
              onChange={(e) => update({ spreadAuxiliary: e.target.checked })}
              className="h-4 w-4 rounded border-gray-300 text-primary-600 focus:ring-primary-500"
            />
            <label htmlFor="spreadAuxiliary" className="text-sm text-gray-700">
              {isPanache ? t('courts.spreadAuxiliaryPlayers') : t('courts.spreadAuxiliary')}
            </label>
          </div>
        </CardContent>
      </Card>

      <Card>
        <CardHeader>
          <CardTitle>{t('courts.byRoundTitle')}</CardTitle>
        </CardHeader>
        <CardContent className="space-y-3">
          <p className="text-sm text-gray-600">{t('courts.byRoundDescription')}</p>
          {qualifyingRows.map((_, i) =>
            roundRow('qualifyingRounds', qualifyingRows, i, t('courts.qualifyingRound', { number: i + 1 }))
          )}
          {!isPanache && (
            <>
              <div className="border-t border-gray-100 pt-3" />
              {eliminationRows.map((_, i) => {
                // The last three rounds go by their names.
                const fromEnd = eliminationRows.length - 1 - i;
                const label = PIN_STAGES.includes(fromEnd)
                  ? stageName(t, fromEnd)
                  : t('courts.eliminationRound', { number: i + 1 });
                return roundRow('eliminationRounds', eliminationRows, i, label);
              })}
            </>
          )}
        </CardContent>
      </Card>

      {!isPanache && (
        <Card>
          <CardHeader>
            <CardTitle>{t('courts.featuredTitle')}</CardTitle>
          </CardHeader>
          <CardContent className="space-y-3">
            <p className="text-sm text-gray-600">{t('courts.featuredDescription')}</p>
            {plan.pins.map((pin, i) => {
              const slots = pinSlots(pin);
              const errors = slotErrors(slots);
              const message = errors.find(Boolean);
              const options = bracketOptions.includes(pin.bracket)
                ? bracketOptions
                : [...bracketOptions, pin.bracket];
              return (
                <div key={i} className="grid gap-2 sm:grid-cols-[9rem_12rem_1fr_auto] sm:items-start">
                  <Select
                    value={pin.bracket}
                    onValueChange={(bracket) => setPin(i, { bracket })}
                    error={pin.bracket.trim() === '' ? t('courts.bracketRequired') : undefined}
                  >
                    {options.map((name) => (
                      <SelectItem key={name} value={name}>
                        {bracketOptions.includes(name)
                          ? name
                          : t('courts.notInTournament', { bracket: name })}
                      </SelectItem>
                    ))}
                  </Select>
                  <Select
                    value={String(pin.stage)}
                    onValueChange={(value) => {
                      const stage = Number(value);
                      // Keep the courts already typed, as many as the new round has matches.
                      setPin(i, { stage, courts: pinSlots({ ...pin, stage }).join(',') });
                    }}
                    error={isRepeatedPin(i) ? t('courts.pinTwice') : undefined}
                  >
                    {PIN_STAGES.map((stage) => (
                      <SelectItem key={stage} value={String(stage)}>
                        {stageName(t, stage)}
                      </SelectItem>
                    ))}
                  </Select>
                  <div className="space-y-1">
                    <div className="flex flex-wrap gap-2">
                      {slots.map((slot, j) => (
                        <Input
                          key={j}
                          className="w-16"
                          inputMode="numeric"
                          aria-label={t('courts.pinCourt', { number: j + 1, count: slots.length })}
                          aria-invalid={errors[j] !== null}
                          value={slot}
                          onChange={(e) => {
                            const next = [...slots];
                            next[j] = e.target.value.replace(/\D/g, '');
                            setPin(i, { courts: next.join(',') });
                          }}
                        />
                      ))}
                    </div>
                    {message && <div className="text-xs text-red-600">{message}</div>}
                  </div>
                  <Button
                    variant="ghost"
                    onClick={() => update({ pins: plan.pins.filter((_, j) => j !== i) })}
                  >
                    {t('common.delete')}
                  </Button>
                </div>
              );
            })}
            <Button
              variant="secondary"
              disabled={!nextPin}
              onClick={() => nextPin && update({ pins: [...plan.pins, nextPin] })}
            >
              {t('courts.addFeatured')}
            </Button>
          </CardContent>
        </Card>
      )}

      {saveError && (
        <div className="rounded-md bg-red-50 p-4 text-sm text-red-700">{saveError}</div>
      )}

      <div className="flex items-center gap-3">
        <Button
          onClick={handleSave}
          disabled={!dirty || fieldErrors || saving}
        >
          {t('common.save')}
        </Button>
        {justSaved && !dirty && <span className="text-sm text-green-700">{t('courts.saved')}</span>}
        {fieldErrors && <span className="text-sm text-red-600">{t('courts.fixErrors')}</span>}
      </div>
    </div>
  );
}
