/**
 * Formats a team captain's name as "LASTNAME F."
 * e.g., "John Doe" -> "DOE J."
 */
export function formatTeamName(captainName: string | null | undefined): string {
  if (!captainName) return 'TBD';

  const parts = captainName.trim().split(/\s+/);
  if (parts.length === 0) return 'TBD';

  if (parts.length === 1) {
    // Single name - just uppercase it
    return parts[0].toUpperCase();
  }

  // Get first initial and last name
  const firstName = parts[0];
  const lastName = parts[parts.length - 1];
  const firstInitial = firstName.charAt(0).toUpperCase();

  return `${lastName.toUpperCase()} ${firstInitial}.`;
}

/**
 * Formats a team's number alongside its captain's name, e.g. "12 - DOE J."
 * The number is what teams use to report their scores, so it should be
 * shown wherever a team's name appears.
 */
export function formatTeamLabel(
  team: { teamNumber: number; captain: string | null | undefined } | null | undefined
): string {
  if (!team) return 'TBD';
  return `${team.teamNumber} - ${formatTeamName(team.captain)}`;
}

/**
 * Formats a panaché temporary team, e.g. "12 - DOE J. / 45 - MARTIN P."
 *
 * These teams are drawn fresh each round and have no number of their own, so
 * unlike formatTeamLabel there is nothing to prefix the side with. Each member
 * keeps the number they registered under, which is what score slips carry.
 */
export function formatPanacheSideLabel(
  side: { members: { teamNumber: number; captain: string | null | undefined }[] } | null | undefined
): string {
  if (!side || side.members.length === 0) return 'TBD';
  return side.members.map((m) => formatTeamLabel(m)).join(' / ');
}

/**
 * The label for one side of a qualifying game, whichever format produced it.
 * Panaché games carry a drawn side; every other format carries a registered team.
 */
export function formatGameSideLabel(
  side: { members: { teamNumber: number; captain: string | null | undefined }[] } | null | undefined,
  team: { teamNumber: number; captain: string | null | undefined } | null | undefined
): string {
  if (side) return formatPanacheSideLabel(side);
  return formatTeamLabel(team);
}
