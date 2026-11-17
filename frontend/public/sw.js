/*
 * Service worker của Smart Learning Space (M33 — PWA): cài ứng dụng lên màn hình chính + trang báo mất mạng.
 *
 * - Mở trang (điều hướng): luôn lấy từ mạng; mất mạng → trang "Bạn đang ngoại tuyến" (đã lưu sẵn).
 * - Icon, manifest: lưu sẵn khi cài.
 * - Không đụng tới /api, /files, /rtc, trang công khai /p/*: dữ liệu riêng tư, link có chữ ký, WebSocket —
 *   luôn đi thẳng mạng như khi chưa có service worker.
 * - Mã ứng dụng (/assets/*, tên có hash) đã được trình duyệt cache lâu theo header của Caddy, không cache lại ở đây
 *   → service worker không giữ bản cũ của ứng dụng, cập nhật bản mới không cần hỏi người dùng.
 */
const VERSION = 'sls-v1';
const PRECACHE = ['/offline.html', '/manifest.json', '/icons/icon.svg', '/icons/icon-192.png', '/icons/favicon-32.png'];

self.addEventListener('install', (event) => {
  event.waitUntil(
    caches
      .open(VERSION)
      .then((cache) => cache.addAll(PRECACHE))
      .then(() => self.skipWaiting()),
  );
});

self.addEventListener('activate', (event) => {
  event.waitUntil(
    caches
      .keys()
      .then((keys) => Promise.all(keys.filter((k) => k !== VERSION).map((k) => caches.delete(k))))
      .then(() => self.clients.claim()),
  );
});

self.addEventListener('fetch', (event) => {
  const req = event.request;
  if (req.method !== 'GET') return;
  const url = new URL(req.url);
  if (url.origin !== self.location.origin) return;

  if (req.mode === 'navigate') {
    if (/^\/(api|files|rtc|p)(\/|$)/.test(url.pathname)) return;
    event.respondWith(
      fetch(req).catch(async () => (await caches.match('/offline.html', { cacheName: VERSION })) || Response.error()),
    );
    return;
  }
  if (PRECACHE.includes(url.pathname)) {
    // Lưu sẵn, cập nhật ngầm khi có mạng
    event.respondWith(
      caches.open(VERSION).then(async (cache) => {
        const hit = await cache.match(req);
        const fresh = fetch(req)
          .then((res) => {
            if (res.ok) cache.put(req, res.clone());
            return res;
          })
          .catch(() => hit);
        return hit || fresh;
      }),
    );
  }
});
