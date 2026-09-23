import test from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { runInNewContext } from 'node:vm';

const html = readFileSync(new URL('../index.html', import.meta.url), 'utf8');
const script = html.match(/<script type="module">([\s\S]*?)<\/script>/)[1]
  .replace(/import init, \{[^}]*\} from .*?;/, '');

async function host({ supported = true, startFails = null } = {}) {
  const events = () => ({
    handlers: new Map(),
    addEventListener(name, handler) {
      this.handlers.set(name, [...(this.handlers.get(name) || []), handler]);
    },
    removeEventListener(name, handler) {
      this.handlers.set(name, (this.handlers.get(name) || []).filter(h => h !== handler));
    },
    async fire(name) { for (const handler of this.handlers.get(name) || []) await handler({ type: name }); },
    dispatchEvent(event) {
      for (const handler of this.handlers.get(event.type) || []) handler(event);
      return true;
    },
  });
  const element = () => ({ ...events(), style: {}, disabled: true, textContent: '',
    appendChild(child) { child.parentElement = this; },
    getAttribute() { return null; }, removeAttribute() {}, setAttribute() {}, remove() {},
    getBoundingClientRect() { return { width: 275, height: 493 }; },
  });
  const main = element(), canvas = element(), button = element();
  const boot = { status: element(), error: element(), retry: element(), fill: element() };
  main.appendChild(canvas);
  const doc = { ...events(), body: { dataset: {} },
    getElementById: id => ({
      'cranamp-host': main, 'cranamp-canvas': canvas, 'pip-button': button,
      'boot-status': boot.status, 'boot-error': boot.error, 'boot-retry': boot.retry,
    })[id],
    querySelector: selector => (selector === '.boot-fill' ? boot.fill : null),
    createElement: () => ({ getContext: () => ({}) }),
  };
  const pipBody = element();
  const pip = { ...events(), closed: false, resizeTo() {}, focus() {}, document: {
    ...events(), head: {}, body: pipBody, documentElement: { style: {} },
  } };
  const asked = [];
  const floating = [];
  const win = { ...events(), setInterval() {}, setTimeout() {}, Audio: function() {},
    location: { reload() {} } };
  if (supported) {
    win.documentPictureInPicture = { async requestWindow(size) { asked.push(size); return pip; } };
  }
  await runInNewContext(`(async () => {${script}})()`, {
    window: win, document: doc, console: { ...console, error() {}, warn() {} }, Event,
    navigator: { gpu: {} },
    fetch: async () => ({ ok: true, headers: { get: () => null }, body: null }),
    init: async () => { if (startFails) throw new Error(startFails); },
    run_app: async () => {},
    set_floating: value => floating.push(value),
    floating_size: () => [275, 232],
  });
  return { win, doc, boot, canvas, main, button, pip, pipBody, asked, floating };
}

test('startup, first pointer, touch and key leave Cranamp embedded', async () => {
  const h = await host();
  for (const event of ['pointerdown', 'touchstart', 'keydown']) await h.win.fire(event);
  assert.equal(h.asked.length, 0);
  assert.equal(h.canvas.parentElement, h.main);
  assert.equal(h.button.disabled, false);
  assert.equal(h.doc.body.dataset.phase, 'running');
});

test('the floating window opens at the size the player reports, and closing restores the canvas', async () => {
  const h = await host();
  await h.button.fire('click');
  assert.equal(JSON.stringify(h.asked), JSON.stringify([{ width: 275, height: 232 }]));
  assert.equal(h.canvas.parentElement, h.pipBody);
  assert.deepEqual(h.floating, [true]);
  await h.pip.fire('pagehide');
  assert.equal(h.canvas.parentElement, h.main);
  assert.deepEqual(h.floating, [true, false]);
  assert.equal(h.button.disabled, false);
  await h.win.fire('pointerdown');
  assert.equal(h.asked.length, 1);
});

test('unsupported browsers remain embedded with floating control disabled', async () => {
  const h = await host({ supported: false });
  await h.button.fire('click');
  assert.equal(h.asked.length, 0);
  assert.equal(h.button.disabled, true);
  assert.equal(h.canvas.parentElement, h.main);
});

test('a player that cannot start says why and offers a reload', async () => {
  const h = await host({ startFails: 'the GPU was lost' });
  assert.equal(h.doc.body.dataset.phase, 'error');
  assert.match(h.boot.error.textContent, /the GPU was lost/);
});
