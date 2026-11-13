<template>
  <div class="page">
    <div class="page-header">
      <h1>Học kèm 1-1</h1>
      <hui-button @hui-click="edit(null)">Mở lịch học kèm</hui-button>
    </div>
    <p class="muted small">
      Mở các khung giờ rảnh hằng tuần; học sinh tự chọn giờ và gửi yêu cầu. Bạn bấm "Nhận" thì hệ thống tạo buổi học,
      gửi lời mời kèm file lịch cho học sinh và nhắc lịch như buổi học thường.
    </p>
    <hui-alert v-if="error" severity="error" :description="error" />

    <section class="panel stack">
      <h2 style="margin: 0">Yêu cầu chờ trả lời ({{ pending.length }})</h2>
      <p v-if="!pending.length" class="muted small" style="margin: 0">Không có yêu cầu nào đang chờ.</p>
      <article v-for="b in pending" :key="b.id" class="booking">
        <div class="stack" style="gap: 2px">
          <b>{{ b.studentName }} <span class="muted small">@{{ b.studentUsername }}</span></b>
          <span class="small">{{ formatSessionTime(b.startsAt, b.endsAt) }} · {{ b.offerTitle }}</span>
          <span v-if="b.message" class="small">“{{ b.message }}”</span>
        </div>
        <span class="row">
          <hui-button size="sm" :loading="busy === b.id" @hui-click="approve(b)">Nhận</hui-button>
          <hui-button size="sm" variant="outline" @hui-click="ask('decline', b)">Từ chối</hui-button>
        </span>
      </article>
    </section>

    <section class="panel stack">
      <h2 style="margin: 0">Lịch đã nhận sắp tới</h2>
      <p v-if="!approved.length" class="muted small" style="margin: 0">Chưa có buổi học kèm nào sắp tới.</p>
      <article v-for="b in approved" :key="b.id" class="booking">
        <div class="stack" style="gap: 2px">
          <b>{{ b.studentName }}</b>
          <span class="small">{{ formatSessionTime(b.startsAt, b.endsAt) }} · {{ b.offerTitle }}</span>
        </div>
        <span class="row">
          <RouterLink v-if="b.sessionId" :to="{ name: 'session', params: { id: b.sessionId } }" class="small">Buổi học →</RouterLink>
          <hui-button size="sm" variant="text" @hui-click="ask('cancel', b)">Hủy lịch</hui-button>
        </span>
      </article>
    </section>

    <section class="stack">
      <h2 style="margin: 0">Khung giờ đang mở</h2>
      <p v-if="loaded && !offers.length" class="panel empty muted">Chưa mở lịch học kèm nào.</p>
      <div class="card-list">
        <article v-for="o in offers" :key="o.id" class="card" :class="{ paused: !o.active }">
          <a href="#" class="title" @click.prevent="edit(o)"><h3>{{ o.title }}</h3></a>
          <div class="row">
            <hui-tag size="sm">{{ formatDuration(o.durationMinutes) }} / buổi</hui-tag>
            <hui-tag size="sm" :color="o.audience === 'everyone' ? 'info' : 'secondary'">
              {{ o.audience === 'everyone' ? 'Mọi tài khoản' : 'Học sinh của tôi' }}
            </hui-tag>
            <hui-tag v-if="!o.active" size="sm" color="warning">Tạm dừng</hui-tag>
            <hui-tag v-if="o.pendingCount" size="sm" color="danger">{{ o.pendingCount }} chờ trả lời</hui-tag>
          </div>
          <p class="small" style="margin: 0">{{ formatWindows(o.windows) }}</p>
          <p v-if="o.description" class="muted small" style="margin: 0">{{ o.description }}</p>
        </article>
      </div>
    </section>

    <OfferFormModal v-model:open="editorOpen" :offer="editing" @saved="load" />
    <PromptModal
      v-model:open="promptOpen"
      :title="promptKind === 'decline' ? 'Từ chối yêu cầu' : 'Hủy lịch học kèm'"
      label="Lý do (học sinh sẽ thấy)"
      :saveFn="sendReason"
      @saved="load"
    />
  </div>
</template>

<script setup>
import { message } from '@itmrhoang/h-ui/hui-message';
import { computed, onMounted, ref } from 'vue';

import { teachingApi } from '../../api';
import PromptModal from '../../components/PromptModal.vue';
import OfferFormModal from '../../components/teaching/OfferFormModal.vue';
import { formatDuration, formatSessionTime, formatWindows } from '../../utils/schedule';

const offers = ref([]);
const bookings = ref([]);
const loaded = ref(false);
const error = ref('');
const busy = ref('');
const editorOpen = ref(false);
const editing = ref(null);
const promptOpen = ref(false);
const promptKind = ref('decline');
const target = ref(null);

const pending = computed(() => bookings.value.filter((b) => b.status === 'pending'));
const approved = computed(() => bookings.value.filter((b) => b.status === 'approved'));

async function load() {
  error.value = '';
  try {
    [offers.value, bookings.value] = await Promise.all([teachingApi.tutoringOffers(), teachingApi.bookings('upcoming')]);
  } catch (e) {
    error.value = e.message;
  } finally {
    loaded.value = true;
  }
}

function edit(o) {
  editing.value = o;
  editorOpen.value = true;
}

async function approve(b) {
  busy.value = b.id;
  try {
    await teachingApi.approveBooking(b.id);
    message.success(`Đã nhận lịch với ${b.studentName}. Học sinh nhận lời mời kèm file lịch.`);
    load();
  } catch (e) {
    message.error(e.message);
  } finally {
    busy.value = '';
  }
}

function ask(kind, b) {
  promptKind.value = kind;
  target.value = b;
  promptOpen.value = true;
}

const sendReason = (reason) =>
  promptKind.value === 'decline' ? teachingApi.declineBooking(target.value.id, reason) : teachingApi.cancelBooking(target.value.id, reason);

onMounted(load);
</script>

<style scoped>
.booking {
  display: flex;
  flex-wrap: wrap;
  gap: 8px;
  justify-content: space-between;
  align-items: center;
  padding: 10px 12px;
  border: 1px solid var(--hui-border-secondary, #eceef1);
  border-radius: var(--hui-radius-md, 8px);
}
.paused {
  opacity: 0.7;
}
</style>
