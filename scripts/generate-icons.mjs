import sharp from 'sharp';
import { execFileSync } from 'child_process';
import { existsSync, mkdirSync, readFileSync, rmSync, writeFileSync } from 'fs';
import { join, dirname } from 'path';
import { fileURLToPath } from 'url';

const __dirname = dirname(fileURLToPath(import.meta.url));
const root = join(__dirname, '..');
const iconsDir = join(root, 'src-tauri', 'icons');
const svgPath = join(iconsDir, 'boule.svg');

const sizes = [
  { name: '32x32.png', size: 32 },
  { name: '64x64.png', size: 64 },
  { name: '128x128.png', size: 128 },
  { name: '128x128@2x.png', size: 256 },
  { name: 'icon.png', size: 512 },
  { name: 'Square30x30Logo.png', size: 30 },
  { name: 'Square44x44Logo.png', size: 44 },
  { name: 'Square71x71Logo.png', size: 71 },
  { name: 'Square89x89Logo.png', size: 89 },
  { name: 'Square107x107Logo.png', size: 107 },
  { name: 'Square142x142Logo.png', size: 142 },
  { name: 'Square150x150Logo.png', size: 150 },
  { name: 'Square284x284Logo.png', size: 284 },
  { name: 'Square310x310Logo.png', size: 310 },
  { name: 'StoreLogo.png', size: 50 },
];

// iOS app icons. Listed separately only because they live in a subdirectory;
// they are regenerated with everything else so the repo never holds two
// different versions of the logo.
const iosSizes = [
  { name: 'AppIcon-20x20@1x.png', size: 20 },
  { name: 'AppIcon-20x20@2x.png', size: 40 },
  { name: 'AppIcon-20x20@2x-1.png', size: 40 },
  { name: 'AppIcon-20x20@3x.png', size: 60 },
  { name: 'AppIcon-29x29@1x.png', size: 29 },
  { name: 'AppIcon-29x29@2x.png', size: 58 },
  { name: 'AppIcon-29x29@2x-1.png', size: 58 },
  { name: 'AppIcon-29x29@3x.png', size: 87 },
  { name: 'AppIcon-40x40@1x.png', size: 40 },
  { name: 'AppIcon-40x40@2x.png', size: 80 },
  { name: 'AppIcon-40x40@2x-1.png', size: 80 },
  { name: 'AppIcon-40x40@3x.png', size: 120 },
  { name: 'AppIcon-60x60@2x.png', size: 120 },
  { name: 'AppIcon-60x60@3x.png', size: 180 },
  { name: 'AppIcon-76x76@1x.png', size: 76 },
  { name: 'AppIcon-76x76@2x.png', size: 152 },
  { name: 'AppIcon-83.5x83.5@2x.png', size: 167 },
  { name: 'AppIcon-512@2x.png', size: 1024 },
];

// Android buckets: the legacy launcher icon, and the adaptive foreground,
// which is drawn on a 108dp canvas of which only the middle 72dp survives
// masking.
const androidDensities = [
  { dir: 'mipmap-mdpi', legacy: 48, foreground: 108 },
  { dir: 'mipmap-hdpi', legacy: 72, foreground: 162 },
  { dir: 'mipmap-xhdpi', legacy: 96, foreground: 216 },
  { dir: 'mipmap-xxhdpi', legacy: 144, foreground: 324 },
  { dir: 'mipmap-xxxhdpi', legacy: 192, foreground: 432 },
];

const svgSource = readFileSync(svgPath, 'utf8');

// Below this, the LDM mark stops resolving as letters and reads as a smudge on
// the boule. Checked by rendering: legible at 128, soft at 120, gone by 96.
const MARK_MIN_PX = 128;

// Two masters from one file, so the mark is defined in exactly one place.
const withMark = Buffer.from(svgSource);
const withoutMark = Buffer.from(svgSource.replace(/\n?\s*<g id="maker-mark"[\s\S]*?<\/g>\n/, '\n'));
if (withoutMark.length >= withMark.length) {
  throw new Error('Could not strip #maker-mark from boule.svg - check the group markup');
}

// Rasterise at 4x the target and let sharp average it down. Going straight to
// a 16px canvas drops the thin stripes entirely in places; supersampling keeps
// them as a faint line, which is what the eye wants at that size. The SVG's
// viewBox is 512 units, so 72dpi is 1:1.
//
// `size` is the size the ARTWORK is drawn at, not the canvas: the adaptive
// Android foreground insets the boule to 58% of a 216px canvas, which puts it
// well under the threshold even though the file is large.
const render = (size) =>
  sharp(size >= MARK_MIN_PX ? withMark : withoutMark, {
    density: (72 * Math.min(size * 4, 2048)) / 512,
  }).resize(size, size);

// The logo scaled to `fraction` of a transparent square of `size`. Used where
// a mask will eat the edges and the art has to keep clear of them.
async function inset(size, fraction) {
  const art = Math.round(size * fraction);
  const offset = Math.round((size - art) / 2);
  return sharp({ create: { width: size, height: size, channels: 4, background: { r: 0, g: 0, b: 0, alpha: 0 } } })
    .composite([{ input: await render(art).png().toBuffer(), left: offset, top: offset }])
    .png()
    .toBuffer();
}

async function generateIcons() {
  for (const { name, size } of sizes) {
    await render(size).png().toFile(join(iconsDir, name));
    console.log(`Generated ${name} (${size}x${size})`);
  }

  // The web build's favicon comes from the same vector, so the browser tab and
  // the app icon can never drift apart. It goes out without the mark: a favicon
  // is drawn at 16-32px, where the mark is a smudge and nothing more.
  writeFileSync(join(root, 'public', 'favicon.svg'), withoutMark);
  console.log('Generated public/favicon.svg (mark stripped)');

  // The generated Xcode project keeps its own copy of the iOS icons in an
  // asset catalog. `tauri ios init` seeds that catalog from Tauri's default
  // template, not from this directory, so without writing both the app ships
  // the stock Tauri logo on iPad.
  const iosCatalogDir = join(
    root, 'src-tauri', 'gen', 'apple',
    'Assets.xcassets', 'AppIcon.appiconset'
  );
  const hasCatalog = existsSync(iosCatalogDir);

  // iOS icons must be opaque: the system flattens any alpha (to black) and
  // App Store Connect rejects icons with an alpha channel outright. The
  // desktop icons keep their transparency, which is correct for those.
  for (const { name, size } of iosSizes) {
    const png = await render(size).flatten({ background: '#ffffff' }).png().toBuffer();
    writeFileSync(join(iconsDir, 'ios', name), png);
    if (hasCatalog) {
      writeFileSync(join(iosCatalogDir, name), png);
    }
    console.log(`Generated ios/${name} (${size}x${size})${hasCatalog ? ' + asset catalog' : ''}`);
  }

  await generateAndroid();
  await generateIco();
  await generateIcns();
}

async function generateAndroid() {
  // The art is 400x460 in a 512 box, so its half-diagonal is 0.595 of the box.
  // To survive a circular mask of radius size/2 the box may be at most
  // 1/(2*0.595) = 0.84 of the icon; the adaptive foreground must additionally
  // fit the 72/108 safe zone, hence the tighter fraction.
  const ROUND = 0.8;
  const ADAPTIVE = 0.58;

  for (const { dir, legacy, foreground } of androidDensities) {
    const target = join(iconsDir, 'android', dir);
    mkdirSync(target, { recursive: true });

    await render(legacy).png().toFile(join(target, 'ic_launcher.png'));

    // The round icon is presented already masked, so it carries its own
    // opaque disc rather than relying on a background layer.
    const disc = Buffer.from(
      `<svg xmlns="http://www.w3.org/2000/svg" width="${legacy}" height="${legacy}">` +
      `<circle cx="${legacy / 2}" cy="${legacy / 2}" r="${legacy / 2}" fill="#ffffff"/></svg>`
    );
    await sharp(disc)
      .composite([{ input: await inset(legacy, ROUND) }])
      .png()
      .toFile(join(target, 'ic_launcher_round.png'));

    writeFileSync(join(target, 'ic_launcher_foreground.png'), await inset(foreground, ADAPTIVE));
    console.log(`Generated android/${dir} (${legacy}, foreground ${foreground})`);
  }
}

// A real Windows .ico: an ICONDIR, one 16-byte ICONDIRENTRY per image, then
// the PNG payloads. Shipping a bare PNG under the .ico extension happens to
// work in some tools and not others.
async function generateIco() {
  const icoSizes = [16, 24, 32, 48, 64, 128, 256];
  const images = [];
  for (const size of icoSizes) {
    images.push({ size, data: await render(size).png().toBuffer() });
  }

  const header = Buffer.alloc(6);
  header.writeUInt16LE(0, 0); // reserved
  header.writeUInt16LE(1, 2); // 1 = icon
  header.writeUInt16LE(images.length, 4);

  const directory = Buffer.alloc(16 * images.length);
  let offset = header.length + directory.length;
  images.forEach(({ size, data }, i) => {
    const e = i * 16;
    directory.writeUInt8(size >= 256 ? 0 : size, e); // 0 means 256
    directory.writeUInt8(size >= 256 ? 0 : size, e + 1);
    directory.writeUInt8(0, e + 2); // palette size
    directory.writeUInt8(0, e + 3); // reserved
    directory.writeUInt16LE(1, e + 4); // colour planes
    directory.writeUInt16LE(32, e + 6); // bits per pixel
    directory.writeUInt32LE(data.length, e + 8);
    directory.writeUInt32LE(offset, e + 12);
    offset += data.length;
  });

  writeFileSync(
    join(iconsDir, 'icon.ico'),
    Buffer.concat([header, directory, ...images.map((i) => i.data)])
  );
  console.log(`Generated icon.ico (${icoSizes.join(', ')})`);
}

// A real multi-resolution .icns, via the iconutil that ships with macOS.
// Elsewhere, leave the existing file alone rather than write a PNG in its
// place: Tauri's bundler will re-encode a PNG, but only into a single 256px
// entry, so Finder has nothing sharp to draw at other sizes.
async function generateIcns() {
  const out = join(iconsDir, 'icon.icns');
  if (process.platform !== 'darwin') {
    console.log('Skipped icon.icns (iconutil is macOS-only)');
    return;
  }

  const iconset = join(iconsDir, 'icon.iconset');
  rmSync(iconset, { recursive: true, force: true });
  mkdirSync(iconset, { recursive: true });

  const entries = [
    ['icon_16x16.png', 16], ['icon_16x16@2x.png', 32],
    ['icon_32x32.png', 32], ['icon_32x32@2x.png', 64],
    ['icon_128x128.png', 128], ['icon_128x128@2x.png', 256],
    ['icon_256x256.png', 256], ['icon_256x256@2x.png', 512],
    ['icon_512x512.png', 512], ['icon_512x512@2x.png', 1024],
  ];
  for (const [name, size] of entries) {
    await render(size).png().toFile(join(iconset, name));
  }

  execFileSync('iconutil', ['-c', 'icns', iconset, '-o', out]);
  rmSync(iconset, { recursive: true, force: true });
  console.log(`Generated icon.icns (${entries.length} representations)`);
}

generateIcons().catch((err) => {
  console.error(err);
  process.exit(1);
});
