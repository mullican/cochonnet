import sharp from 'sharp';
import { readFileSync, writeFileSync } from 'fs';
import { join, dirname } from 'path';
import { fileURLToPath } from 'url';

const __dirname = dirname(fileURLToPath(import.meta.url));
const iconsDir = join(__dirname, '..', 'src-tauri', 'icons');
const svgPath = join(iconsDir, 'boule.svg');

const sizes = [
  { name: '32x32.png', size: 32 },
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
