import { createApp } from 'vue';
import { createPinia } from 'pinia';
import HuiVue, { loadHui } from '@itmrhoang/h-ui/vue';
import { setLocale } from '@itmrhoang/h-ui/core';
import '@itmrhoang/h-ui/undefined.css';
import { registerIcon } from '@itmrhoang/h-ui/hui-icon';

import './styles/app.css';
import App from './App.vue';
import { router } from './router';
import { useAuthStore } from './stores/auth';

async function bootstrap() {
  setLocale('vi');
  // Icon chưa có trong bộ của H-UI
  registerIcon('bell', '<path d="M6 8a6 6 0 0 1 12 0c0 7 3 9 3 9H3s3-2 3-9"/><path d="M10.3 21a1.94 1.94 0 0 0 3.4 0"/>');
  await loadHui();

  const app = createApp(App);
  app.use(createPinia());
  app.use(HuiVue, { components: false });

  // Khôi phục phiên (cookie refresh) trước khi router kiểm tra quyền.
  await useAuthStore().restore();

  app.use(router);
  await router.isReady();
  app.mount('#app');
}

bootstrap();
