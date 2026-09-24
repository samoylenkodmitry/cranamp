#!/usr/bin/env node
// Draws the rolled-up main window, Winamp's windowshade mode, into a skin
// open in Skin Studio: the 275x14 strip on TITLEBAR.BMP, focused and
// unfocused, the unroll button, the small seek track with its thumb, and the
// five cursors the rolled-up window reads.
//
//   node tools/skin-studio/windowshade.mjs <skin> [--replace] [--save]
//
// The strip follows Winamp's layout. Winamp draws its visualizer at 79,5,
// 38x5, over the art while music plays, so the name sits left of it and the
// visualizer gets a slot of its own in the skin's visualizer background, the
// way the time gets its window. --replace draws it again over a project that
// already has a Windowshade layer.
//
// Everything is drawn through Studio's own pen, on a paint layer of its own,
// from the skin's own pixels: the strip keeps the title bar's rim and corners
// (the options, minimize and close buttons are the title bar's cells, so the
// strip has to meet them), lifts the title's wordmark, and adds a time window
// in the font's own colours, the mini transport the player answers clicks
// on, and a seek slot. The rolled-up cursors are the skin's own full-window
// cursors for the same jobs, so the set stays one hand.
import { call, Pen } from './moonlit.mjs';

const MAIN_STRIP = [27, 29];
const MAIN_STRIP_UNFOCUSED = [27, 42];
const WIDTH = 275;
const HEIGHT = 14;

// Where the player puts things on the strip; see src/winamp/sprites.rs.
const TIME_WINDOW = [126, 3, 33, 8];
// Framed like the time window around the visualizer's field at 79,5, 38x5.
const VIS_SLOT = [78, 3, 40, 8];
// The panel the mini transport sits on where the art behind it is busy.
const TRANSPORT_PLATE = [167, 3, 58, 9];
const COLON_X = 142;
const TRANSPORT = {
  previous: [170, ['#...##', '#..###', '#.####', '#.####', '#..###', '#...##']],
  play: [180, ['##..', '###.', '####', '####', '###.', '##..']],
  pause: [189, ['##.##', '##.##', '##.##', '##.##', '##.##', '##.##']],
  stop: [199, ['....', '####', '####', '####', '####', '....']],
  next: [207, ['##...#', '###..#', '####.#', '####.#', '###..#', '##...#']],
  eject: [217, ['..##..', '.####.', '######', '......', '######', '......']],
};
const SHADE_CURSORS = [
  ['wsnormal', 'titlebar'],
  ['wsmin', 'min'],
  ['wswinbut', 'winbut'],
  ['wsclose', 'close'],
  ['wsposbar', 'posbar'],
];

async function lift(sheet, rect) {
  await call('studio_atlas', { sheet });
  const held = await call('studio_cluster', { rect });
  return held.rows.map(row => [...row].map(ch => held.palette[ch] ?? null));
}

function stamp(pixels) {
  const palette = {};
  const index = new Map();
  let next = 0xe000;
  const rows = pixels.map(row => row.map(colour => {
    if (colour == null) return ' ';
    if (!index.has(colour)) {
      const ch = String.fromCodePoint(next++);
      index.set(colour, ch);
      palette[ch] = colour;
    }
    return index.get(colour);
  }).join(''));
  return { rows, palette };
}

const grid = (width, height, colour = null) =>
  Array.from({ length: height }, () => Array(width).fill(colour));

function mode(colours) {
  const counts = new Map();
  for (const colour of colours) counts.set(colour, (counts.get(colour) ?? 0) + 1);
  return [...counts].sort((a, b) => b[1] - a[1])[0][0];
}

function put(target, x, y, rows, colour) {
  rows.forEach((row, dy) => [...row].forEach((ch, dx) => {
    if (ch === '#') target[y + dy][x + dx] = colour;
  }));
}

// The strip from a title row: its rim and corners, a clean body, and the
// window closed off underneath. `look` says which title columns are plain
// glass and where the wordmark is.
function strip(title, look, font, visGround) {
  const out = look.plain == null ? artStrip(title, look) : glassStrip(title, look);
  const [tx, ty, tw, th] = TIME_WINDOW;
  inset(out, tx, ty, tw, th, look.outline, font.ground);
  inset(out, ...VIS_SLOT, look.outline, visGround);
  out[5][COLON_X] = font.ink;
  out[7][COLON_X] = font.ink;
  if (look.plate) inset(out, ...TRANSPORT_PLATE, look.outline, font.ground);
  for (const [x, rows] of Object.values(TRANSPORT)) put(out, x, 5, rows, look.icon);
  return out;
}

function inset(out, x0, y0, width, height, edge, ground) {
  for (let y = y0; y < y0 + height; y++) {
    for (let x = x0; x < x0 + width; x++) {
      const border = y === y0 || y === y0 + height - 1 || x === x0 || x === x0 + width - 1;
      out[y][x] = border ? edge : ground;
    }
  }
}

// Silverplay's glass: a plain body under the title's rim, closed underneath.
function glassStrip(title, look) {
  const out = title.map(row => row.slice());
  const plain = title.map(row => row[look.plain]);
  const [capLeft, capRight] = look.caps;
  for (let y = 0; y < HEIGHT; y++) {
    for (let x = capLeft; x < WIDTH - capRight; x++) out[y][x] = plain[y];
  }
  // The window's underside: the title's top edge again, so the row the
  // focused and unfocused strips share holds the same pixels in both.
  const top = title[0];
  const rim = plain[look.rimRow];
  for (let x = capLeft; x < WIDTH - capRight; x++) {
    out[HEIGHT - 2][x] = rim;
    out[HEIGHT - 1][x] = top[x];
  }
  for (const [x, y, colour] of look.corners(title)) out[y][x] = colour;
  const [wx, wy, ww, wh] = look.wordmark;
  for (let y = 0; y < wh; y++) {
    for (let x = 0; x < ww; x++) {
      const colour = title[wy + y][wx + x];
      if (colour !== plain[wy + y]) out[look.wordmarkAt[1] + y][look.wordmarkAt[0] + x] = colour;
    }
  }
  look.ornament?.(out);
  return out;
}

// An illustrated title keeps its art: what would sit under the clock is
// moved aside, a name is lifted out of the art and set down on the left,
// and the window closes with the title's own top edge.
function artStrip(title, look) {
  const out = title.map(row => row.slice());
  // Stretches refilled from a clean slice of the same title, row for row.
  for (const [x0, x1, from, to] of look.fill ?? []) {
    for (let x = x0; x < x1; x++) {
      const source = from + ((x - x0) % (to - from));
      for (let y = 0; y < HEIGHT; y++) out[y][x] = title[y][source];
    }
  }
  // Letters taken out of the art: each ink pixel takes the nearest pixel
  // above or below it that is not ink.
  for (const { rect: [x0, y0, w, h], inks } of look.heal ?? []) {
    const ink = new Set(inks);
    for (let x = x0; x < x0 + w; x++) {
      for (let y = y0; y < y0 + h; y++) {
        if (!ink.has(title[y][x])) continue;
        for (let d = 1; d < HEIGHT; d++) {
          const above = title[y - d]?.[x];
          const below = title[y + d]?.[x];
          if (above != null && !ink.has(above)) { out[y][x] = above; break; }
          if (below != null && !ink.has(below)) { out[y][x] = below; break; }
        }
      }
    }
  }
  // Pieces of the title set down elsewhere: all of their pixels, or only
  // those in the given inks.
  for (const { rect: [x0, y0, w, h], to: [tx, ty], inks } of look.move ?? []) {
    const ink = inks && new Set(inks);
    for (let y = 0; y < h; y++) {
      for (let x = 0; x < w; x++) {
        const colour = title[y0 + y][x0 + x];
        if (!ink || ink.has(colour)) out[ty + y][tx + x] = colour;
      }
    }
  }
  out[HEIGHT - 1] = title[0].slice();
  look.ornament?.(out);
  return out;
}

// The unroll button: the shade button's own cell with its arrow turned over.
function unroll(cell) {
  const grounds = cell.map(row => mode(row));
  const glyph = [];
  cell.forEach((row, y) => row.forEach((colour, x) => {
    if (colour !== grounds[y]) glyph.push([x, y, colour]);
  }));
  const out = cell.map((row, y) => row.map(() => grounds[y]));
  const top = Math.min(...glyph.map(([, y]) => y));
  const bottom = Math.max(...glyph.map(([, y]) => y));
  for (const [x, y, colour] of glyph) out[top + bottom - y][x] = colour;
  return out;
}

function seekTrack(look, font) {
  const track = grid(17, 7, look.outline);
  for (let y = 1; y < 6; y++) for (let x = 1; x < 16; x++) track[y][x] = y === 5 ? look.slotLight : font.ground;
  return track;
}

function seekThumb(look) {
  return Array.from({ length: 7 }, (_, y) =>
    y === 0 || y === 6
      ? [look.thumbEdge, look.thumbBody, look.thumbEdge]
      : [look.thumbBody, look.thumbLight, look.thumbBody]);
}

const draw = (label, operations) => call('studio_draw', { label, operations, layers: [], states: 'current' });

async function shadeCursors() {
  // Cursors keep their transparency on the sheet itself, not a paint layer.
  await call('studio_targets', { paint_layer: null });
  await call('studio_cursors', { action: 'draw', regions: SHADE_CURSORS.map(([rolled]) => rolled) });
  const report = await call('studio_cursors', {});
  const hotspot = Object.fromEntries(report.regions.map(region => [region.region, region.hotspot]));
  for (const [rolled, full] of SHADE_CURSORS) {
    const pixels = await lift(`${full}.cur`, [0, 0, 32, 32]);
    await call('studio_atlas', { sheet: `${rolled}.cur` });
    const clear = { op: 'rect', x: 0, y: 0, width: 32, height: 32, color: '#00000000', fill: true };
    await draw(`${rolled} is ${full}`, [clear, { op: 'stamp', x: 0, y: 0, ...stamp(pixels) }]);
    await call('studio_cursors', { action: 'hotspot', regions: [rolled], hotspot: hotspot[full] });
  }
}

export async function windowshade(look) {
  const status = await call('studio_status', {});
  if (status.paint_layers.layers.some(plane => plane.name === 'Windowshade')) {
    throw new Error('this skin already has a Windowshade layer; open the project it came from');
  }
  await call('studio_layers', { action: 'add', name: 'Windowshade' });
  const title = await lift('titlebar.bmp', [27, 0, WIDTH, HEIGHT]);
  const unfocusedTitle = await lift('titlebar.bmp', [27, 15, WIDTH, HEIGHT]);
  const shadeCells = [await lift('titlebar.bmp', [0, 18, 9, 9]), await lift('titlebar.bmp', [9, 18, 9, 9])];
  const digit = await lift('text.bmp', [0, 6, 5, 6]);
  const font = { ground: digit[5][0], ink: digit.flat().find(colour => colour !== digit[5][0]) };

  const options = await call('studio_options', {});
  const visGround = options.visualizer_colors?.[0] ?? font.ground;
  const focused = strip(title, look, font, visGround);
  // Unfocused, the title's own dimming carries over colour for colour.
  const dim = new Map();
  title.forEach((row, y) => row.forEach((colour, x) => {
    if (!dim.has(colour)) dim.set(colour, unfocusedTitle[y][x]);
  }));
  const unfocused = focused.map(row => row.map(colour => dim.get(colour) ?? colour));
  unfocused[0] = focused[HEIGHT - 1].slice();

  await call('studio_atlas', { sheet: 'titlebar.bmp' });
  await draw('Rolled-up strip', [
    { op: 'stamp', x: MAIN_STRIP[0], y: MAIN_STRIP[1], ...stamp(focused) },
    { op: 'stamp', x: MAIN_STRIP_UNFOCUSED[0], y: MAIN_STRIP_UNFOCUSED[1], ...stamp(unfocused) },
  ]);
  await draw('Unroll button', [
    { op: 'stamp', x: 0, y: 27, ...stamp(unroll(shadeCells[0])) },
    { op: 'stamp', x: 9, y: 27, ...stamp(unroll(shadeCells[1])) },
  ]);
  const thumb = stamp(seekThumb(look));
  await draw('Rolled-up seek bar', [
    { op: 'stamp', x: 0, y: 36, ...stamp(seekTrack(look, font)) },
    { op: 'stamp', x: 17, y: 36, ...thumb },
    { op: 'stamp', x: 20, y: 36, ...thumb },
    { op: 'stamp', x: 23, y: 36, ...thumb },
  ]);
  await shadeCursors();
  await call('studio_canvas', { surface: 'shade' });
}

// Silverplay: dark aquamarine glass under a cool rim, the ornament of its
// title (a thin rule ending in a diamond) leading into the clock.
export const SILVERPLAY = {
  plain: 123,
  caps: [10, 10],
  rimRow: 1,
  outline: '#102b40',
  icon: '#c7e8e7',
  slotLight: '#32627b',
  thumbEdge: '#508ea3',
  thumbBody: '#8cc5d2',
  thumbLight: '#c7e8e7',
  wordmark: [140, 5, 23, 5],
  wordmarkAt: [20, 6],
  corners: title => {
    const O = title[0][0];
    const [K, M, L] = ['#102b40', '#32627b', '#8cc5d2'];
    const left = [[0, 12, O], [1, 12, K], [2, 12, M], [3, 12, L], [4, 12, L], [5, 12, M], [6, 12, M],
      [7, 12, M], [8, 12, M], [9, 12, M]];
    for (let x = 0; x < 10; x++) left.push([x, 13, title[0][x]]);
    const right = left.map(([x, y, colour]) => [WIDTH - 1 - x, y, y === 13 ? title[0][WIDTH - 1 - x] : colour]);
    return [...left, ...right];
  },
  ornament: out => {
    for (let x = 48; x <= 68; x++) out[8][x] = '#32627b';
    put(out, 70, 6, ['..#..', '.###.', '#####', '.###.', '..#..'], '#508ea3');
  },
};

// The illustrated Catamp skins keep their title art and move what collides.
const artLook = (colours, parts) => ({ ...colours, ...parts });

export const SKINS = {
  silverplay: { project: 'Catamp Silverplay', ...SILVERPLAY },
  'feral-night': artLook(
    { outline: '#9281ab', icon: '#f6efd6', slotLight: '#3d3158', thumbEdge: '#9281ab', thumbBody: '#e2d9be', thumbLight: '#f6efd6', plate: true },
    {
      project: 'Catamp Feral Night',
      fill: [[78, 244, 172, 244]],
      move: [{ rect: [137, 0, 35, 14], to: [41, 0], inks: ['#f6efd6', '#e2d9be', '#8d7f83', '#4d3d67', '#4f3c72'] }],
    }),
  'cat-scan': artLook(
    { outline: '#93ab9d', icon: '#c3d8c8', slotLight: '#2f3b3d', thumbEdge: '#93ab9d', thumbBody: '#c3d8c8', thumbLight: '#e3f2e6', plate: true },
    {
      project: 'Catamp Cat Scan',
      // The display keeps the skin's name and its end cap, left of the
      // visualizer.
      fill: [[73, 244, 240, 262]],
      move: [
        { rect: [53, 0, 45, 14], to: [20, 0] },
        { rect: [232, 0, 8, 14], to: [65, 0] },
      ],
    }),
  seance: artLook(
    { outline: '#e8d3ae', icon: '#e8d3ae', slotLight: '#3a262c', thumbEdge: '#9d95ad', thumbBody: '#e8d3ae', thumbLight: '#efd481', plate: true },
    {
      project: 'Catamp Seance',
      // Two candles stay; the third gives way to the name.
      fill: [[40, 180, 55, 70]],
      move: [{ rect: [141, 4, 35, 7], to: [41, 4], inks: ['#e8d3ae'] }],
    }),
  salvage: artLook(
    { outline: '#2a382c', icon: '#dcefeb', slotLight: '#3c3a2b', thumbEdge: '#6e694d', thumbBody: '#cee5c6', thumbLight: '#dcefeb', plate: true },
    {
      project: 'Catamp Salvage',
      // The letters cast a shadow in the grass's own green, so the old name
      // is refilled from the grunge beside it rather than healed.
      fill: [[86, 182, 188, 238]],
      move: [{ rect: [138, 3, 41, 8], to: [30, 3], inks: ['#dcefeb', '#2a382c'] }],
    }),
  freefall: artLook(
    { outline: '#0b1120', icon: '#cdf3ff', slotLight: '#44598a', thumbEdge: '#44598a', thumbBody: '#6d84bd', thumbLight: '#cdf3ff', plate: false },
    {
      project: 'Catamp Freefall',
      // The name takes the place of the falling marks.
      fill: [[40, 78, 20, 40]],
      heal: [{ rect: [94, 4, 78, 7], inks: ['#cdf3ff'] }],
      move: [{ rect: [131, 5, 39, 5], to: [37, 5], inks: ['#cdf3ff'] }],
    }),
  catnip: artLook(
    { outline: '#493448', icon: '#493448', slotLight: '#e58b95', thumbEdge: '#493448', thumbBody: '#e58b95', thumbLight: '#ffc0b7', plate: false },
    {
      project: 'Catamp Catnip',
      // The station's name takes the place of LATE AGAIN.
      heal: [
        { rect: [14, 2, 44, 11], inks: ['#493448'] },
        { rect: [116, 2, 54, 11], inks: ['#493448', '#e58b95'] },
        { rect: [84, 3, 28, 11], inks: ['#493448', '#71988e'] },
        { rect: [200, 3, 15, 8], inks: ['#eaa365', '#71988e'] },
      ],
      move: [{ rect: [118, 2, 50, 11], to: [16, 2], inks: ['#493448', '#e58b95'] }],
    }),
  'moon-garden': artLook(
    { outline: '#354c5f', icon: '#ecedde', slotLight: '#212a45', thumbEdge: '#354c5f', thumbBody: '#f4cf8d', thumbLight: '#f5dcc1', plate: true },
    {
      project: 'Catamp Moon Garden',
      // The moon rises on a patch of its own sky beside the leaves, and
      // leaves sky where it stood.
      heal: [{ rect: [126, 4, 52, 10], inks: ['#f4cf8d', '#395963', '#c4b5be'] }],
      fill: [[78, 126, 187, 194], [200, 244, 176, 196]],
      move: [{ rect: [206, 0, 28, 14], to: [50, 0] }],
    }),
  'midnight-snack': artLook(
    { outline: '#3b212f', icon: '#fae5b1', slotLight: '#3d2331', thumbEdge: '#3b212f', thumbBody: '#f4bd68', thumbLight: '#fae5b1', plate: true },
    {
      project: 'Catamp Midnight Snack',
      // MIDNIGHT is as much of the name as fits between the cat and the
      // visualizer; the candle gives way to it.
      fill: [[43, 78, 44, 56]],
      heal: [{ rect: [76, 3, 62, 7], inks: ['#f4bd68'] }],
      move: [{ rect: [80, 4, 32, 5], to: [45, 4], inks: ['#f4bd68'] }],
    }),
  'purr-chaos': artLook(
    { outline: '#25352f', icon: '#25352f', slotLight: '#bed1aa', thumbEdge: '#304b36', thumbBody: '#bed1aa', thumbLight: '#fff0c4', plate: false },
    {
      project: 'Catamp Purr Chaos Font Fixed',
      // NO THOUGHTS, closer together, where the leaf was: the options
      // button covers the leaf's stem anyway.
      fill: [[15, 110, 28, 33]],
      heal: [
        { rect: [110, 2, 65, 9], inks: ['#25352f'] },
        { rect: [205, 2, 25, 10], inks: ['#775b44'] },
      ],
      move: [
        { rect: [35, 3, 11, 7], to: [16, 3], inks: ['#25352f'] },
        { rect: [53, 3, 47, 7], to: [30, 3], inks: ['#25352f'] },
      ],
    }),
};

if (import.meta.url === `file://${process.argv[1]}`) {
  const [name, ...flags] = process.argv.slice(2);
  const look = SKINS[name];
  if (!look) throw new Error(`skin is one of ${Object.keys(SKINS).join(', ')}`);
  const root = new URL('../../', import.meta.url).pathname;
  const project = `${root}assets/skins/${look.project}`;
  await call('studio_project', { action: 'open', path: `${project}.cstudio`, discard: true });
  if (flags.includes('--replace')) {
    const status = await call('studio_status', {});
    for (const plane of status.paint_layers.layers.filter(plane => plane.name === 'Windowshade')) {
      await call('studio_layers', { action: 'delete', id: plane.id });
    }
  }
  await windowshade(look);
  if (flags.includes('--save')) {
    await call('studio_project', { action: 'save', path: `${project}.cstudio` });
    await call('studio_export', { path: `${project}.wsz` });
  }
}
