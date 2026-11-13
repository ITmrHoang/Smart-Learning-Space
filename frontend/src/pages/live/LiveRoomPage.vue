<template>
  <LiveRoom v-if="info" :info="info" @left="back" />
  <div v-else class="center">
    <div class="panel stack box">
      <h1 style="margin: 0">Phòng học trực tuyến</h1>
      <hui-alert v-if="error" severity="error" :description="error" />
      <p v-else class="muted">Đang vào phòng…</p>
      <RouterLink :to="{ name: 'session', params: { id: route.params.id } }">← Về trang buổi học</RouterLink>
    </div>
  </div>
</template>

<script setup>
/** Phòng học của buổi (học sinh / giáo viên đã đăng nhập). Khách vào bằng link: GuestJoinPage. */
import { onMounted, onUnmounted, ref } from 'vue';
import { useRoute, useRouter } from 'vue-router';

import { liveApi } from '../../api';
import LiveRoom from '../../components/live/LiveRoom.vue';

const route = useRoute();
const router = useRouter();
const info = ref(null);
const error = ref('');

function back() {
  router.push({ name: 'session', params: { id: route.params.id } });
}

onMounted(async () => {
  try {
    info.value = await liveApi.join(route.params.id);
    document.title = `${info.value.session.title} — phòng học`;
  } catch (e) {
    error.value = e.message;
  }
});
onUnmounted(() => (document.title = 'Smart Learning Space'));
</script>

<style scoped>
.center {
  min-height: 100vh;
  display: grid;
  place-items: center;
  padding: 16px;
  background: var(--hui-bg-layout, #eef2f7);
}
.box {
  width: min(480px, 100%);
}
</style>
