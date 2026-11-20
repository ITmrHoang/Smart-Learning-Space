import { cpSync, createReadStream, statSync } from 'node:fs';
import { createRequire } from 'node:module';
import { dirname, join, resolve, sep } from 'node:path';
import { defineConfig } from 'vite';
import vue from '@vitejs/plugin-vue';

/**
 * Tài nguyên pdfjs-dist (font chuẩn, bảng mã chữ, bộ giải mã ảnh scan .wasm) ở `/pdfjs/*`: trình duyệt chỉ tải
 * khi đọc một PDF cần tới. Lúc dev phục vụ thẳng từ node_modules, lúc build chép vào `dist/pdfjs/`.
 */
function pdfjsAssets() {
  const root = dirname(createRequire(import.meta.url).resolve('pdfjs-dist/package.json'));
  const dirs = ['standard_fonts', 'cmaps', 'wasm', 'iccs'];
  let outDir = 'dist';
  return {
    name: 'sls-pdfjs-assets',
    configResolved(config) {
      outDir = resolve(config.root, config.build.outDir);
    },
    configureServer(server) {
      server.middlewares.use('/pdfjs/', (req, res, next) => {
        const path = decodeURIComponent(req.url.split('?')[0]).replace(/^\/+/, '');
        const file = resolve(root, path);
        const allowed = dirs.some((d) => file.startsWith(join(root, d) + sep));
        if (!allowed || !statSync(file, { throwIfNoEntry: false })?.isFile()) return next();
        res.setHeader('Content-Type', file.endsWith('.wasm') ? 'application/wasm' : 'application/octet-stream');
        createReadStream(file).pipe(res);
      });
    },
    closeBundle() {
      for (const d of dirs) cpSync(join(root, d), join(outDir, 'pdfjs', d), { recursive: true });
    },
  };
}

export default defineConfig({
  plugins: [
    vue({
      // Thẻ hui-* là Web Component của H-UI, không phải component Vue.
      template: { compilerOptions: { isCustomElement: (tag) => tag.startsWith('hui-') } },
    }),
    pdfjsAssets(),
  ],
  server: {
    host: '127.0.0.1',
    port: 5173,
    strictPort: true,
    // Gọi API qua cùng origin → cookie refresh (SameSite=Strict) hoạt động, không cần CORS.
    // /files/* → Garage (upload / tải file bằng presigned URL); giữ nguyên Host để chữ ký khớp.
    // /p/*, /sitemap.xml, /robots.txt: trang công khai cho máy tìm kiếm do api dựng sẵn.
    proxy: {
      '/api': 'http://127.0.0.1:8080',
      // Dạng chuỗi của Vite tự bật changeOrigin (đổi Host) → chữ ký S3 sai; phải giữ Host của trình duyệt.
      '/files': { target: 'http://127.0.0.1:3900', changeOrigin: false },
      '/p/': 'http://127.0.0.1:8080',
      '/sitemap.xml': 'http://127.0.0.1:8080',
      '/robots.txt': 'http://127.0.0.1:8080',
    },
    // H-UI được link tới repo ở máy (file:../../HUI) → cho phép Vite đọc file ngoài thư mục dự án.
    fs: { allow: ['..', '../../HUI'] },
  },
  test: {
    environment: 'jsdom',
    include: ['src/**/*.test.js'],
  },
});
