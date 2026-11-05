import { createApp } from 'vue';
import { createPinia } from 'pinia';
import HuiVue, { loadHui } from '@itmrhoang/h-ui/vue';
import { setLocale } from '@itmrhoang/h-ui/core';
import '@itmrhoang/h-ui/undefined.css';

import './styles/app.css';
import App from './App.vue';
import { router } from './router';
import { useAuthStore } from './stores/auth';

async function bootstrap() {
  setLocale('vi');
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
