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
