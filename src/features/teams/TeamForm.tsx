import { useForm } from 'react-hook-form';
import { useTranslation } from 'react-i18next';
import { Button, Input } from '../../components/ui';
import type { TeamFormData } from '../../types';

interface TeamFormProps {
  defaultValues?: Partial<TeamFormData>;
  showPlayer3?: boolean;
  /** Panache registers individuals: one name, no partners, plus a champion flag. */
  isPanache?: boolean;
  /** Only offered when editing: there is nothing to withdraw from on the way in. */
  showWithdrawn?: boolean;
  onSubmit: (data: TeamFormData) => void;
  onCancel: () => void;
}

export function TeamForm({
  defaultValues,
  showPlayer3 = true,
  isPanache = false,
  showWithdrawn = false,
  onSubmit,
  onCancel,
}: TeamFormProps) {
  const { t } = useTranslation();

  const {
    register,
    handleSubmit,
    formState: { errors },
  } = useForm<TeamFormData>({
    defaultValues: {
      teamNumber: '',
      captain: '',
      player2: '',
      player3: '',
      region: '',
      club: '',
      isChampion: false,
      isWithdrawn: false,
      ...defaultValues,
    },
  });

  const validateRequired = (value: string) => {
    if (!value || value.trim() === '') {
      return t('validation.required');
    }
    return true;
  };

  const validateTeamNumber = (value: string) => {
    if (!value || value.trim() === '') {
      return t('validation.required');
    }
    if (!/^\d+$/.test(value.trim()) || parseInt(value, 10) < 1) {
      return t('validation.positiveNumber');
    }
    return true;
  };

  return (
    <form onSubmit={handleSubmit(onSubmit)} className="space-y-4">
      <Input
        label={t('teams.teamNumber')}
        type="number"
        min={1}
        {...register('teamNumber', { validate: validateTeamNumber })}
        error={errors.teamNumber?.message}
      />

      <Input
        label={isPanache ? t('teams.player') : t('teams.captain')}
        {...register('captain', { validate: validateRequired })}
        error={errors.captain?.message}
      />

      {!isPanache && (
        <Input
          label={t('teams.player2')}
          {...register('player2', { validate: validateRequired })}
          error={errors.player2?.message}
        />
      )}

      {!isPanache && showPlayer3 && (
        <Input
          label={t('teams.player3')}
          {...register('player3')}
        />
      )}

      {isPanache && (
        <div className="flex items-center gap-2">
          <input
            type="checkbox"
            id="isChampion"
            {...register('isChampion')}
            className="h-4 w-4 rounded border-gray-300 text-primary-600 focus:ring-primary-500"
          />
          <label htmlFor="isChampion" className="text-sm text-gray-700">
            {t('teams.championHint')}
          </label>
        </div>
      )}

      <Input
        label={t('teams.region')}
        {...register('region')}
      />

      <Input
        label={t('teams.club')}
        {...register('club')}
      />

      {showWithdrawn && (
        <div className="flex items-start gap-2">
          <input
            type="checkbox"
            id="isWithdrawn"
            {...register('isWithdrawn')}
            className="mt-0.5 h-4 w-4 rounded border-gray-300 text-primary-600 focus:ring-primary-500"
          />
          <label htmlFor="isWithdrawn" className="text-sm text-gray-700">
            {t('teams.withdrawn')}
            <span className="block text-xs text-gray-500">{t('teams.withdrawnHint')}</span>
          </label>
        </div>
      )}

      <div className="flex justify-end gap-2 pt-4">
        <Button type="button" variant="secondary" onClick={onCancel}>
          {t('common.cancel')}
        </Button>
        <Button type="submit">{t('common.save')}</Button>
      </div>
    </form>
  );
}
