import test from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { runInNewContext } from 'node:vm';

const html = readFileSync(new URL('../index.html', import.meta.url), 'utf8');
const script = html.match(/<script type="module">([\s\S]*?)<\/script>/)[1]
  .replace(/import init, \{ run_app \} from .*?;/, '');

async function host(supported = true) {
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
  const element = () => ({ ...events(), style: {}, disabled: true,
    appendChild(child) { child.parentElement = this; },
    getAttribute() { return null; }, removeAttribute() {}, setAttribute() {},
    getBoundingClientRect() { return { width: 275, height: 493 }; },
  });
  const main = element(), canvas = element(), button = element();
  main.appendChild(canvas);
  const doc = { ...events(), getElementById: id => ({
    'cranamp-host': main, 'cranamp-canvas': canvas, 'pip-button': button,
  })[id] };
  const floatingHost = element();
  const pip = { ...events(), closed: false, resizeTo() {}, focus() {}, document: {
    ...events(), head: {}, body: { style: {} }, documentElement: { style: {} },
    getElementById: () => floatingHost,
  } };
  let requests = 0;
  const win = { ...events(), setInterval() {}, setTimeout() {}, Audio: function() {} };
  if (supported) win.documentPictureInPicture = { async requestWindow() { requests++; return pip; } };
  await runInNewContext(`(async () => {${script}})()`, {
    window: win, document: doc, console, Event, init: async () => {}, run_app: async () => {},
  });
  return { win, canvas, main, button, pip, floatingHost, requests: () => requests };
}

test('startup, first pointer, touch and key leave Cranamp embedded', async () => {
  const h = await host();
  for (const event of ['pointerdown', 'touchstart', 'keydown']) await h.win.fire(event);
  assert.equal(h.requests(), 0);
  assert.equal(h.canvas.parentElement, h.main);
  assert.equal(h.button.disabled, false);
});

test('only the floating button opens PiP; closing restores the canvas', async () => {
  const h = await host();
  await h.button.fire('click');
  assert.equal(h.requests(), 1);
  assert.equal(h.canvas.parentElement, h.floatingHost);
  await h.pip.fire('pagehide');
  assert.equal(h.canvas.parentElement, h.main);
  assert.equal(h.button.disabled, false);
  await h.win.fire('pointerdown');
  assert.equal(h.requests(), 1);
});

test('unsupported browsers remain embedded with floating control disabled', async () => {
  const h = await host(false);
  await h.button.fire('click');
  assert.equal(h.requests(), 0);
  assert.equal(h.button.disabled, true);
  assert.equal(h.canvas.parentElement, h.main);
});
