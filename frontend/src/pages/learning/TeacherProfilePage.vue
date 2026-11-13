<template>
  <div class="page">
    <hui-alert v-if="error" severity="error" :description="error" />
    <template v-if="t">
      <section class="panel profile">
        <img v-if="t.avatarUrl" :src="t.avatarUrl" :alt="t.fullName" class="avatar" />
        <div v-else class="avatar initials" aria-hidden="true">{{ initials }}</div>
        <div class="stack" style="gap: 4px">
          <h1 style="margin: 0">{{ t.fullName }}</h1>
          <span v-if="t.headline" class="muted">{{ t.headline }}</span>
          <span class="row">
            <hui-tag v-for="s in t.subjects" :key="s" size="sm" color="primary">{{ s }}</hui-tag>
          </span>
          <span class="small">
            <a :href="`/p/giao-vien/${t.username}`" target="_blank" rel="noopener">Trang công khai</a>
            <template v-if="t.isMe"> · <RouterLink :to="{ name: 'profile', hash: '#ho-so-giao-vien' }">Sửa hồ sơ</RouterLink></template>
          </span>
        </div>
      </section>

      <section v-if="t.bio" class="panel"><hui-markdown :value="t.bio" /></section>

      <section v-if="t.offers.length" class="stack">
        <h2 style="margin: 0">Học kèm 1-1</h2>
        <div class="card-list">
          <article v-for="o in t.offers" :key="o.id" class="card">
            <h3 style="margin: 0">{{ o.title }}</h3>
            <hui-tag size="sm">{{ formatDuration(o.durationMinutes) }} / buổi</hui-tag>
            <p class="small" style="margin: 0">{{ formatWindows(o.windows) }}</p>
            <p v-if="o.description" class="muted small" style="margin: 0">{{ o.description }}</p>
            <hui-button v-if="!t.isMe" size="sm" @hui-click="book(o)">Chọn giờ</hui-button>
          </article>
        </div>
      </section>

      <section class="stack">
        <h2 style="margin: 0">Lớp học công khai</h2>
        <p v-if="!t.classes.length" class="panel empty muted">Chưa có lớp học công khai.</p>
        <div class="card-list">
          <article v-for="c in t.classes" :key="c.id" class="card">
            <RouterLink class="title" :to="{ name: 'class', params: { id: c.id } }"><h3>{{ c.title }}</h3></RouterLink>
            <div class="row">
              <hui-tag size="sm">{{ c.subjectName }}</hui-tag>
              <hui-tag size="sm" color="secondary">{{ MODE_LABELS[c.mode] }}</hui-tag>
              <span class="muted small">{{ c.studentCount }} học sinh</span>
            </div>
            <p v-if="c.description" class="muted small clamp" style="margin: 0">{{ c.description }}</p>
          </article>
        </div>
      </section>
    </template>
    <BookingModal v-model:open="bookingOpen" :offer="selected" @booked="$router.push({ name: 'tutoring' })" />
  </div>
</template>

<script setup>
import { computed, onUnmounted, ref, watch } from 'vue';
import { useRoute } from 'vue-router';

import { publicApi } from '../../api';
import BookingModal from '../../components/learning/BookingModal.vue';
import { formatDuration, formatWindows } from '../../utils/schedule';
import { MODE_LABELS } from '../../utils/teaching';

const route = useRoute();
const t = ref(null);
const error = ref('');
const bookingOpen = ref(false);
const selected = ref(null);

const initials = computed(() =>
  (t.value?.fullName ?? '')
    .split(/\s+/)
    .filter(Boolean)
    .slice(-2)
    .map((w) => w[0].toUpperCase())
    .join(''),
);

function book(o) {
  selected.value = o;
  bookingOpen.value = true;
}

async function load() {
  error.value = '';
  try {
    t.value = await publicApi.teacher(route.params.key);
    document.title = `${t.value.fullName} — giáo viên — Smart Learning Space`;
  } catch (e) {
    error.value = e.status === 404 ? 'Không tìm thấy giáo viên (hoặc giáo viên chưa công khai hồ sơ).' : e.message;
  }
}

watch(() => route.params.key, (k) => k && load(), { immediate: true });
onUnmounted(() => (document.title = 'Smart Learning Space'));
</script>

<style scoped>
.profile {
  display: flex;
  gap: 16px;
  align-items: center;
  flex-wrap: wrap;
}
.avatar {
  width: 96px;
  height: 96px;
  border-radius: 50%;
  object-fit: cover;
  background: var(--hui-bg-secondary, #f5f6f8);
}
.initials {
  display: grid;
  place-items: center;
  font-size: 2rem;
  font-weight: 700;
  color: var(--hui-primary, #0864ad);
}
.clamp {
  display: -webkit-box;
  -webkit-line-clamp: 3;
  -webkit-box-orient: vertical;
  overflow: hidden;
}
</style>
