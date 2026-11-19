<template>
  <hui-slides
    ref="el"
    class="lesson-slides"
    :index="index"
    :readonly="readonly"
    :noControls="noControls"
    @hui-slide-change="emit('change', $event.detail.index)"
  >
    <section v-for="(md, i) in slides" :key="i" class="slide">
      <hui-markdown :value="md" />
    </section>
  </hui-slides>
</template>

<script setup>
/** Bài học (Markdown) thành slide trình chiếu bằng `hui-slides` — dùng ở trang trình chiếu, trang buổi học, phòng live. */
import { computed, ref } from 'vue';

import { splitSlides } from '../../utils/slides';

const props = defineProps({
  content: { type: String, default: '' },
  index: { type: Number, default: 0 },
  /** Học sinh xem theo giáo viên: không tự chuyển slide */
  readonly: { type: Boolean, default: false },
  noControls: { type: Boolean, default: false },
});
const emit = defineEmits(['change']);
const el = ref(null);
const slides = computed(() => splitSlides(props.content));

defineExpose({
  total: () => slides.value.length,
  toggleFullscreen: () => el.value?.toggleFullscreen(),
  focus: () => el.value?.focus(),
});
</script>

<style scoped>
.slide {
  /* Slide dài hơn khung 16:9: cuộn trong slide thay vì bị cắt */
  overflow: auto !important;
}
</style>
