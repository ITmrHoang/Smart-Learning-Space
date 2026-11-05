<template>
  <article class="card">
    <RouterLink class="title" :to="{ name: 'class', params: { id: cls.id } }"><h3>{{ cls.title }}</h3></RouterLink>
    <div class="row">
      <hui-tag size="sm">{{ subjectName }}</hui-tag>
      <hui-tag size="sm" color="info">{{ MODE_LABELS[cls.mode] }}</hui-tag>
      <hui-tag v-if="cls.myStatus" size="sm" :color="MEMBER_STATUS_COLORS[cls.myStatus]">{{ MEMBER_STATUS_LABELS[cls.myStatus] }}</hui-tag>
      <hui-tag v-if="cls.isOwner" size="sm" color="primary">Lớp của bạn</hui-tag>
    </div>
    <p class="muted small">Giáo viên {{ cls.teacherName }} · {{ cls.stepCount }} bài / đề · {{ cls.studentCount }} học sinh</p>
    <template v-if="progress">
      <div class="progress-bar"><span :style="{ width: `${progress.percent}%` }" /></div>
      <span class="small muted">Đã xong {{ progress.done }}/{{ progress.total }} ({{ progress.percent }}%)</span>
    </template>
    <slot />
  </article>
</template>

<script setup>
/** Thẻ lớp học cho học sinh (tạm làm trong app; sau tách thành `hui-class-card` trong H-UI). */
import { computed } from 'vue';

import { useMasterData } from '../../utils/masterData';
import { MEMBER_STATUS_COLORS, MEMBER_STATUS_LABELS, MODE_LABELS } from '../../utils/teaching';

const props = defineProps({
  cls: { type: Object, required: true },
  progress: { type: Object, default: null },
});
const { data: master } = useMasterData();
const subjectName = computed(() => master.value?.subjects.find((s) => s.id === props.cls.subjectId)?.name ?? '');
</script>
