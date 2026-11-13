<template>
  <div class="page">
    <div class="page-header">
      <h1>Ôn tập hôm nay</h1>
    </div>
    <p class="muted small">Các bộ thẻ ghi nhớ trong lớp học của bạn. Ôn đều mỗi ngày vài phút giúp nhớ lâu hơn học dồn một lần.</p>
    <hui-alert v-if="error" severity="error" :description="error" />
    <div v-if="loaded && !decks.length" class="panel empty muted">Các lớp học của bạn chưa có bộ thẻ ghi nhớ nào.</div>
    <div class="card-list">
      <article v-for="d in decks" :key="d.deckId" class="card">
        <h3>{{ d.title }}</h3>
        <p class="muted small" style="margin: 0">{{ d.classTitle }} · {{ d.total }} thẻ</p>
        <div class="row">
          <hui-tag v-if="d.due" size="sm" color="warning">{{ d.due }} thẻ đến hạn</hui-tag>
          <hui-tag v-if="d.newCount" size="sm" color="info">{{ d.newCount }} thẻ mới</hui-tag>
          <hui-tag v-if="!d.due && !d.newCount" size="sm" color="success">Đã ôn xong</hui-tag>
        </div>
        <RouterLink :to="{ name: 'deck-study', params: { id: d.classId, stepId: d.stepId } }">
          {{ d.due || d.newCount ? 'Ôn ngay →' : 'Luyện thêm →' }}
        </RouterLink>
      </article>
    </div>
  </div>
</template>

<script setup>
import { onMounted, ref } from 'vue';

import { learnApi } from '../../api';

const decks = ref([]);
const loaded = ref(false);
const error = ref('');

onMounted(async () => {
  try {
    decks.value = await learnApi.myReviews();
  } catch (e) {
    error.value = e.message;
  } finally {
    loaded.value = true;
  }
});
</script>
