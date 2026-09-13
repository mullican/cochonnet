import { formatTeamName } from '../../lib/utils';

type NumberedTeam = {
  teamNumber: number;
  captain: string | null | undefined;
};

/**
 * A team's number, as a chip.
 *
 * Sized in `em` rather than a fixed `text-xs`, so the same chip reads correctly
 * beside a table row's small text and beside the large bold name on the winner
 * card, without a size prop at every call site.
 *
 * 2.3em is the narrowest min-width that renders a one- and a two-digit chip at
 * the same width, so a column of 3s and 14s starts its names on one line rather
 * than stepping in and out. Three digits still push wider; padding the column
 * for a 128 that may never exist would cost every other row.
 */
export function TeamNumberChip({ value }: { value: number }) {
  return (
    <span className="inline-block min-w-[2.3em] shrink-0 rounded-md bg-gray-100 px-[0.45em] py-[0.2em] text-center text-[0.85em] font-semibold tabular-nums text-gray-600">
      {value}
    </span>
  );
}

/**
 * A team's number alongside its captain's name - the number in a chip rather
 * than run together with the name as "12 - DOE J.".
 *
 * Aligned on the baseline, not the box: the chip carries its own padding, so
 * centring it would let the number drift off the line the name sits on.
 *
 * The plain-text `formatTeamLabel` in lib/utils stays for the PDF exports,
 * which render through @react-pdf and cannot use this.
 */
export function TeamLabel({
  team,
  className = '',
}: {
  team: NumberedTeam | null | undefined;
  className?: string;
}) {
  if (!team) return <span className={className}>TBD</span>;

  return (
    <span className={`inline-flex min-w-0 items-baseline gap-1.5 ${className}`}>
      <TeamNumberChip value={team.teamNumber} />
      <span className="min-w-0 truncate">{formatTeamName(team.captain)}</span>
    </span>
  );
}
