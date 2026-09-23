// Keeps the player on the device, so an installed Cranamp opens at once and
// without a network. build-web.sh names the cache after the build it ships,
// so a new deploy fills a cache of its own and the old one is dropped.
const CACHE = "__CRANAMP_CACHE__";
const SHELL = [
  "./",
  "./index.html",
  "./pkg/cranamp.js",
  "./pkg/cranamp_bg.wasm",
  "./manifest.webmanifest",
  "./favicon.png",
  "./apple-touch-icon.png",
  "./icon-192.png",
  "./icon-512.png",
  "./icon-maskable-512.png",
];

self.addEventListener("install", (event) => {
  event.waitUntil(
    caches.open(CACHE).then((cache) => cache.addAll(SHELL)).then(() => self.skipWaiting()),
  );
});

self.addEventListener("activate", (event) => {
  event.waitUntil(
    caches
      .keys()
      .then((keys) => Promise.all(
        keys.filter((key) => key.startsWith("cranamp-") && key !== CACHE).map((key) => caches.delete(key)),
      ))
      .then(() => self.clients.claim()),
  );
});

function remember(request, response) {
  if (response.ok && response.type === "basic") {
    const copy = response.clone();
    caches.open(CACHE).then((cache) => cache.put(request, copy));
  }
  return response;
}

self.addEventListener("fetch", (event) => {
  const { request } = event;
  if (request.method !== "GET" || new URL(request.url).origin !== self.location.origin) {
    return;
  }
  // Audio is read in ranges, which a cache cannot answer; the page asks the
  // network for it as it always did.
  if (request.headers.has("range")) {
    return;
  }
  if (request.mode === "navigate") {
    // The page itself comes from the network first, so a new deploy shows on
    // the next visit, and from the cache when there is no network.
    event.respondWith(
      fetch(request)
        .then((response) => remember(request, response))
        .catch(() => caches.match("./index.html")),
    );
    return;
  }
  event.respondWith(
    caches.match(request).then((cached) => cached || fetch(request).then((response) => remember(request, response))),
  );
});
