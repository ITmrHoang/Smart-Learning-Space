<template>
  <div class="status">
    <section class="panel">
      <h1>Trạng thái hệ thống</h1>
      <hui-alert v-if="error" severity="error" :description="error" />
      <dl v-if="health" class="info">
        <dt>API</dt>
        <dd><hui-tag :color="health.status === 'ok' ? 'success' : 'warning'">{{ health.status }}</hui-tag> v{{ health.version }}</dd>
        <dt>PostgreSQL</dt>
        <dd><hui-tag :color="health.checks.database ? 'success' : 'danger'">{{ health.checks.database ? 'OK' : 'Lỗi' }}</hui-tag></dd>
        <dt>Valkey</dt>
        <dd><hui-tag :color="health.checks.valkey ? 'success' : 'danger'">{{ health.checks.valkey ? 'OK' : 'Lỗi' }}</hui-tag></dd>
        <dt>RabbitMQ</dt>
        <dd><hui-tag :color="health.checks.queue ? 'success' : 'danger'">{{ health.checks.queue ? 'OK' : 'Lỗi' }}</hui-tag></dd>
      </dl>
      <p class="small"><RouterLink to="/">Về trang chủ</RouterLink></p>
    </section>
  </div>
</template>

<script setup>
import { onMounted, ref } from 'vue';

import { systemApi } from '../api';

const health = ref(null);
const error = ref('');

onMounted(async () => {
  try {
    health.value = await systemApi.health();
  } catch (e) {
    error.value = e.message;
  }
});
</script>

<style scoped>
.status {
  max-width: 520px;
  margin: 48px auto;
  padding: 0 16px;
}
</style>
