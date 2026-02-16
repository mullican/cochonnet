import { useForm, useFieldArray } from 'react-hook-form';
import { useTranslation } from 'react-i18next';
import { Button, Input, Select, SelectItem, Card, CardContent, CardFooter } from '../../components/ui';
import type { TournamentFormData } from '../../types';

interface TournamentFormProps {
  defaultValues?: Partial<TournamentFormData>;
  onSubmit: (data: TournamentFormData) => void;
  onCancel: () => void;
  isLoading?: boolean;
}

export function TournamentForm({ defaultValues, onSubmit, onCancel, isLoading }: TournamentFormProps) {
  const { t } = useTranslation();

  const {
    register,
    handleSubmit,
    watch,
    setValue,
    formState: { errors },
    control,
  } = useForm<TournamentFormData>({
    mode: 'onBlur',
    defaultValues: {
      name: '',
      teamComposition: 'mixed',
      type: 'regional',
      startDate: new Date().toISOString().split('T')[0],
      endDate: new Date().toISOString().split('T')[0],
      director: '',
      headUmpire: '',
      additionalUmpires: [],
      format: 'triple',
      numberOfCourts: 8,
      numberOfQualifyingRounds: 5,
      hasConsolante: false,
      advanceAll: false,
      advanceCount: null,
      bracketSize: 16,
      pairingMethod: 'swiss',
      regionAvoidance: false,
      ...defaultValues,
    },
  });

  const { fields, append, remove } = useFieldArray({
    control,
    name: 'additionalUmpires',
  });

  const validateRequired = (value: string) => {
    if (!value || value.trim() === '') {
      return t('validation.required');
    }
    return true;
  };

  return (
    <form onSubmit={handleSubmit(onSubmit)} className="space-y-6">
      {/* Tournament Setup Section */}
      <Card>
        <CardContent className="space-y-4 pt-6">
          <h3 className="font-semibold text-gray-900">{t('tournaments.setup')}</h3>

          {/* Name row + Type/Composition/Format row */}
          <div className="grid gap-4 grid-cols-1 sm:grid-cols-4">
            <Input
              label={t('tournaments.name')}
              {...register('name', { validate: validateRequired })}
              error={errors.name?.message}
            />

            <Select
              label={t('tournaments.type')}
              value={watch('type')}
              onValueChange={(v) => setValue('type', v as TournamentFormData['type'])}
            >
              <SelectItem value="regional">{t('tournaments.typeOptions.regional')}</SelectItem>
              <SelectItem value="national">{t('tournaments.typeOptions.national')}</SelectItem>
              <SelectItem value="open">{t('tournaments.typeOptions.open')}</SelectItem>
              <SelectItem value="club">{t('tournaments.typeOptions.club')}</SelectItem>
            </Select>

            <Select
              label={t('tournaments.teamComposition')}
              value={watch('teamComposition')}
              onValueChange={(v) => setValue('teamComposition', v as TournamentFormData['teamComposition'])}
            >
              <SelectItem value="men">{t('tournaments.teamCompositionOptions.men')}</SelectItem>
              <SelectItem value="women">{t('tournaments.teamCompositionOptions.women')}</SelectItem>
              <SelectItem value="mixed">{t('tournaments.teamCompositionOptions.mixed')}</SelectItem>
              <SelectItem value="select">{t('tournaments.teamCompositionOptions.select')}</SelectItem>
            </Select>

            <Select
              label={t('tournaments.format')}
              value={watch('format')}
              onValueChange={(v) => setValue('format', v as TournamentFormData['format'])}
            >
              <SelectItem value="single">{t('tournaments.formatOptions.single')}</SelectItem>
              <SelectItem value="double">{t('tournaments.formatOptions.double')}</SelectItem>
              <SelectItem value="triple">{t('tournaments.formatOptions.triple')}</SelectItem>
            </Select>
          </div>

          {/* Dates, Rounds, Courts row */}
          <div className="grid gap-4 grid-cols-2 sm:grid-cols-4">
            <Input
              type="date"
              label={t('tournaments.startDate')}
              {...register('startDate', { validate: validateRequired })}
              error={errors.startDate?.message}
            />

            <Input
              type="date"
              label={t('tournaments.endDate')}
              {...register('endDate', { validate: validateRequired })}
              error={errors.endDate?.message}
            />

            <Input
              type="number"
              min={1}
              max={20}
              label={t('tournaments.numberOfQualifyingRounds')}
              {...register('numberOfQualifyingRounds', { valueAsNumber: true })}
              error={errors.numberOfQualifyingRounds?.message}
              disabled={watch('pairingMethod') === 'poolPlay'}
            />

            <Input
              type="number"
              min={1}
              label={t('tournaments.numberOfCourts')}
              {...register('numberOfCourts', { valueAsNumber: true })}
              error={errors.numberOfCourts?.message}
            />
          </div>

          {/* Umpire Information */}
          <div className="grid gap-4 grid-cols-1 sm:grid-cols-2 pt-2">
            <Input
              label={t('tournaments.director')}
              {...register('director', { validate: validateRequired })}
              error={errors.director?.message}
            />

            <Input
              label={t('tournaments.headUmpire')}
              {...register('headUmpire', { validate: validateRequired })}
              error={errors.headUmpire?.message}
            />
          </div>

          <div className="space-y-2">
            <label className="text-sm font-medium text-gray-700">
              {t('tournaments.additionalUmpires')}
            </label>
            {fields.map((field, index) => (
              <div key={field.id} className="flex gap-2">
                <Input
                  {...register(`additionalUmpires.${index}.value` as const)}
                  className="flex-1"
                />
                <Button
                  type="button"
                  variant="ghost"
                  onClick={() => remove(index)}
                >
                  {t('common.remove')}
                </Button>
              </div>
            ))}
            <Button
              type="button"
              variant="secondary"
              size="sm"
              onClick={() => append({ value: '' })}
            >
              {t('common.add')}
            </Button>
          </div>
        </CardContent>
      </Card>

      {/* Competition Structure Section */}
      <Card>
        <CardContent className="space-y-4 pt-6">
          <h3 className="font-semibold text-gray-900">{t('tournaments.competitionStructure')}</h3>

          <div className="grid gap-4 grid-cols-2 sm:grid-cols-4 items-end">
            <Select
              label={t('tournaments.pairingMethod')}
              value={watch('pairingMethod')}
              onValueChange={(v) => {
                setValue('pairingMethod', v as TournamentFormData['pairingMethod']);
                // Pool Play is fixed at 3 rounds
                if (v === 'poolPlay') {
                  setValue('numberOfQualifyingRounds', 3);
                }
              }}
            >
              <SelectItem value="swiss">{t('tournaments.pairingMethodOptions.swiss')}</SelectItem>
              <SelectItem value="swissHotel">{t('tournaments.pairingMethodOptions.swissHotel')}</SelectItem>
              <SelectItem value="roundRobin">{t('tournaments.pairingMethodOptions.roundRobin')}</SelectItem>
              <SelectItem value="poolPlay">{t('tournaments.pairingMethodOptions.poolPlay')}</SelectItem>
            </Select>

            <Select
              label={t('tournaments.bracketSize')}
              value={String(watch('bracketSize'))}
              onValueChange={(v) => setValue('bracketSize', parseInt(v))}
            >
              <SelectItem value="4">4</SelectItem>
              <SelectItem value="8">8</SelectItem>
              <SelectItem value="16">16</SelectItem>
              <SelectItem value="32">32</SelectItem>
            </Select>

            <div className="flex items-center gap-2 pb-2">
              <input
                type="checkbox"
                id="hasConsolante"
                {...register('hasConsolante')}
                className="h-4 w-4 rounded border-gray-300 text-primary-600 focus:ring-primary-500"
              />
              <label htmlFor="hasConsolante" className="text-sm text-gray-700">
                {t('tournaments.consolante')}
              </label>
            </div>
          </div>

          <div className="pt-4">
            <h4 className="text-sm font-medium text-gray-700 mb-3">{t('tournaments.ameliaIslandOptions')}</h4>
            <div className="flex flex-wrap gap-x-8 gap-y-2">
              <div className="flex items-center gap-2">
                <input
                  type="checkbox"
                  id="regionAvoidance"
                  {...register('regionAvoidance')}
                  className="h-4 w-4 rounded border-gray-300 text-primary-600 focus:ring-primary-500"
                />
                <label htmlFor="regionAvoidance" className="text-sm text-gray-700">
                  {t('tournaments.regionAvoidance')}
                </label>
              </div>

              <div className="flex items-center gap-2">
                <input
                  type="checkbox"
                  id="advanceAll"
                  {...register('advanceAll')}
                  className="h-4 w-4 rounded border-gray-300 text-primary-600 focus:ring-primary-500"
                />
                <label htmlFor="advanceAll" className="text-sm text-gray-700">
                  {t('tournaments.advanceAllLabel')}
                </label>
              </div>
            </div>
          </div>
        </CardContent>

        <CardFooter className="flex justify-end gap-4">
          <Button type="button" variant="secondary" onClick={onCancel}>
            {t('common.cancel')}
          </Button>
          <Button type="submit" disabled={isLoading}>
            {isLoading ? t('common.loading') : t('common.save')}
          </Button>
        </CardFooter>
      </Card>
    </form>
  );
}
