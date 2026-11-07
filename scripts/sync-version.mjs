// Keeps the app's version the same in every file that states one.
//
//   node scripts/sync-version.mjs           copy package.json's version to the rest
//   node scripts/sync-version.mjs 1.6.0     set 1.6.0 everywhere, package.json included
//   node scripts/sync-version.mjs --check   change nothing; fail if any file disagrees
//
// A release is made by tagging: `git tag v1.6.0` on a clean main. The
// reference-transaction hook in .githooks/ sees the new tag, runs the second
// form, commits the result and moves the tag onto that commit. `npm version`
// reaches the same place through package.json's "version" hook, which runs the
// first form. The release workflow runs --check, so a tag made any other way
// is refused rather than published under the wrong number.
//
// Each file keeps a literal version rather than pointing at package.json:
// tauri.conf.json is what the macOS About panel and the installers read, and
// Cargo.toml / Cargo.lock are what cargo checks the other two against.
import { readFileSync, writeFileSync } from 'fs';
import { join, dirname } from 'path';
import { fileURLToPath } from 'url';

const root = join(dirname(fileURLToPath(import.meta.url)), '..');
const args = process.argv.slice(2);
const check = args.includes('--check');
const requested = args.find((arg) => !arg.startsWith('--'));

if (requested && !/^\d+\.\d+\.\d+$/.test(requested)) {
  console.error(`Not a version: ${requested}. Expected something like 1.6.0.`);
  process.exit(1);
}

const read = (file) => readFileSync(join(root, file), 'utf8');
const version = requested ?? JSON.parse(read('package.json')).version;

/** Rewrites a JSON file, in npm's own two-space layout. */
const json = (edit) => (text) => {
  const data = JSON.parse(text);
  edit(data);
  return JSON.stringify(data, null, 2) + '\n';
};

const targets = [
  {
    file: 'package.json',
    update: json((data) => {
      data.version = version;
    }),
  },
  {
    file: 'package-lock.json',
    update: json((data) => {
      data.version = version;
      if (data.packages?.['']) data.packages[''].version = version;
    }),
  },
  {
    file: 'src-tauri/tauri.conf.json',
    update: json((data) => {
      data.version = version;
    }),
  },
  {
    // The first `version =` line is the [package] one.
    file: 'src-tauri/Cargo.toml',
    update: (text) => text.replace(/^version = "[^"]*"$/m, `version = "${version}"`),
  },
  {
    file: 'src-tauri/Cargo.lock',
    update: (text) =>
      text.replace(
        /(\nname = "petanque-tournament"\nversion = ")[^"]*"/,
        `$1${version}"`
      ),
  },
];

const stale = [];
for (const { file, update } of targets) {
  const before = read(file);
  const after = update(before);
  if (after === before) continue;
  if (check) stale.push(file);
  else writeFileSync(join(root, file), after);
}

if (stale.length > 0) {
  console.error(
    `Not at version ${version}: ${stale.join(', ')}. Run: node scripts/sync-version.mjs`
  );
  process.exit(1);
}
