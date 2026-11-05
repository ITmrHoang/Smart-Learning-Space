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
    proxy: { '/api': 'http://127.0.0.1:8080' },
    // H-UI được link tới repo ở máy (file:../../HUI) → cho phép Vite đọc file ngoài thư mục dự án.
    fs: { allow: ['..', '../../HUI'] },
  },
  test: {
    environment: 'jsdom',
    include: ['src/**/*.test.js'],
  },
});
