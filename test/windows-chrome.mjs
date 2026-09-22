import { mkdirSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';

const [url, outputDirectory, endpoint = 'http://127.0.0.1:9223'] = process.argv.slice(2);
if (!url || !outputDirectory) throw new Error('Usage: node test/windows-chrome.mjs URL OUTPUT_DIRECTORY [CDP_URL]');
mkdirSync(outputDirectory, { recursive: true });
const target = await (await fetch(`${endpoint}/json/new?about:blank`, { method: 'PUT' })).json();
const socket = new WebSocket(target.webSocketDebuggerUrl);
await new Promise((resolve, reject) => { socket.onopen = resolve; socket.onerror = reject; });
const events = [];
const pending = new Map();
let sequence = 0;
let rendererReady;
const ready = new Promise(resolve => { rendererReady = resolve; });
socket.onmessage = event => {
  const message = JSON.parse(event.data);
  if (message.id) {
    const request = pending.get(message.id);
    if (!request) return;
    pending.delete(message.id);
    clearTimeout(request.timeout);
    if (message.error) request.reject(new Error(JSON.stringify(message.error)));
    else request.resolve(message.result);
  } else {
    events.push(message);
    if (message.method === 'Runtime.consoleAPICalled' && message.params.args.some(arg => String(arg.value).includes('renderer ready'))) rendererReady();
  }
};
function command(method, params = {}) {
  const id = ++sequence;
  return new Promise((resolve, reject) => {
    const timeout = setTimeout(() => { pending.delete(id); reject(new Error(`${method} exceeded 30 seconds`)); }, 30000);
    pending.set(id, { resolve, reject, timeout });
    socket.send(JSON.stringify({ id, method, params }));
  });
}
try {
  await command('Runtime.enable');
  await command('Page.enable');
  await command('Log.enable');
  await command('Page.addScriptToEvaluateOnNewDocument', { source: `
    window.__cranampTestAudio = [];
    const NativeAudio = window.Audio;
    window.Audio = function(...args) { const audio = new NativeAudio(...args); window.__cranampTestAudio.push(audio); return audio; };
    window.Audio.prototype = NativeAudio.prototype;
  ` });
  const started = Date.now();
  await command('Page.navigate', { url });
  await Promise.race([ready, new Promise((_, reject) => {
    setTimeout(() => reject(new Error('Renderer initialization exceeded 30 seconds')), 30000).unref();
  })]);
  const frames = await command('Runtime.evaluate', {
    expression: 'new Promise(resolve => requestAnimationFrame(() => requestAnimationFrame(() => resolve(true))))',
    awaitPromise: true, returnByValue: true,
  });
  if (frames.exceptionDetails || frames.result?.value !== true) throw new Error(JSON.stringify(frames));
  const screenshot = await command('Page.captureScreenshot');
  writeFileSync(join(outputDirectory, 'window.png'), Buffer.from(screenshot.data, 'base64'));
  if (process.env.CRANAMP_CHECK_PLAYBACK === '1') {
    await command('Input.dispatchMouseEvent', { type: 'mousePressed', x: 76, y: 118, button: 'left', clickCount: 1 });
    await command('Input.dispatchMouseEvent', { type: 'mouseReleased', x: 76, y: 118, button: 'left', clickCount: 1 });
    const playback = await command('Runtime.evaluate', { awaitPromise: true, returnByValue: true, expression: `new Promise(resolve => {
      const deadline = performance.now() + 10000;
      function check() {
        const audio = window.__cranampTestAudio.find(a => !a.paused && a.currentTime > 0);
        if (audio) return resolve({ playing: true, time: audio.currentTime, source: audio.currentSrc });
        if (performance.now() > deadline) return resolve({ playing: false });
        requestAnimationFrame(check);
      }
      check();
    })` });
    writeFileSync(join(outputDirectory, 'playback.json'), JSON.stringify(playback, null, 2));
    if (playback.result?.value?.playing !== true) throw new Error('Clicking Play did not advance the audio stream');
  }
  const errors = events.filter(event => event.method === 'Runtime.exceptionThrown' ||
    (event.method === 'Runtime.consoleAPICalled' && event.params.type === 'error'));
  if (errors.length) throw new Error(JSON.stringify(errors));
  console.log(`Chrome rendered and serviced animation frames in ${Date.now() - started} ms`);
} finally {
  writeFileSync(join(outputDirectory, 'events.json'), JSON.stringify(events, null, 2));
  socket.close();
  await fetch(`${endpoint}/json/close/${target.id}`, { signal: AbortSignal.timeout(5000) }).catch(() => {});
}
