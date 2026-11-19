<template>
  <div class="page">
    <div class="page-header">
      <div class="stack" style="gap: 4px">
        <RouterLink :to="{ name: 'session', params: { id: route.params.id } }" class="small">← Về buổi học</RouterLink>
        <h1>{{ lesson?.title ?? 'Bài học' }}</h1>
      </div>
      <hui-segmented v-if="lesson" :options="VIEW_OPTIONS" :value="view" @change="view = $event.detail.value" />
    </div>
    <hui-alert v-if="error" severity="error" :description="error" />

    <template v-if="lesson">
      <section v-if="view === 'slides'" class="panel">
        <LessonSlides :content="lesson.content" :index="index" @change="index = $event" />
      </section>
      <section v-else class="panel stack">
        <p v-if="lesson.summary" class="muted">{{ lesson.summary }}</p>
        <div v-if="video" class="video">
          <iframe v-if="video.type === 'youtube'" :src="video.src" title="Video bài giảng" allowfullscreen loading="lazy" />
          <video v-else :src="video.src" controls preload="metadata" />
        </div>
        <hui-markdown :value="lesson.content" />
      </section>
      <section v-if="lesson.attachments?.length" class="panel">
        <AttachmentList :modelValue="lesson.attachments" />
      </section>
    </template>
  </div>
</template>

<script setup>
/** Học sinh xem bài đã gắn vào buổi học (xem trước / xem lại), có chế độ trình chiếu. */
import { computed, onMounted, ref } from 'vue';
import { useRoute } from 'vue-router';

import { sessionApi } from '../../api';
import AttachmentList from '../../components/teaching/AttachmentList.vue';
import LessonSlides from '../../components/teaching/LessonSlides.vue';
import { videoEmbed } from '../../utils/teaching';

const VIEW_OPTIONS = [
  { value: 'read', label: 'Đọc bài' },
  { value: 'slides', label: 'Trình chiếu' },
];

const route = useRoute();
const lesson = ref(null);
const error = ref('');
const view = ref('read');
const index = ref(0);
const video = computed(() => videoEmbed(lesson.value?.videoUrl?.trim()));

onMounted(async () => {
  try {
    lesson.value = await sessionApi.lesson(route.params.id, route.params.lessonId);
  } catch (e) {
    error.value = e.message;
  }
});
</script>

<style scoped>
.video {
  aspect-ratio: 16 / 9;
  max-width: 820px;
}
.video iframe,
.video video {
  width: 100%;
  height: 100%;
  border: 0;
  border-radius: var(--hui-radius-md, 8px);
}
</style>
