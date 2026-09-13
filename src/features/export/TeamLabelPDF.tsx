import { Text, View, StyleSheet } from '@react-pdf/renderer';
import { formatTeamName } from '../../lib/utils';

type NumberedTeam = {
  teamNumber: number;
  captain: string | null | undefined;
};

/**
 * The print counterpart of the on-screen TeamNumberChip.
 *
 * @react-pdf has no `em`, so the proportions the web chip gets from relative
 * units are computed here from the caller's font size instead. The ratios are
 * the same ones: 0.85 for the number, 2.3 wide enough that one- and two-digit
 * chips match, and the gray-100 / gray-600 pair.
 *
 * Worth knowing before widening it: in a deep bracket the match box is under
 * 100pt wide, and every point the chip takes comes off the name beside it.
 */
export function teamLabelStyles(fontSize: number, compact = false) {
  const chipFont = fontSize * 0.85;
  return StyleSheet.create({
    row: {
      flexDirection: 'row',
      // Not 'baseline': react-pdf maps it to Yoga.Align.Baseline, but Yoga has
      // no text baseline to work from here and drops the chip below the name -
      // and below the last line, not the first, once a name wraps.
      alignItems: 'center',
    },
    chip: {
      // The min-width buys one thing: a one- and a two-digit chip come out the
      // same width, so names start on a common left edge down a column. A
      // bracket has no such column - each match box is its own island - and in
      // a 128 draw the box is ~94pt wide, where those spare points are the
      // difference between LAURENT M. and LAURENT... So compact drops it.
      minWidth: compact ? 0 : chipFont * 2.3,
      paddingHorizontal: chipFont * 0.45,
      paddingVertical: chipFont * 0.2,
      borderRadius: chipFont * 0.4,
      backgroundColor: '#f3f4f6',
      marginRight: fontSize * (compact ? 0.25 : 0.35),
    },
    chipText: {
      fontSize: chipFont,
      fontFamily: 'Helvetica-Bold',
      color: '#4b5563',
      textAlign: 'center',
    },
    name: {
      fontSize,
      flex: 1,
    },
  });
}

/**
 * A team's number in a chip, followed by its captain's name.
 *
 * `fallback` covers the cases the brackets need - an unfilled slot (TBD) or a
 * bye - which have no team and so no number to show.
 */
export function TeamLabelPDF({
  team,
  fontSize,
  fallback,
  compact = false,
  style,
  nameStyle,
}: {
  team: NumberedTeam | null | undefined;
  fontSize: number;
  fallback: string;
  /** Drop the number-aligning min-width where horizontal room is scarce. */
  compact?: boolean;
  style?: object | object[];
  nameStyle?: object | object[];
}) {
  const s = teamLabelStyles(fontSize, compact);
  // Callers pass either one style or a list of conditional ones. Splatting
  // keeps a list from arriving as a nested array, which react-pdf will not
  // flatten for us.
  const flatten = (v: object | object[] | undefined) => (Array.isArray(v) ? v : v ? [v] : []);

  if (!team) {
    return <Text style={[s.name, ...flatten(nameStyle)]}>{fallback}</Text>;
  }

  return (
    <View style={[s.row, ...flatten(style)]}>
      <View style={s.chip}>
        <Text style={s.chipText}>{team.teamNumber}</Text>
      </View>
      <Text style={[s.name, ...flatten(nameStyle)]}>{formatTeamName(team.captain)}</Text>
    </View>
  );
}
