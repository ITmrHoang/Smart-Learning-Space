<template>
  <div class="page">
    <div class="page-header">
      <h1>Thẻ ghi nhớ</h1>
      <RouterLink :to="{ name: 'teach-deck', params: { id: 'moi' } }"><hui-button>Tạo bộ thẻ</hui-button></RouterLink>
    </div>
    <p class="muted small">
      Bộ thẻ (công thức, định nghĩa, từ vựng…) để học sinh ôn theo kiểu lặp lại ngắt quãng: nhớ tốt thì lâu mới ôn lại, quên thì ôn sớm.
      Thêm bộ thẻ vào lộ trình của lớp học bằng nút "+ Thẻ ghi nhớ".
    </p>
    <div class="toolbar">
      <hui-input class="grow" type="search" label="Tìm" clearable @hui-search="search($event.detail.value)" @clear="search('')" />
    </div>
    <hui-alert v-if="error" severity="error" :description="error" />
    <div v-if="loaded && !decks.length" class="panel empty muted">Chưa có bộ thẻ nào.</div>
    <div class="card-list">
      <article v-for="d in decks" :key="d.id" class="card">
        <RouterLink class="title" :to="{ name: 'teach-deck', params: { id: d.id } }"><h3>{{ d.title }}</h3></RouterLink>
        <div class="row">
          <hui-tag size="sm">{{ subjectName(d.subjectId) }}</hui-tag>
          <hui-tag size="sm" :color="d.cardCount ? 'info' : 'warning'">{{ d.cardCount }} thẻ</hui-tag>
        </div>
        <p v-if="d.description" class="muted small" style="margin: 0">{{ d.description }}</p>
        <p class="muted small" style="margin: 0">Cập nhật {{ formatDateTime(d.updatedAt) }}</p>
      </article>
    </div>
  </div>
</template>

<script setup>
import { onMounted, ref } from 'vue';

import { teachingApi } from '../../api';
import { formatDateTime } from '../../utils/form';
import { useMasterData } from '../../utils/masterData';

const { data: master, load: loadMaster } = useMasterData();
const decks = ref([]);
const loaded = ref(false);
const error = ref('');
const subjectName = (id) => master.value?.subjects.find((s) => s.id === id)?.name ?? '';

async function search(q = '') {
  error.value = '';
  try {
    decks.value = await teachingApi.decks({ q: q || undefined });
  } catch (e) {
    error.value = e.message;
  } finally {
    loaded.value = true;
  }
}

onMounted(() => {
  loadMaster();
  search();
});
</script>
