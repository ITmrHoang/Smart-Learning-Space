<template>
  <div class="page study">
    <RouterLink :to="{ name: 'class', params: { id: classId } }" class="small">← Về lớp học</RouterLink>
    <hui-alert v-if="error" severity="error" :description="error" />
    <template v-if="deck">
      <div class="row between">
        <h1 style="margin: 0">{{ deck.title }}</h1>
        <span class="row">
          <hui-tag color="info">{{ deck.cards.length }} thẻ</hui-tag>
          <hui-tag v-if="preview" color="primary">Giáo viên xem thử — không lưu</hui-tag>
        </span>
      </div>
      <p v-if="deck.description" class="muted" style="margin: 0">{{ deck.description }}</p>

      <template v-if="current">
        <div class="row between small">
          <span>Còn <b>{{ queue.length }}</b> thẻ trong lượt này · đã ôn {{ done }}</span>
          <span class="muted">{{ STATE_LABELS[current.state] }}</span>
        </div>
        <div class="progress-bar"><span :style="{ width: `${percent}%` }" /></div>
        <hui-flashcard
          ref="cardEl"
          :front="current.front"
          :back="current.back"
          :intervals="intervals"
          :disabled="saving"
          @hui-rate="rate($event.detail.rating)"
        />
        <p class="muted small" style="margin: 0">Lật thẻ rồi tự chấm: <b>Quên</b> sẽ gặp lại trong lượt này; nhớ càng tốt thì càng lâu mới phải ôn lại.</p>
      </template>

      <section v-else class="panel stack done">
        <h2 style="margin: 0">{{ done ? '🎉 Đã ôn xong lượt này!' : 'Chưa có thẻ nào đến hạn ôn' }}</h2>
        <p style="margin: 0">
          <template v-if="nextDue">Lần ôn tới: <b>{{ formatDateTime(nextDue) }}</b>. </template>
          Hệ thống sẽ nhắc thẻ cần ôn ở trang <RouterLink to="/on-tap">Ôn tập hôm nay</RouterLink>.
        </p>
        <div class="row">
          <hui-button variant="outline" @hui-click="practiceAll">Luyện thêm tất cả thẻ</hui-button>
          <RouterLink :to="{ name: 'class', params: { id: classId } }"><hui-button variant="text">Về lớp học</hui-button></RouterLink>
        </div>
      </section>
    </template>
  </div>
</template>

<script setup>
import { message } from '@itmrhoang/h-ui/hui-message';
import { computed, nextTick, onMounted, ref } from 'vue';
import { useRoute } from 'vue-router';

import { learnApi } from '../../api';
import { afterRating, buildQueue, formatInterval } from '../../utils/flashcards';
import { formatDateTime } from '../../utils/form';

const STATE_LABELS = { new: 'Thẻ mới', learning: 'Đang học lại', review: 'Thẻ ôn lại' };

const route = useRoute();
const classId = computed(() => route.params.id);
const deck = ref(null);
const queue = ref([]);
const done = ref(0);
const total = ref(0);
const saving = ref(false);
const error = ref('');
const nextDue = ref(null);
const preview = ref(false);
const cardEl = ref(null);

const current = computed(() => queue.value[0] ?? null);
const intervals = computed(() => (current.value?.nextMinutes ?? []).map(formatInterval));
const percent = computed(() => (total.value ? Math.round((done.value / total.value) * 100) : 0));

async function load() {
  error.value = '';
  try {
    const d = await learnApi.stepDeck(classId.value, route.params.stepId);
    deck.value = d;
    preview.value = false;
    queue.value = buildQueue(d.cards);
    total.value = queue.value.length;
    nextDue.value = d.nextDueAt;
  } catch (e) {
    error.value = e.message;
  }
}

async function rate(rating) {
  const card = current.value;
  if (!card || saving.value) return;
  saving.value = true;
  try {
    const res = await learnApi.review(classId.value, route.params.stepId, card.id, rating);
    preview.value = !res.saved;
    if (rating !== 'again') done.value += 1;
    if (!nextDue.value || new Date(res.dueAt) < new Date(nextDue.value)) nextDue.value = res.dueAt;
    // Lật về mặt trước ngay (không hiệu ứng) rồi mới đổi thẻ, để không lộ mặt sau của thẻ kế
    cardEl.value?.reset?.();
    await nextTick();
    queue.value = afterRating(queue.value, rating);
  } catch (e) {
    message.error(e.message);
  } finally {
    saving.value = false;
  }
}

function practiceAll() {
  queue.value = [...deck.value.cards];
  total.value = queue.value.length;
  done.value = 0;
}

onMounted(load);
</script>

<style scoped>
.study {
  max-width: 760px;
}
.done {
  text-align: left;
}
</style>
