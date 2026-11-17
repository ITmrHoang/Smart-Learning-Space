import { existsSync, readFileSync } from 'node:fs';
import { join } from 'node:path';

import { describe, expect, it, vi } from 'vitest';

import { setupPwa, usePwaInstall } from './pwa';

// Vitest chạy trong thư mục frontend
const publicDir = join(process.cwd(), 'public/');

describe('pwa', () => {
  it('giữ sự kiện cài đặt của trình duyệt, nút "Cài ứng dụng" gọi hộp thoại một lần', async () => {
    setupPwa({ register: false });
    const { canInstall, install } = usePwaInstall();
    expect(canInstall.value).toBe(false);

    const e = new Event('beforeinstallprompt', { cancelable: true });
    e.prompt = vi.fn(async () => {});
    e.userChoice = Promise.resolve({ outcome: 'accepted' });
    window.dispatchEvent(e);
    expect(e.defaultPrevented).toBe(true);
    expect(canInstall.value).toBe(true);

    expect(await install()).toBe(true);
    expect(e.prompt).toHaveBeenCalledTimes(1);
    expect(canInstall.value).toBe(false);
    expect(await install()).toBe(false);
  });

  it('manifest đủ điều kiện cài đặt, icon có thật', () => {
    const manifest = JSON.parse(readFileSync(`${publicDir}manifest.json`, 'utf8'));
    expect(manifest.display).toBe('standalone');
    expect(manifest.start_url).toBe('/');
    const sizes = manifest.icons.map((i) => i.sizes);
    expect(sizes).toContain('192x192');
    expect(sizes).toContain('512x512');
    expect(manifest.icons.some((i) => i.purpose === 'maskable')).toBe(true);
    for (const icon of manifest.icons) expect(existsSync(`${publicDir}${icon.src.slice(1)}`), icon.src).toBe(true);
    // Service worker lưu sẵn trang ngoại tuyến, không đụng tới API
    const sw = readFileSync(`${publicDir}sw.js`, 'utf8');
    expect(sw).toContain("'/offline.html'");
    expect(existsSync(`${publicDir}offline.html`)).toBe(true);
    expect(sw).toMatch(/api\|files\|rtc/);
  });
});
