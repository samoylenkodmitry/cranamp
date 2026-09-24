#!/usr/bin/env node
// Draws the two footer cells classic Winamp shows once the playlist is wider
// than 275, into a skin open in Skin Studio:
//
//   the footer tile, pledit (179,0,25,38), repeated between the corners, and
//   the visualizer panel, pledit (205,0,75,38), beside the right corner from
//   350 wide.
//
//   node tools/skin-studio/footer.mjs <skin> [--replace] [--save]
//
// --replace draws the Footer layer again in place of the one the skin has.
//
// Both continue the footer the skin already has, so a wider playlist reads as
// the same footer with more room in it. Each skin says where the ground
// between its corners comes from:
//
//   column  a column of the footer at 275, repeated: for grounds made of
//           horizontal bands, where the corners meet without a picture across
//           the seam.
//   plain   the rows of a column down to a line, the ground colour below it:
//           for a ground that is one colour under the footer's top rails.
//   noise   each row made again from the colours that row has in the given
//           quiet spans, a two-pixel dither kept as a dither: for speckled and
//           dotted grounds a repeated column would streak.
//
// While the main window is closed, Winamp draws its mini visualizer on the
// panel: a 72x16 field at +2,+12, painted in VISCOLOR.TXT's colour 0 and
// dotted. The panel is the ground with a screen let into it there, a
// one-pixel edge in the colour of the skin's own footer displays around the
// field, so the visualizer appears in a screen rather than on the ground.
// The default skin, assets/winamp.wsz, has Winamp's own tile and panel
// already and is left as it is.
import { call } from './moonlit.mjs';

const TILE = [179, 0, 25, 38];
const PANEL = [205, 0, 75, 38];
const LEFT = [0, 72, 125, 38];
const RIGHT = [126, 72, 150, 38];
const SEAM = 125;
/// The screen in the panel: the mini visualizer's field and its edge.
const SCREEN = [1, 11, 74, 18];

export const SKINS = {
  silverplay: { project: 'Catamp Silverplay', ground: { column: SEAM }, screen: '#102b40' },
  'feral-night': { project: 'Catamp Feral Night', ground: { noise: [[114, 128]] }, screen: '#231a39' },
  'cat-scan': { project: 'Catamp Cat Scan', ground: { noise: [[2, 10], [260, 273]] }, screen: '#080f10' },
  seance: { project: 'Catamp Seance', ground: { column: SEAM }, screen: '#0f0a17' },
  salvage: { project: 'Catamp Salvage', ground: { noise: [[2, 10], [258, 273]] }, screen: '#112022' },
  freefall: { project: 'Catamp Freefall', ground: { column: SEAM }, screen: '#33456e' },
  // The cat's body crosses the seam, so the cat grows longer.
  catnip: { project: 'Catamp Catnip', ground: { column: SEAM - 1 }, screen: '#493448' },
  'moon-garden': { project: 'Catamp Moon Garden', ground: { plain: SEAM, below: 4, colour: '#171e32' }, screen: '#756951' },
  'midnight-snack': { project: 'Catamp Midnight Snack', ground: { column: SEAM }, screen: '#3b212f' },
  'purr-chaos': { project: 'Catamp Purr Chaos Font Fixed', ground: { column: SEAM }, screen: '#365247' },
};

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
    if (!index.has(colour)) {
      const ch = String.fromCodePoint(next++);
      index.set(colour, ch);
      palette[ch] = colour;
    }
    return index.get(colour);
  }).join(''));
  return { rows, palette };
}

// A fixed hash, so a skin drawn twice comes out the same.
function hash(a, b, c) {
  let h = Math.imul(a ^ 0x9e3779b9, 0x85ebca6b) ^ Math.imul(b + 0x632be5ab, 0xc2b2ae35) ^ Math.imul(c, 0x27d4eb2f);
  h ^= h >>> 15;
  h = Math.imul(h, 0x2c1b3c6d);
  h ^= h >>> 12;
  return h >>> 0;
}

/// A dither's period, when a row's samples are one: two colours alternating
/// or a single colour.
function period(samples) {
  for (const p of [1, 2]) {
    if (samples.every((colour, i) => i + p >= samples.length || colour === samples[i + p])) return p;
  }
  return null;
}

/// `width` columns of ground for the footer, `salt` telling apart the tile
/// and the panel so the panel's speckle is not the tile's three times over.
function ground(footer, spec, width, salt) {
  return footer.map((row, y) => {
    if ('column' in spec) return Array(width).fill(row[spec.column]);
    if ('plain' in spec) return Array(width).fill(y < spec.below ? row[spec.plain] : spec.colour);
    const spans = spec.noise;
    const samples = spans.flatMap(([from, to]) => row.slice(from, to));
    const p = period(samples);
    if (p) {
      // Keep the dither in step with the left corner at the first seam.
      const anchor = spans[0][0];
      return Array.from({ length: width }, (_, x) => row[anchor + ((SEAM + x - anchor) % p)]);
    }
    return Array.from({ length: width }, (_, x) => samples[hash(salt, y, x) % samples.length]);
  });
}

export async function footer(look) {
  const status = await call('studio_status', {});
  if (status.paint_layers?.layers?.some(plane => plane.name === 'Footer')) {
    throw new Error('this skin already has a Footer layer; open the project it came from');
  }
  await call('studio_layers', { action: 'add', name: 'Footer' });
  const left = await lift('pledit.bmp', LEFT);
  const right = await lift('pledit.bmp', RIGHT);
  const whole = left.map((row, y) => row.concat(right[y]));
  const tile = ground(whole, look.ground, TILE[2], 1);
  const panel = ground(whole, look.ground, PANEL[2], 2);
  const options = await call('studio_options', {});
  const field = options.visualizer_colors?.[0];
  if (!field) throw new Error('the skin has no VISCOLOR.TXT colour 0 for the screen');
  const [sx, sy, sw, sh] = SCREEN;
  for (let y = sy; y < sy + sh; y++) {
    for (let x = sx; x < sx + sw; x++) {
      const edge = y === sy || y === sy + sh - 1 || x === sx || x === sx + sw - 1;
      panel[y][x] = edge ? look.screen : field;
    }
  }
  await call('studio_atlas', { sheet: 'pledit.bmp' });
  await call('studio_draw', {
    label: 'Playlist footer tile and visualizer panel',
    layers: [],
    states: 'current',
    operations: [
      { op: 'stamp', x: TILE[0], y: TILE[1], ...stamp(tile) },
      { op: 'stamp', x: PANEL[0], y: PANEL[1], ...stamp(panel) },
    ],
  });
  await call('studio_canvas', { surface: 'footer' });
}

if (import.meta.url === `file://${process.argv[1]}`) {
  const [name, ...flags] = process.argv.slice(2);
  const look = SKINS[name];
  if (!look) throw new Error(`skin is one of ${Object.keys(SKINS).join(', ')}`);
  const root = new URL('../../', import.meta.url).pathname;
  await call('studio_project', { action: 'open', path: `${root}assets/skins/${look.project}.cstudio`, discard: true });
  if (flags.includes('--replace')) {
    const status = await call('studio_status', {});
    for (const plane of status.paint_layers.layers.filter(plane => plane.name === 'Footer')) {
      await call('studio_layers', { action: 'delete', id: plane.id });
    }
  }
  await footer(look);
  if (flags.includes('--save')) {
    const project = `${root}assets/skins/${look.project}`;
    await call('studio_project', { action: 'save', path: `${project}.cstudio` });
    await call('studio_export', { path: `${project}.wsz` });
  }
}
