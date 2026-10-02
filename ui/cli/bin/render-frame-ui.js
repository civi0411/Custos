import fs from 'node:fs';
import path from 'node:path';
import jpeg from 'jpeg-js';
import { PNG } from 'pngjs';
import { fileURLToPath } from 'node:url';

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);

export function loadImage(filePath) {
  const buf = fs.readFileSync(filePath);
  const isJpg = buf[0] === 0xff && buf[1] === 0xd8;

  if (isJpg) {
    const raw = jpeg.decode(buf, { useTArray: true });
    return {
      width: raw.width,
      height: raw.height,
      data: raw.data, // RGBA Uint8Array
    };
  } else {
    const png = PNG.sync.read(buf);
    return {
      width: png.width,
      height: png.height,
      data: png.data, // RGBA Buffer
    };
  }
}

export function processAndRasterize(img, targetW, targetH, sharpenStrength = 0.35) {
  const w = img.width;
  const h = img.height;
  const data = img.data;

  // 1. Flood fill outer background from border pixels
  const visited = new Uint8Array(w * h);
  const queue = [];

  const isBgPixel = (x, y) => {
    const idx = (y * w + x) * 4;
    return data[idx] > 230 && data[idx + 1] > 230 && data[idx + 2] > 230;
  };

  for (let x = 0; x < w; x++) {
    if (isBgPixel(x, 0)) {
      visited[x] = 1;
      queue.push(x, 0);
    }
    const bY = h - 1;
    if (isBgPixel(x, bY)) {
      visited[bY * w + x] = 1;
      queue.push(x, bY);
    }
  }

  for (let y = 0; y < h; y++) {
    if (isBgPixel(0, y) && !visited[y * w]) {
      visited[y * w] = 1;
      queue.push(0, y);
    }
    const rX = w - 1;
    if (isBgPixel(rX, y) && !visited[y * w + rX]) {
      visited[y * w + rX] = 1;
      queue.push(rX, y);
    }
  }

  let head = 0;
  while (head < queue.length) {
    const cx = queue[head++];
    const cy = queue[head++];
    for (const [dx, dy] of [[-1, 0], [1, 0], [0, -1], [0, 1]]) {
      const nx = cx + dx;
      const ny = cy + dy;
      if (nx >= 0 && nx < w && ny >= 0 && ny < h) {
        const nPos = ny * w + nx;
        if (!visited[nPos]) {
          const idx = nPos * 4;
          if (data[idx] > 218 && data[idx + 1] > 218 && data[idx + 2] > 218) {
            visited[nPos] = 1;
            queue.push(nx, ny);
          }
        }
      }
    }
  }

  for (let i = 0; i < visited.length; i++) {
    if (visited[i]) {
      data[i * 4 + 3] = 0; // Transparent
    }
  }

  // 2. Autocrop
  let minX = w, minY = h, maxX = 0, maxY = 0;
  for (let y = 0; y < h; y++) {
    for (let x = 0; x < w; x++) {
      if (data[(y * w + x) * 4 + 3] > 0) {
        if (x < minX) minX = x;
        if (y < minY) minY = y;
        if (x > maxX) maxX = x;
        if (y > maxY) maxY = y;
      }
    }
  }

  const pad = 2;
  minX = Math.max(0, minX - pad);
  minY = Math.max(0, minY - pad);
  maxX = Math.min(w - 1, maxX + pad);
  maxY = Math.min(h - 1, maxY + pad);
  const cropW = Math.max(1, maxX - minX + 1);
  const cropH = Math.max(1, maxY - minY + 1);

  // 3. High-Quality Area-Averaged Downsample
  // Grid buffer: targetW x targetH (RGBA)
  const gridR = new Float32Array(targetW * targetH);
  const gridG = new Float32Array(targetW * targetH);
  const gridB = new Float32Array(targetW * targetH);
  const gridA = new Float32Array(targetW * targetH);

  for (let ty = 0; ty < targetH; ty++) {
    const sy0 = minY + (ty / targetH) * cropH;
    const sy1 = minY + ((ty + 1) / targetH) * cropH;
    const iy0 = Math.floor(sy0);
    const iy1 = Math.min(h - 1, Math.ceil(sy1));

    for (let tx = 0; tx < targetW; tx++) {
      const sx0 = minX + (tx / targetW) * cropW;
      const sx1 = minX + ((tx + 1) / targetW) * cropW;
      const ix0 = Math.floor(sx0);
      const ix1 = Math.min(w - 1, Math.ceil(sx1));

      let sumR = 0, sumG = 0, sumB = 0;
      let count = 0;
      let totalAlpha = 0;
      let totalSamples = 0;

      for (let y = iy0; y <= iy1; y++) {
        for (let x = ix0; x <= ix1; x++) {
          const idx = (y * w + x) * 4;
          const a = data[idx + 3];
          totalSamples++;
          totalAlpha += a;
          if (a > 30) {
            sumR += data[idx];
            sumG += data[idx + 1];
            sumB += data[idx + 2];
            count++;
          }
        }
      }

      const gIdx = ty * targetW + tx;
      if (count > 0 && totalAlpha / totalSamples > 40) {
        gridR[gIdx] = sumR / count;
        gridG[gIdx] = sumG / count;
        gridB[gIdx] = sumB / count;
        gridA[gIdx] = 255;
      } else {
        gridA[gIdx] = 0;
      }
    }
  }

  // 4. Sharpening Filter
  const sharpR = new Uint8Array(targetW * targetH);
  const sharpG = new Uint8Array(targetW * targetH);
  const sharpB = new Uint8Array(targetW * targetH);
  const sharpA = new Uint8Array(targetW * targetH);

  for (let ty = 0; ty < targetH; ty++) {
    for (let tx = 0; tx < targetW; tx++) {
      const idx = ty * targetW + tx;
      const a = gridA[idx];
      sharpA[idx] = a > 0 ? 255 : 0;
      if (a === 0) continue;

      const top = ty > 0 && gridA[idx - targetW] > 0 ? (idx - targetW) : idx;
      const bot = ty < targetH - 1 && gridA[idx + targetW] > 0 ? (idx + targetW) : idx;
      const left = tx > 0 && gridA[idx - 1] > 0 ? (idx - 1) : idx;
      const right = tx < targetW - 1 && gridA[idx + 1] > 0 ? (idx + 1) : idx;

      for (const [grid, sharp] of [[gridR, sharpR], [gridG, sharpG], [gridB, sharpB]]) {
        const center = grid[idx];
        const lap = 4 * center - (grid[top] + grid[bot] + grid[left] + grid[right]);
        const val = Math.min(255, Math.max(0, Math.round(center + sharpenStrength * lap)));
        sharp[idx] = val;
      }
    }
  }

  // 5. Half-block ANSI rasterization
  const outRows = [];
  for (let r = 0; r < targetH; r += 2) {
    let line = '';
    for (let c = 0; c < targetW; c++) {
      const idx1 = r * targetW + c;
      const a1 = sharpA[idx1] > 0;
      const r1 = sharpR[idx1];
      const g1 = sharpG[idx1];
      const b1 = sharpB[idx1];

      const idx2 = (r + 1) * targetW + c;
      const hasBot = (r + 1 < targetH);
      const a2 = hasBot && sharpA[idx2] > 0;
      const r2 = hasBot ? sharpR[idx2] : 0;
      const g2 = hasBot ? sharpG[idx2] : 0;
      const b2 = hasBot ? sharpB[idx2] : 0;

      if (!a1 && !a2) {
        line += ' ';
      } else if (a1 && !a2) {
        line += `\x1b[38;2;${r1};${g1};${b1}m▀\x1b[0m`;
      } else if (!a1 && a2) {
        line += `\x1b[38;2;${r2};${g2};${b2}m▄\x1b[0m`;
      } else {
        line += `\x1b[38;2;${r1};${g1};${b1}m\x1b[48;2;${r2};${g2};${b2}m▀\x1b[0m`;
      }
    }
    outRows.push(line);
  }

  return outRows;
}

export function rasterizeFullBanner(img, targetW = 96, targetH = 46) {
  const w = img.width;
  const h = img.height;
  const data = img.data;

  const outRows = [];

  for (let r = 0; r < targetH; r += 2) {
    let line = '';
    for (let c = 0; c < targetW; c++) {
      // Top pixel
      const sx1 = Math.floor((c / targetW) * w);
      const sy1 = Math.floor((r / targetH) * h);
      const idx1 = (sy1 * w + sx1) * 4;
      const r1 = data[idx1], g1 = data[idx1 + 1], b1 = data[idx1 + 2];
      const vis1 = r1 > 20 || g1 > 20 || b1 > 20;

      // Bottom pixel
      const hasBot = r + 1 < targetH;
      const sx2 = Math.floor((c / targetW) * w);
      const sy2 = Math.floor(((r + 1) / targetH) * h);
      const idx2 = (sy2 * w + sx2) * 4;
      const r2 = hasBot ? data[idx2] : 0;
      const g2 = hasBot ? data[idx2 + 1] : 0;
      const b2 = hasBot ? data[idx2 + 2] : 0;
      const vis2 = hasBot && (r2 > 20 || g2 > 20 || b2 > 20);

      if (!vis1 && !vis2) {
        line += ' ';
      } else if (vis1 && !vis2) {
        line += `\x1b[38;2;${r1};${g1};${b1}m▀\x1b[0m`;
      } else if (!vis1 && vis2) {
        line += `\x1b[38;2;${r2};${g2};${b2}m▄\x1b[0m`;
      } else {
        line += `\x1b[38;2;${r1};${g1};${b1}m\x1b[48;2;${r2};${g2};${b2}m▀\x1b[0m`;
      }
    }
    outRows.push(line);
  }

  return outRows;
}

export function buildAllMascots() {
  const frameDir = path.resolve(__dirname, '../frame-ui');

  const assets = {
    guardian: { file: 'owl.png', w: 38, h: 46 },
    coder: { file: 'custos-owl-coder.png', w: 20, h: 24 },
    inspector: { file: 'custos-owl-inspector.png', w: 20, h: 24 },
    steward: { file: 'custos-owl-steward.png', w: 20, h: 24 },
  };

  const results = {};

  // Check if banner-main.png exists
  const bannerMainPath = path.join(frameDir, 'banner-main.png');
  if (fs.existsSync(bannerMainPath)) {
    console.log('Processing banner-main.png directly...');
    const bannerImg = loadImage(bannerMainPath);
    const bannerLines = rasterizeFullBanner(bannerImg, 96, 46);
    results.bannerFull = bannerLines;
    console.log(` -> Generated ${bannerLines.length} lines for bannerFull (w=96, h=46)`);
  }

  for (const [key, cfg] of Object.entries(assets)) {
    const fullPath = path.join(frameDir, cfg.file);
    if (!fs.existsSync(fullPath)) {
      console.warn(`Asset not found: ${fullPath}`);
      continue;
    }
    console.log(`Processing ${key} from ${cfg.file}...`);
    const img = loadImage(fullPath);
    const lines = processAndRasterize(img, cfg.w, cfg.h);
    results[key] = lines;
    console.log(` -> Generated ${lines.length} lines for ${key} (w=${cfg.w}, h=${cfg.h})`);
  }

  const cachePath = path.join(__dirname, 'mascots-cache.json');
  fs.writeFileSync(cachePath, JSON.stringify(results, null, 2), 'utf8');
  console.log(`Saved all mascots to ${cachePath}`);
  return results;
}

if (process.argv[1]?.endsWith('render-frame-ui.js')) {
  buildAllMascots();
}
