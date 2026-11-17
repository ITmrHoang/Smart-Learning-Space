/**
 * PWA (M33): đăng ký service worker (chỉ bản build — khi dev, Vite tự nạp lại mã) + nút "Cài ứng dụng"
 * khi trình duyệt cho phép (Chrome / Edge / Android). iPhone: Chia sẻ → "Thêm vào MH chính".
 */
import { computed, ref } from 'vue';

/** Sự kiện `beforeinstallprompt` trình duyệt đưa ra; null = không cài được hoặc đã cài. */
const installEvent = ref(null);

export function setupPwa({ register = import.meta.env.PROD } = {}) {
  if (typeof window === 'undefined') return;
  window.addEventListener('beforeinstallprompt', (e) => {
    e.preventDefault();
    installEvent.value = e;
  });
  window.addEventListener('appinstalled', () => {
    installEvent.value = null;
  });
  if (register && 'serviceWorker' in navigator) {
    const start = () => navigator.serviceWorker.register('/sw.js').catch(() => {});
    if (document.readyState === 'complete') start();
    else window.addEventListener('load', start, { once: true });
  }
}

export function usePwaInstall() {
  /** Hiện hộp thoại cài của trình duyệt; true nếu người dùng đồng ý. */
  async function install() {
    const e = installEvent.value;
    if (!e) return false;
    installEvent.value = null;
    await e.prompt();
    const choice = await e.userChoice;
    return choice?.outcome === 'accepted';
  }
  return { canInstall: computed(() => installEvent.value !== null), install };
}

/** Đang chạy như ứng dụng đã cài (cửa sổ riêng, không có thanh địa chỉ). */
export function isStandalone() {
  return window.matchMedia?.('(display-mode: standalone)').matches === true || navigator.standalone === true;
}
