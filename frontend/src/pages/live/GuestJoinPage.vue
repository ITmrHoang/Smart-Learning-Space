<template>
  <LiveRoom v-if="info" :info="info" @left="left = true; info = null" />
  <div v-else class="center">
    <div class="panel stack box">
      <span class="brand">Smart Learning Space</span>
      <template v-if="left">
        <h1 style="margin: 0">Bạn đã rời phòng</h1>
        <p class="muted" style="margin: 0">Cảm ơn bạn đã tham gia buổi học.</p>
      </template>
      <template v-else-if="ticket">
        <h1 style="margin: 0">{{ ticket.session.title }}</h1>
        <p class="muted" style="margin: 0">Giáo viên {{ ticket.session.teacherName }} · {{ formatSessionTime(ticket.session.startsAt, ticket.session.endsAt) }}</p>
        <hui-alert v-if="status === 'denied'" severity="error" description="Giáo viên không cho vào buổi học này." />
        <hui-alert v-else-if="status === 'removed'" severity="warning" description="Bạn đã được mời ra khỏi phòng." />
        <div v-else class="waiting">
          <hui-spinner size="sm" />
          <span>Đang chờ giáo viên cho vào phòng… Giữ trang này mở.</span>
        </div>
      </template>
      <form v-else class="stack" @submit.prevent="request">
        <h1 style="margin: 0">Vào buổi học</h1>
        <p class="muted small" style="margin: 0">
          Bạn vào với tư cách <b>khách</b>: xem và nghe buổi học, không bật mic / camera, không chat. Giáo viên duyệt
          rồi bạn mới vào được phòng.
        </p>
        <hui-alert v-if="error" severity="error" :description="error" />
        <hui-input v-model="name" label="Tên hiển thị" placeholder="Ví dụ: Phụ huynh em An" required />
        <hui-button type="submit" :loading="loading" :disabled="name.trim().length < 2">Xin vào phòng</hui-button>
      </form>
    </div>
  </div>
</template>

<script setup>
/** Khách vào buổi học bằng link (không cần tài khoản): nhập tên → phòng chờ → giáo viên duyệt → vào phòng. */
import { onBeforeUnmount, ref } from 'vue';
import { useRoute } from 'vue-router';

import { liveApi } from '../../api';
import LiveRoom from '../../components/live/LiveRoom.vue';
import { formatSessionTime } from '../../utils/schedule';

const route = useRoute();
const name = ref('');
const loading = ref(false);
const error = ref('');
const ticket = ref(null);
const status = ref('waiting');
const info = ref(null);
const left = ref(false);
let timer = null;

async function request() {
  loading.value = true;
  error.value = '';
  try {
    ticket.value = await liveApi.guestJoin(route.params.id, String(route.query.k ?? ''), name.value.trim());
    poll();
    timer = setInterval(poll, 3000);
  } catch (e) {
    error.value = e.message;
  } finally {
    loading.value = false;
  }
}

async function poll() {
  try {
    const s = await liveApi.guestState(ticket.value.guestId, ticket.value.token);
    status.value = s.status;
    if (s.status !== 'waiting') clearInterval(timer);
    if (s.live) info.value = s.live;
  } catch (e) {
    error.value = e.message;
  }
}

onBeforeUnmount(() => clearInterval(timer));
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
.brand {
  font-weight: 700;
  color: var(--hui-primary, #0864ad);
}
.waiting {
  display: flex;
  gap: 10px;
  align-items: center;
}
</style>
