# Pétanque Tournament Management

This application will be used to manage pétanque tournaments according to FPUSA rules with a few additional options.

Key references:
1. International Rules of Pétanque: https://www.usapetanque.org/uploads/3/4/9/7/34979601/official_rules_petanque-en.pdf
2. FPUSA tournament regulations: https://www.usapetanque.org/uploads/3/4/9/7/34979601/fpusa_tournament_regulations_-_2025_rev_12.3__1_.docx.pdf

## Requirements

### Architecture
- The app will be built with Tauri and TypeScript, buildable for MacOS and Windows
- The app needs to function without an Internet connection, using local data storage
- The app should be able to import a team registration list for a tournament in CSV format; it should provide a sample file with the columns it needs

### Design
- The app should have a clean, modern look and feel
- The user interface should allow selection of English (default) or French as its language

### Functional Requirements
- The app should allow the user to create, manage, update, and delete multiple tournaments
- A tournament can have several attributes which affect its management:
  * Name
  * Team composition (men, women, mixed, select)
  * Type (regional, national, club)
  * Date(s)
  * Director
  * Head umpire and additional umpires
  * Single-day vs two-day (refer to FPUSA regulations)
  * Single, double, or triple format
  * Number of courts (which limits the number of teams)
  * List of teams - each must have a captain identified whose name is the team name
  * Whether or not a "consolante" bracket will be played
  * Whether all teams will move to the elimination round, or only the top X teams (where X is 4, 8, 16, or 32)
- The app should assign team pairings (in qualifying rounds, and the first round of a bracket) and assign those paired games to courts for each round of play. Rounds are defined based on the tournament setup:
  * There are various methods of defining pairings in the qualifying rounds, found in the FPUSA regulation (round robin, swiss, etc)
  * A tournament may have one or more qualifying rounds, according to the FPUSA regulations
  * After the qualifying rounds, teams are ranked according to their performance (using a strict formula found in the rules) and placed into a bracket.  * Court assignments should be made randomly, with the following business logic:
    - In qualifying rounds, the same teams should not play each other twice
    - Teams should not play on the same court twice unless this is mathematically unavoidable
    - The app should support an optional region attribute for teams, and if enabled, avoid pairing teams from the same region in qualifying rounds
    - When there is an odd number of teams, the app should assign a BYE as needed and score that game according to the rules
- If the tournament has a consolante, then the losers of the first round of elimination play form a second bracket which should be scored independently of the main bracket
- The number of teams moving to the elimination round determines the size of the bracket. If all teams move to the elimination round, then an additional attribute is required to determine the size of the brackets. For example, if there are 90 teams playing, and the bracket size is 16, then there will be brackets as follows:
  * Top 16 teams = A bracket
  * Second 16 teams = B bracket
  * ...and so forth, until the F bracket which has only 10 teams and will require bye rounds
  * If this theoretical tournament had a consolante, then the 8 losers of the first round of the A bracket would form a consolante bracket called AA
- The app should allow the user to enter the score for each game played (from 0 to 13) and make assignments for the next round based on the scores entered
- The app should display a printable score/result sheet for each stage of the qualifying rounds
- The app should display a printable bracket for each stage of the elimination rounds
