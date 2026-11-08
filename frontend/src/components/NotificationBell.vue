<template>
  <hui-badge :count="unread ? String(unread) : ''" :max="99">
    <hui-button variant="text" icon="bell" shape="circle" :aria-label="unread ? `Thông báo, ${unread} chưa đọc` : 'Thông báo'" @hui-click="show" />
  </hui-badge>

  <hui-drawer :open="open" title="Thông báo" size="420" @hui-close="open = false">
    <hui-button v-if="unread" slot="extra" size="sm" variant="link" :loading="readingAll" @hui-click="readAll">Đọc tất cả</hui-button>
    <hui-alert v-if="error" severity="error" :description="error" />
    <p v-if="loaded && !items.length" class="muted empty">Chưa có thông báo nào.</p>
    <ul class="notifications">
      <li v-for="n in items" :key="n.id">
        <button type="button" class="item" :class="{ unread: !n.readAt }" @click="openItem(n)">
          <hui-icon :name="NOTIFICATION_KIND_ICONS[n.kind] ?? 'info'" class="kind" aria-hidden="true" />
          <span class="text">
            <span class="title">{{ n.title }}</span>
            <span v-if="n.body" class="body">{{ n.body }}</span>
            <span class="meta">{{ NOTIFICATION_KIND_LABELS[n.kind] ?? 'Thông báo' }} · {{ ago(n.createdAt) }}</span>
          </span>
          <span v-if="!n.readAt" class="dot" aria-label="Chưa đọc" />
        </button>
      </li>
    </ul>
    <div v-if="items.length < total" class="more">
      <hui-button size="sm" variant="outline" :loading="loading" @hui-click="loadMore">Xem thêm</hui-button>
    </div>
  </hui-drawer>
</template>

<script setup>
import { onMounted, onUnmounted, ref } from 'vue';
import { useRouter } from 'vue-router';

import { notificationApi } from '../api';
import { NOTIFICATION_KIND_ICONS, NOTIFICATION_KIND_LABELS } from '../utils/schedule';

/** Hỏi lại số chưa đọc mỗi 60 giây (WebSocket làm cùng phòng live). */
const POLL_MS = 60_000;
const PAGE_SIZE = 20;

const router = useRouter();
const unread = ref(0);
const open = ref(false);
const items = ref([]);
const total = ref(0);
const page = ref(1);
const loaded = ref(false);
const loading = ref(false);
const readingAll = ref(false);
const error = ref('');
let timer = null;

async function refreshCount() {
  try {
    unread.value = (await notificationApi.unreadCount()).unread;
  } catch {
    /* mất mạng / hết phiên: bỏ qua, lần sau hỏi lại */
  }
}

async function load(p = 1) {
  loading.value = true;
  error.value = '';
  try {
    const res = await notificationApi.list({ page: p, pageSize: PAGE_SIZE });
    items.value = p === 1 ? res.items : [...items.value, ...res.items];
    total.value = res.total;
    page.value = p;
  } catch (e) {
    error.value = e.message;
  } finally {
    loading.value = false;
    loaded.value = true;
  }
}

function show() {
  open.value = true;
  load(1);
  refreshCount();
}

const loadMore = () => load(page.value + 1);

async function openItem(n) {
  if (!n.readAt) {
    n.readAt = new Date().toISOString();
    unread.value = Math.max(0, unread.value - 1);
    notificationApi.read(n.id).catch(() => {});
  }
  if (n.link) {
    open.value = false;
    router.push(n.link);
  }
}

async function readAll() {
  readingAll.value = true;
  try {
    await notificationApi.readAll();
    const now = new Date().toISOString();
    for (const n of items.value) n.readAt ??= now;
    unread.value = 0;
  } finally {
    readingAll.value = false;
  }
}

function ago(value) {
  const min = Math.round((Date.now() - new Date(value).getTime()) / 60_000);
  if (min < 1) return 'vừa xong';
  if (min < 60) return `${min} phút trước`;
  const h = Math.round(min / 60);
  if (h < 24) return `${h} giờ trước`;
  return new Date(value).toLocaleDateString('vi-VN');
}

function onVisible() {
  if (document.visibilityState === 'visible') refreshCount();
}

onMounted(() => {
  refreshCount();
  timer = setInterval(() => document.visibilityState === 'visible' && refreshCount(), POLL_MS);
  document.addEventListener('visibilitychange', onVisible);
});
onUnmounted(() => {
  clearInterval(timer);
  document.removeEventListener('visibilitychange', onVisible);
});
</script>

<style scoped>
.notifications {
  list-style: none;
  margin: 0;
  padding: 0;
  display: grid;
  gap: 4px;
}
.item {
  display: flex;
  gap: 10px;
  align-items: flex-start;
  width: 100%;
  padding: 10px;
  border: 0;
  border-radius: var(--hui-radius-md, 8px);
  background: transparent;
  color: inherit;
  font: inherit;
  text-align: left;
  cursor: pointer;
}
.item:hover {
  background: var(--hui-fill-secondary, #f3f4f6);
}
.item.unread {
  background: var(--hui-fill-tertiary, #f5f8ff);
}
.kind {
  flex: none;
  margin-top: 2px;
  font-size: 1.1rem;
  color: var(--hui-primary);
}
.text {
  display: grid;
  gap: 2px;
  flex: 1;
  min-width: 0;
}
.title {
  font-weight: 600;
}
.body {
  font-size: 0.875rem;
  color: var(--hui-text-secondary, #555);
}
.meta {
  font-size: 0.75rem;
  color: var(--hui-text-muted, #777);
}
.dot {
  flex: none;
  width: 8px;
  height: 8px;
  margin-top: 6px;
  border-radius: 50%;
  background: var(--hui-primary);
}
.empty {
  text-align: center;
  padding: 24px 0;
}
.more {
  display: flex;
  justify-content: center;
  padding-top: 12px;
}
</style>
