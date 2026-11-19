<template>
  <div class="present">
    <header class="bar">
      <RouterLink :to="{ name: 'teach-lesson', params: { id: route.params.id } }" class="small">← Quay lại soạn bài</RouterLink>
      <b class="title">{{ lesson?.title }}</b>
      <span class="muted small">← → / Space: chuyển slide · F: toàn màn hình · G: xem tất cả</span>
      <hui-button size="sm" :disabled="!lesson" @hui-click="slides?.toggleFullscreen()">Toàn màn hình</hui-button>
    </header>
    <hui-alert v-if="error" severity="error" :description="error" />
    <main v-if="lesson" class="stage">
      <LessonSlides ref="slides" :content="lesson.content" :index="index" @change="index = $event" />
    </main>
  </div>
</template>

<script setup>
/** Giáo viên trình chiếu bản nháp đang soạn (không cần xuất bản). */
import { nextTick, onBeforeUnmount, onMounted, ref } from 'vue';
import { useRoute } from 'vue-router';

import { teachingApi } from '../../api';
import LessonSlides from '../../components/teaching/LessonSlides.vue';

const route = useRoute();
const lesson = ref(null);
const error = ref('');
const index = ref(0);
const slides = ref(null);
const previousTitle = document.title;

onMounted(async () => {
  try {
    lesson.value = await teachingApi.lesson(route.params.id);
    document.title = `${lesson.value.title} — trình chiếu`;
    await nextTick();
    slides.value?.focus();
  } catch (e) {
    error.value = e.message;
  }
});

onBeforeUnmount(() => {
  document.title = previousTitle;
});
</script>

<style scoped>
.present {
  min-height: 100vh;
  display: flex;
  flex-direction: column;
  background: #e5e7eb;
}
.bar {
  display: flex;
  gap: 16px;
  align-items: center;
  flex-wrap: wrap;
  padding: 8px 16px;
  background: #fff;
}
.title {
  flex: 1;
}
.stage {
  flex: 1;
  display: grid;
  place-items: center;
  padding: 16px;
}
.stage > * {
  width: min(100%, calc((100vh - 120px) * 16 / 9));
}
</style>
