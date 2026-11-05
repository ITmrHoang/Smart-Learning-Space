<template>
  <div class="page lesson-page">
    <RouterLink :to="{ name: 'class', params: { id: route.params.id } }" class="small">← Về lộ trình</RouterLink>
    <hui-alert v-if="error" severity="error" :description="error" />
    <template v-if="data">
      <div class="page-header">
        <h1>{{ data.lesson.title }}</h1>
        <hui-tag v-if="data.step.status === 'done'" color="success">Đã học xong</hui-tag>
      </div>
      <p v-if="data.lesson.summary" class="muted">{{ data.lesson.summary }}</p>
      <div v-if="video" class="video">
        <iframe
          v-if="video.type === 'youtube'"
          :src="video.src"
          title="Video bài giảng"
          allow="accelerometer; encrypted-media; gyroscope; picture-in-picture"
          allowfullscreen
        />
        <video v-else :src="video.src" controls preload="metadata" />
      </div>
      <section class="panel">
        <hui-markdown :value="data.lesson.content" />
      </section>
      <div class="row between">
        <hui-button v-if="data.prevStepId" variant="outline" @hui-click="go(data.prevStepId)">← Bài trước</hui-button>
        <span v-else />
        <div class="row">
          <hui-button v-if="data.step.status !== 'done' && !data.isPreview" :loading="completing" @hui-click="complete">Đã học xong</hui-button>
          <hui-button v-if="data.nextStepId" variant="outline" @hui-click="goNext">Tiếp theo →</hui-button>
        </div>
      </div>
      <p class="muted small">Phiên bản {{ data.lesson.version }} · cập nhật {{ formatDateTime(data.lesson.publishedAt) }}</p>
    </template>
  </div>
</template>

<script setup>
import { message } from '@itmrhoang/h-ui/hui-message';
import { computed, ref, watch } from 'vue';
import { useRoute, useRouter } from 'vue-router';

import { learnApi } from '../../api';
import { formatDateTime } from '../../utils/form';
import { videoEmbed } from '../../utils/teaching';

const route = useRoute();
const router = useRouter();
const data = ref(null);
const error = ref('');
const completing = ref(false);
const video = computed(() => videoEmbed(data.value?.lesson.videoUrl));

async function load() {
  error.value = '';
  data.value = null;
  try {
    data.value = await learnApi.stepLesson(route.params.id, route.params.stepId);
  } catch (e) {
    error.value = e.message;
  }
}

async function complete() {
  completing.value = true;
  try {
    await learnApi.completeStep(route.params.id, route.params.stepId);
    data.value.step.status = 'done';
    message.success('Đã đánh dấu học xong.');
  } catch (e) {
    message.error(e.message);
  } finally {
    completing.value = false;
  }
}

/** Bước tiếp theo có thể là đề → về lộ trình để bấm "Làm bài". */
async function goNext() {
  try {
    await learnApi.stepLesson(route.params.id, data.value.nextStepId);
    go(data.value.nextStepId);
  } catch {
    router.push({ name: 'class', params: { id: route.params.id } });
  }
}

function go(stepId) {
  router.push({ name: 'lesson-view', params: { id: route.params.id, stepId } });
}

watch(() => route.params.stepId, load, { immediate: true });
</script>

<style scoped>
.lesson-page {
  max-width: 900px;
}
.video {
  aspect-ratio: 16 / 9;
  background: #000;
  border-radius: var(--hui-radius-lg, 12px);
  overflow: hidden;
}
.video iframe,
.video video {
  width: 100%;
  height: 100%;
  border: 0;
}
</style>
