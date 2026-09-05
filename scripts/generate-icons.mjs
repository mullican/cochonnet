import sharp from 'sharp';
import { existsSync, readFileSync, writeFileSync } from 'fs';
import { join, dirname } from 'path';
import { fileURLToPath } from 'url';

const __dirname = dirname(fileURLToPath(import.meta.url));
const iconsDir = join(__dirname, '..', 'src-tauri', 'icons');
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

async function generateIcons() {
  const svgBuffer = readFileSync(svgPath);

  for (const { name, size } of sizes) {
    const outputPath = join(iconsDir, name);
    await sharp(svgBuffer)
      .resize(size, size)
      .png()
      .toFile(outputPath);
    console.log(`Generated ${name} (${size}x${size})`);
  }

  // The generated Xcode project keeps its own copy of the iOS icons in an
  // asset catalog. `tauri ios init` seeds that catalog from Tauri's default
  // template, not from this directory, so without writing both the app ships
  // the stock Tauri logo on iPad.
  const iosCatalogDir = join(
    __dirname, '..', 'src-tauri', 'gen', 'apple',
    'Assets.xcassets', 'AppIcon.appiconset'
  );
  const hasCatalog = existsSync(iosCatalogDir);

  // iOS icons must be opaque: the system flattens any alpha (to black) and
  // App Store Connect rejects icons with an alpha channel outright. The
  // desktop icons keep their transparency, which is correct for those.
  for (const { name, size } of iosSizes) {
    const png = await sharp(svgBuffer)
      .resize(size, size)
      .flatten({ background: '#ffffff' })
      .png()
      .toBuffer();
    writeFileSync(join(iconsDir, 'ios', name), png);
    if (hasCatalog) {
      writeFileSync(join(iosCatalogDir, name), png);
    }
    console.log(`Generated ios/${name} (${size}x${size})${hasCatalog ? ' + asset catalog' : ''}`);
  }

  // Generate ICO file (Windows) - use 256x256 as the main size
  const icoSizes = [16, 32, 48, 256];
  const icoImages = await Promise.all(
    icoSizes.map(size =>
      sharp(svgBuffer)
        .resize(size, size)
        .png()
        .toBuffer()
    )
  );

  // For ICO, we'll just use the 256x256 PNG as a simple solution
  // A proper ICO would need ico-endec or similar library
  const ico256 = await sharp(svgBuffer).resize(256, 256).png().toBuffer();
  writeFileSync(join(iconsDir, 'icon.ico'), ico256);
  console.log('Generated icon.ico (256x256 PNG format)');

  // For ICNS (macOS), we'll use the 512x512 PNG
  const icns512 = await sharp(svgBuffer).resize(512, 512).png().toBuffer();
  writeFileSync(join(iconsDir, 'icon.icns'), icns512);
  console.log('Generated icon.icns (512x512 PNG format)');

  console.log('\nNote: For proper ICO and ICNS files, consider using:');
  console.log('  - png2icons or icns-lib for macOS .icns');
  console.log('  - png-to-ico for Windows .ico');
}

generateIcons().catch(console.error);
