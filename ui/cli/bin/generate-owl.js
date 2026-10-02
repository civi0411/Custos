import fs from 'node:fs';
import path from 'node:path';
import jpeg from 'jpeg-js';

export function renderPixelOwl(imagePath, targetW = 38, targetH = 46) {
  const buf = fs.readFileSync(imagePath);
  const raw = jpeg.decode(buf, { useTArray: true });
  const w = raw.width;
  const h = raw.height;
  const data = raw.data;

  // Flood fill outer background
  const visited = new Uint8Array(w * h);
  const queue = [];

  for (let x = 0; x < w; x++) {
    for (const y of [0, h - 1]) {
      const idx = (y * w + x) * 4;
      if (data[idx] > 230 && data[idx + 1] > 230 && data[idx + 2] > 230) {
        visited[y * w + x] = 1;
        queue.push(x, y);
      }
    }
  }
  for (let y = 0; y < h; y++) {
    for (const x of [0, w - 1]) {
      const idx = (y * w + x) * 4;
      if (data[idx] > 230 && data[idx + 1] > 230 && data[idx + 2] > 230) {
        if (!visited[y * w + x]) {
          visited[y * w + x] = 1;
          queue.push(x, y);
        }
      }
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
          if (data[idx] > 220 && data[idx + 1] > 220 && data[idx + 2] > 220) {
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

  // Autocrop
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

  const pad = 4;
  minX = Math.max(0, minX - pad);
  minY = Math.max(0, minY - pad);
  maxX = Math.min(w - 1, maxX + pad);
  maxY = Math.min(h - 1, maxY + pad);
  const cropW = maxX - minX + 1;
  const cropH = maxY - minY + 1;

  const outRows = [];

  for (let r = 0; r < targetH; r += 2) {
    let line = '';
    for (let c = 0; c < targetW; c++) {
      // Top pixel
      const srcX1 = Math.floor(minX + (c / targetW) * cropW);
      const srcY1 = Math.floor(minY + (r / targetH) * cropH);
      const idx1 = (srcY1 * w + srcX1) * 4;
      const a1 = data[idx1 + 3] >= 65;

      // Bottom pixel
      const srcX2 = Math.floor(minX + (c / targetW) * cropW);
      const srcY2 = Math.floor(minY + ((r + 1) / targetH) * cropH);
      const idx2 = (srcY2 * w + srcX2) * 4;
      const a2 = (r + 1 < targetH) && data[idx2 + 3] >= 65;

      if (!a1 && !a2) {
        line += ' ';
      } else if (a1 && !a2) {
        line += `\x1b[38;2;${data[idx1]};${data[idx1 + 1]};${data[idx1 + 2]}m▀\x1b[0m`;
      } else if (!a1 && a2) {
        line += `\x1b[38;2;${data[idx2]};${data[idx2 + 1]};${data[idx2 + 2]}m▄\x1b[0m`;
      } else {
        line += `\x1b[38;2;${data[idx1]};${data[idx1 + 1]};${data[idx1 + 2]}m\x1b[48;2;${data[idx2]};${data[idx2 + 1]};${data[idx2 + 2]}m▀\x1b[0m`;
      }
    }
    outRows.push(line);
  }

  return outRows;
}

// Test when run directly
if (process.argv[1]?.endsWith('generate-owl.js')) {
  const p = path.resolve('public/assets/owl.png');
  const rows = renderPixelOwl(p, 38, 46);
  console.log(`Rendered ${rows.length} rows:`);
  for (const row of rows) {
    console.log(row);
  }
}
