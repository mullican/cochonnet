import { useForm } from 'react-hook-form';
import { useTranslation } from 'react-i18next';
import { Button, Input } from '../../components/ui';

interface TeamFormData {
  captain: string;
  player2: string;
  player3: string;
  region: string;
  club: string;
}

interface TeamFormProps {
  defaultValues?: Partial<TeamFormData>;
  onSubmit: (data: TeamFormData) => void;
  onCancel: () => void;
}

export function TeamForm({ defaultValues, onSubmit, onCancel }: TeamFormProps) {
  const { t } = useTranslation();

  const {
    register,
    handleSubmit,
    formState: { errors },
  } = useForm<TeamFormData>({
    defaultValues: {
      captain: '',
      player2: '',
      player3: '',
      region: '',
      club: '',
      ...defaultValues,
    },
  });

  const validateRequired = (value: string) => {
    if (!value || value.trim() === '') {
      return t('validation.required');
    }
    return true;
  };

  return (
    <form onSubmit={handleSubmit(onSubmit)} className="space-y-4">
      <Input
        label={t('teams.captain')}
        {...register('captain', { validate: validateRequired })}
        error={errors.captain?.message}
      />

      <Input
        label={t('teams.player2')}
        {...register('player2', { validate: validateRequired })}
        error={errors.player2?.message}
      />

      <Input
        label={t('teams.player3')}
        {...register('player3')}
      />

      <Input
        label={t('teams.region')}
        {...register('region')}
      />

      <Input
        label={t('teams.club')}
        {...register('club')}
      />

      <div className="flex justify-end gap-2 pt-4">
        <Button type="button" variant="secondary" onClick={onCancel}>
          {t('common.cancel')}
        </Button>
        <Button type="submit">{t('common.save')}</Button>
      </div>
    </form>
  );
}
