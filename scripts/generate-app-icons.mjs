import { execFileSync } from "node:child_process";
import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";
import zlib from "node:zlib";

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const iconDir = path.join(root, "src-tauri", "icons");
const iconsetDir = path.join(iconDir, "icon.iconset");

fs.mkdirSync(iconDir, { recursive: true });

const crcTable = new Uint32Array(256);
for (let n = 0; n < 256; n += 1) {
  let c = n;
  for (let k = 0; k < 8; k += 1) {
    c = c & 1 ? 0xedb88320 ^ (c >>> 1) : c >>> 1;
  }
  crcTable[n] = c >>> 0;
}

function crc32(buffer) {
  let c = 0xffffffff;
  for (const byte of buffer) {
    c = crcTable[(c ^ byte) & 0xff] ^ (c >>> 8);
  }
  return (c ^ 0xffffffff) >>> 0;
}

function pngChunk(type, data = Buffer.alloc(0)) {
  const typeBuffer = Buffer.from(type);
  const length = Buffer.alloc(4);
  const crc = Buffer.alloc(4);
  length.writeUInt32BE(data.length, 0);
  crc.writeUInt32BE(crc32(Buffer.concat([typeBuffer, data])), 0);
  return Buffer.concat([length, typeBuffer, data, crc]);
}

function writePng(file, width, height, rgba) {
  const header = Buffer.alloc(13);
  header.writeUInt32BE(width, 0);
  header.writeUInt32BE(height, 4);
  header[8] = 8;
  header[9] = 6;

  const stride = width * 4;
  const raw = Buffer.alloc((stride + 1) * height);
  for (let y = 0; y < height; y += 1) {
    raw[y * (stride + 1)] = 0;
    Buffer.from(rgba.buffer, y * stride, stride).copy(raw, y * (stride + 1) + 1);
  }

  fs.writeFileSync(
    file,
    Buffer.concat([
      Buffer.from([0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a]),
      pngChunk("IHDR", header),
      pngChunk("IDAT", zlib.deflateSync(raw, { level: 9 })),
      pngChunk("IEND"),
    ]),
  );
}

function hex(color) {
  return [
    Number.parseInt(color.slice(1, 3), 16),
    Number.parseInt(color.slice(3, 5), 16),
    Number.parseInt(color.slice(5, 7), 16),
  ];
}

function mix(a, b, t) {
  return [
    Math.round(a[0] + (b[0] - a[0]) * t),
    Math.round(a[1] + (b[1] - a[1]) * t),
    Math.round(a[2] + (b[2] - a[2]) * t),
  ];
}

function blendPixel(data, width, x, y, rgb, alpha) {
  if (alpha <= 0) return;
  const index = (y * width + x) * 4;
  const dstAlpha = data[index + 3] / 255;
  const srcAlpha = Math.min(1, Math.max(0, alpha));
  const outAlpha = srcAlpha + dstAlpha * (1 - srcAlpha);
  data[index] = Math.round((rgb[0] * srcAlpha + data[index] * dstAlpha * (1 - srcAlpha)) / outAlpha);
  data[index + 1] = Math.round((rgb[1] * srcAlpha + data[index + 1] * dstAlpha * (1 - srcAlpha)) / outAlpha);
  data[index + 2] = Math.round((rgb[2] * srcAlpha + data[index + 2] * dstAlpha * (1 - srcAlpha)) / outAlpha);
  data[index + 3] = Math.round(outAlpha * 255);
}

function insideRoundedRect(x, y, rect) {
  const { x: rx, y: ry, w, h, r } = rect;
  if (x < rx || y < ry || x > rx + w || y > ry + h) return false;
  const cx = x < rx + r ? rx + r : x > rx + w - r ? rx + w - r : x;
  const cy = y < ry + r ? ry + r : y > ry + h - r ? ry + h - r : y;
  const dx = x - cx;
  const dy = y - cy;
  return dx * dx + dy * dy <= r * r;
}

function pointInPolygon(x, y, points) {
  let inside = false;
  for (let i = 0, j = points.length - 1; i < points.length; j = i, i += 1) {
    const xi = points[i][0];
    const yi = points[i][1];
    const xj = points[j][0];
    const yj = points[j][1];
    const crosses = yi > y !== yj > y && x < ((xj - xi) * (y - yi)) / (yj - yi) + xi;
    if (crosses) inside = !inside;
  }
  return inside;
}

function drawRoundedRect(data, size, rect, top, bottom) {
  const samples = size < 128 ? 5 : size < 512 ? 4 : 3;
  const step = 1 / samples;
  const x0 = Math.floor(rect.x);
  const y0 = Math.floor(rect.y);
  const x1 = Math.ceil(rect.x + rect.w);
  const y1 = Math.ceil(rect.y + rect.h);

  for (let y = Math.max(0, y0); y < Math.min(size, y1); y += 1) {
    for (let x = Math.max(0, x0); x < Math.min(size, x1); x += 1) {
      let hits = 0;
      for (let sy = 0; sy < samples; sy += 1) {
        for (let sx = 0; sx < samples; sx += 1) {
          if (insideRoundedRect(x + (sx + 0.5) * step, y + (sy + 0.5) * step, rect)) {
            hits += 1;
          }
        }
      }
      if (!hits) continue;
      const t = (y - rect.y) / rect.h;
      blendPixel(data, size, x, y, mix(top, bottom, t), hits / (samples * samples));
    }
  }
}

function drawPolygon(data, size, sourcePoints, color, alpha = 1) {
  const samples = size < 128 ? 5 : size < 512 ? 4 : 3;
  const step = 1 / samples;
  const points = sourcePoints.map(([x, y]) => [(x / 1024) * size, (y / 1024) * size]);
  const xs = points.map(([x]) => x);
  const ys = points.map(([, y]) => y);
  const x0 = Math.floor(Math.min(...xs));
  const y0 = Math.floor(Math.min(...ys));
  const x1 = Math.ceil(Math.max(...xs));
  const y1 = Math.ceil(Math.max(...ys));

  for (let y = Math.max(0, y0); y < Math.min(size, y1); y += 1) {
    for (let x = Math.max(0, x0); x < Math.min(size, x1); x += 1) {
      let hits = 0;
      for (let sy = 0; sy < samples; sy += 1) {
        for (let sx = 0; sx < samples; sx += 1) {
          if (pointInPolygon(x + (sx + 0.5) * step, y + (sy + 0.5) * step, points)) {
            hits += 1;
          }
        }
      }
      if (hits) blendPixel(data, size, x, y, color, (hits / (samples * samples)) * alpha);
    }
  }
}

function render(size) {
  const data = new Uint8ClampedArray(size * size * 4);
  const s = size / 1024;
  const card = { x: 82 * s, y: 82 * s, w: 860 * s, h: 860 * s, r: 188 * s };

  drawRoundedRect(data, size, card, hex("#0b3438"), hex("#061b1f"));

  drawPolygon(
    data,
    size,
    [
      [298, 706],
      [394, 706],
      [394, 486],
      [512, 630],
      [630, 486],
      [630, 706],
      [726, 706],
      [726, 334],
      [626, 334],
      [512, 474],
      [398, 334],
      [298, 334],
    ],
    hex("#20e78f"),
  );

  drawPolygon(
    data,
    size,
    [
      [654, 368],
      [724, 296],
      [754, 326],
      [684, 398],
    ],
    hex("#38d8e8"),
  );
  drawPolygon(
    data,
    size,
    [
      [738, 266],
      [832, 266],
      [832, 360],
    ],
    hex("#38d8e8"),
  );

  return data;
}

const pngTargets = [
  ["32x32.png", 32],
  ["64x64.png", 64],
  ["icon.png", 32],
  ["128x128.png", 128],
  ["128x128@2x.png", 256],
  ["icon-512.png", 512],
  ["app-icon-source.png", 1024],
  ["app-icon.svg.png", 1024],
];

for (const [name, size] of pngTargets) {
  writePng(path.join(iconDir, name), size, size, render(size));
}

fs.writeFileSync(
  path.join(iconDir, "app-icon.svg"),
  `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 1024 1024">
  <defs>
    <linearGradient id="bg" x1="82" y1="82" x2="942" y2="942" gradientUnits="userSpaceOnUse">
      <stop offset="0" stop-color="#0b3438"/>
      <stop offset="1" stop-color="#061b1f"/>
    </linearGradient>
  </defs>
  <rect x="82" y="82" width="860" height="860" rx="188" fill="url(#bg)"/>
  <path d="M298 706V334h100l114 140 114-140h100v372h-96V486L512 630 394 486v220z" fill="#20e78f"/>
  <path d="m654 368 70-72 30 30-70 72z" fill="#38d8e8"/>
  <path d="M738 266h94v94z" fill="#38d8e8"/>
</svg>
`,
);

fs.rmSync(iconsetDir, { recursive: true, force: true });

execFileSync("npx", ["tauri", "icon", path.join(iconDir, "app-icon-source.png"), "-o", iconDir], {
  stdio: "inherit",
});

for (const [name, size] of pngTargets) {
  writePng(path.join(iconDir, name), size, size, render(size));
}
