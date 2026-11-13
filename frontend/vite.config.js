import { defineConfig } from 'vite';
import vue from '@vitejs/plugin-vue';

export default defineConfig({
  plugins: [
    vue({
      // Thẻ hui-* là Web Component của H-UI, không phải component Vue.
      template: { compilerOptions: { isCustomElement: (tag) => tag.startsWith('hui-') } },
    }),
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
