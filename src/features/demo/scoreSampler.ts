/**
 * Random but realistic pétanque scores, for walking an audience through a whole
 * tournament without typing several hundred results by hand.
 *
 * The weights below are not invented. They are the observed frequencies of the
 * 435 games of the 2024 Amelia Island Open - the same results workbook
 * `src-tauri/tests/fixtures/aio_2024.json` holds and the ranking test replays.
 * A game to 13 is the rule, but 12% of that field's games were still short when
 * time was called, so a generator that only ever produces 13-x looks subtly
 * wrong to anyone who has actually run one, and never exercises the ranking
 * code's handling of a low-scoring win.
 */

type Weighted = ReadonlyArray<readonly [value: number, count: number]>;

/** Winner's score. 382 of the 435 games reached 13; the rest ran out of time. */
const WINNER_SCORES: Weighted = [
  [7, 1],
  [8, 2],
  [9, 5],
  [10, 9],
  [11, 19],
  [12, 17],
  [13, 382],
];

/** Loser's score across the same 435 games: fairly flat to 7, then thinning. */
const LOSER_SCORES: Weighted = [
  [0, 37],
  [1, 41],
  [2, 37],
  [3, 41],
  [4, 36],
  [5, 46],
  [6, 42],
  [7, 36],
  [8, 29],
  [9, 32],
  [10, 23],
  [11, 22],
  [12, 13],
];

function sample(weights: Weighted): number {
  const total = weights.reduce((sum, [, count]) => sum + count, 0);
  let roll = Math.random() * total;
  for (const [value, count] of weights) {
    roll -= count;
    if (roll < 0) return value;
  }
  // Only reachable through floating-point drift on the last step.
  return weights[weights.length - 1][0];
}

/** A winning and a losing score, the winner's always the larger of the two. */
export function randomResult(): { winner: number; loser: number } {
  const winner = sample(WINNER_SCORES);
  // The loser's score is drawn from the same field but has to stay under the
  // winner's: a game called at 9 cannot have been lost 11.
  const loser = sample(LOSER_SCORES.filter(([score]) => score < winner));
  return { winner, loser };
}

/**
 * One result, assigned to the two sides at random so the demo does not show
 * every game won by the team listed first.
 */
export function randomScores(): { team1: number; team2: number } {
  const { winner, loser } = randomResult();
  return Math.random() < 0.5
    ? { team1: winner, team2: loser }
    : { team1: loser, team2: winner };
}
