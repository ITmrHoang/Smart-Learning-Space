<template>
  <section v-if="data && (data.board.length || data.polls.length || data.boardTurns.length)" class="panel stack">
    <h2 style="margin: 0">Lưu lại từ buổi học</h2>
    <div v-if="data.board.length" class="stack">
      <b class="small">Bảng ({{ data.board.length }} trang)</b>
      <div class="pages">
        <a v-for="p in data.board" :key="p.page" :href="p.imageUrl" target="_blank" rel="noopener" class="page">
          <img :src="p.imageUrl" :alt="`Bảng trang ${p.page + 1}`" loading="lazy" />
          <span class="small muted">Trang {{ p.page + 1 }}</span>
        </a>
      </div>
    </div>
    <div v-if="data.polls.length" class="stack">
      <b class="small">Bình chọn / quiz</b>
      <article v-for="poll in data.polls" :key="poll.id" class="poll">
        <div class="row">
          <hui-tag size="sm" :color="poll.kind === 'quiz' ? 'warning' : 'info'">{{ poll.kind === 'quiz' ? 'Quiz' : 'Bình chọn' }}</hui-tag>
          <hui-markdown inline :value="poll.question" />
          <span class="muted small">· {{ poll.voters }} người</span>
        </div>
        <div v-for="(o, i) in poll.options" :key="i" class="bar-row" :class="{ correct: poll.correct.includes(i) }">
          <span class="opt">{{ String.fromCharCode(65 + i) }}. {{ o }}</span>
          <span class="bar"><span :style="{ width: `${percent(poll.counts[i], poll.voters)}%` }" /></span>
          <span class="small">{{ poll.counts[i] }} ({{ percent(poll.counts[i], poll.voters) }}%)</span>
        </div>
      </article>
    </div>
    <div v-if="data.boardTurns.length" class="stack">
      <b class="small">Lên bảng</b>
      <span class="small">{{ data.boardTurns.map((t) => `${t.fullName} (${t.turns} lần)`).join(' · ') }}</span>
    </div>
  </section>
</template>

<script setup>
/** Bảng đã lưu, kết quả bình chọn / quiz, lượt lên bảng (giáo viên) của một buổi học trong phòng của hệ thống. */
import { onMounted, ref } from 'vue';

import { liveApi } from '../../api';
import { percent } from '../../utils/live';

const props = defineProps({ sessionId: { type: String, required: true } });
const data = ref(null);

onMounted(async () => {
  data.value = await liveApi.records(props.sessionId).catch(() => null);
});
</script>

<style scoped>
.pages {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(200px, 1fr));
  gap: 10px;
}
.page {
  display: grid;
  gap: 4px;
  text-decoration: none;
}
.page img {
  width: 100%;
  aspect-ratio: 16 / 9;
  object-fit: contain;
  border: 1px solid var(--hui-border-secondary, #eceef1);
  border-radius: 8px;
  background: #fff;
}
.poll {
  display: grid;
  gap: 4px;
  padding: 8px 0;
  border-bottom: 1px dashed var(--hui-border-secondary, #eceef1);
}
.bar-row {
  display: grid;
  grid-template-columns: minmax(0, 1fr) 140px auto;
  gap: 8px;
  align-items: center;
}
.bar-row.correct .opt {
  font-weight: 700;
  color: var(--hui-success, #16a34a);
}
.bar {
  height: 8px;
  border-radius: 999px;
  background: var(--hui-bg-secondary, #f0f2f5);
  overflow: hidden;
}
.bar span {
  display: block;
  height: 100%;
  background: var(--hui-primary, #0864ad);
}
</style>
