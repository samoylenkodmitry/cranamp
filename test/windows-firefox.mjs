import { mkdirSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';

const [url, outputDirectory, driver = 'http://127.0.0.1:4444'] = process.argv.slice(2);
if (!url || !outputDirectory) throw new Error('Usage: node test/windows-firefox.mjs URL OUTPUT_DIRECTORY [GECKODRIVER_URL]');
mkdirSync(outputDirectory, { recursive: true });
const events = [];
let rendererReady;
const ready = new Promise(resolve => { rendererReady = resolve; });
let session;
let socket;
async function command(path, body, timeout = 30000) {
  const response = await fetch(`${driver}${path}`, {
    method: body === undefined ? 'GET' : 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: body === undefined ? undefined : JSON.stringify(body),
    signal: AbortSignal.timeout(timeout),
  });
  const result = await response.json();
  if (!response.ok) throw new Error(JSON.stringify(result));
  return result.value;
}
try {
  const created = await command('/session', { capabilities: { alwaysMatch: {
    browserName: 'firefox', webSocketUrl: true,
    'moz:firefoxOptions': { binary: 'C:\\Program Files\\Mozilla Firefox\\firefox.exe' },
  } } });
  session = created.sessionId;
  writeFileSync(join(outputDirectory, 'session.json'), JSON.stringify(created, null, 2));
  socket = new WebSocket(created.capabilities.webSocketUrl);
  await new Promise((resolve, reject) => {
    socket.onopen = resolve;
    socket.onerror = reject;
  });
  const subscribed = new Promise(resolve => {
    socket.onmessage = event => {
      const value = JSON.parse(event.data);
      events.push(value);
      if (value.params?.text?.includes('renderer ready')) rendererReady();
      if (value.id === 1) resolve();
    };
  });
  socket.send(JSON.stringify({ id: 1, method: 'session.subscribe', params: { events: ['log.entryAdded'] } }));
  await subscribed;
  if (process.env.CRANAMP_SHADER_TRACE === '1' || process.env.CRANAMP_CHECK_PLAYBACK === '1') {
    const installed = new Promise(resolve => socket.addEventListener('message', event => {
      if (JSON.parse(event.data).id === 2) resolve();
    }));
    socket.send(JSON.stringify({ id: 2, method: 'script.addPreloadScript', params: {
      functionDeclaration: `() => {
        window.__cranampTestAudio = [];
        const NativeAudio = window.Audio;
        window.Audio = function(...args) {
          const audio = new NativeAudio(...args);
          window.__cranampTestAudio.push(audio);
          return audio;
        };
        window.Audio.prototype = NativeAudio.prototype;
        if (${process.env.CRANAMP_SHADER_TRACE === '1'}) {
          const original = WebGL2RenderingContext.prototype.shaderSource;
          WebGL2RenderingContext.prototype.shaderSource = function(shader, source) {
            console.info('cranamp-shader-source', source);
            return original.call(this, shader, source);
          };
        }
      }`,
    } }));
    await installed;
  }
  const started = Date.now();
  await command(`/session/${session}/url`, { url });
  await Promise.race([ready, new Promise((_, reject) => {
    setTimeout(() => reject(new Error('Renderer initialization exceeded 30 seconds')), 30000).unref();
  })]);
  await command(`/session/${session}/execute/async`, {
    script: 'const done = arguments[arguments.length - 1]; requestAnimationFrame(() => requestAnimationFrame(() => done(true)));', args: [],
  });
  const screenshot = await command(`/session/${session}/screenshot`);
  writeFileSync(join(outputDirectory, 'window.png'), Buffer.from(screenshot, 'base64'));
  if (process.env.CRANAMP_CHECK_PLAYBACK === '1') {
    await command(`/session/${session}/actions`, { actions: [{ type: 'pointer', id: 'mouse', parameters: { pointerType: 'mouse' }, actions: [
      { type: 'pointerMove', duration: 0, x: 76, y: 118, origin: 'viewport' },
      { type: 'pointerDown', button: 0 }, { type: 'pointerUp', button: 0 },
    ] }] });
    const playback = await command(`/session/${session}/execute/async`, {
      script: `const done = arguments[arguments.length - 1];
        const deadline = performance.now() + 10000;
        function check() {
          const audio = window.__cranampTestAudio.find(a => !a.paused && a.currentTime > 0);
          if (audio) return done({ playing: true, time: audio.currentTime, source: audio.currentSrc });
          if (performance.now() > deadline) return done({ playing: false });
          requestAnimationFrame(check);
        }
        check();`, args: [],
    });
    writeFileSync(join(outputDirectory, 'playback.json'), JSON.stringify(playback, null, 2));
    if (!playback.playing) throw new Error('Clicking Play did not advance the audio stream');
  }
  if (!events.some(event => event.params?.text?.includes('renderer ready'))) {
    throw new Error('Renderer did not finish initialization');
  }
  const errors = events.filter(event => event.params?.level === 'error');
  if (errors.length) throw new Error(JSON.stringify(errors));
  console.log(`Firefox rendered and serviced animation frames in ${Date.now() - started} ms`);
} finally {
  writeFileSync(join(outputDirectory, 'events.json'), JSON.stringify(events, null, 2));
  socket?.close();
  if (session) {
    await fetch(`${driver}/session/${session}`, { method: 'DELETE', signal: AbortSignal.timeout(5000) }).catch(() => {});
  }
}
