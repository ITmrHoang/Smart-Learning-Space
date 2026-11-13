<template>
  <section id="ho-so-giao-vien" class="panel stack">
    <div class="row between">
      <h2 style="margin: 0">Hồ sơ giáo viên công khai</h2>
      <span v-if="profile" class="small">
        <a :href="profile.publicPath" target="_blank" rel="noopener">Trang công khai</a> ·
        <RouterLink :to="{ name: 'teacher-profile', params: { key: profile.username } }">Xem trong ứng dụng</RouterLink>
      </span>
    </div>
    <p class="muted small" style="margin: 0">
      Học sinh và máy tìm kiếm (Google…) thấy trang này cùng các lớp học công khai và lịch học kèm mở cho mọi tài khoản.
    </p>
    <hui-alert v-if="error" severity="error" :description="error" />
    <template v-if="profile">
      <div class="row">
        <img v-if="avatarUrl" :src="avatarUrl" alt="Ảnh đại diện" class="avatar" />
        <div v-else class="avatar empty" aria-hidden="true">?</div>
        <span class="row">
          <hui-button size="sm" variant="outline" :loading="uploading" @hui-click="avatarInput?.click()">Đổi ảnh đại diện</hui-button>
          <hui-button v-if="form.avatarFileId" size="sm" variant="text" @hui-click="clearAvatar">Bỏ ảnh</hui-button>
          <input ref="avatarInput" type="file" accept="image/png,image/jpeg,image/webp" hidden @change="onAvatar" />
        </span>
      </div>
      <hui-input v-model="form.headline" label="Một dòng giới thiệu" placeholder="Ví dụ: Giáo viên Toán THCS, 10 năm ôn thi vào 10" :error="fields.headline" />
      <MarkdownEditor v-model="form.bio" label="Giới thiệu" :rows="6" compact :error="fields.bio" placeholder="Kinh nghiệm, cách dạy, thành tích của học sinh…" />
      <label class="small"><input v-model="form.isPublic" type="checkbox" /> Công khai hồ sơ (bỏ chọn = chỉ mình tôi xem được)</label>
      <div class="row">
        <hui-button :loading="loading" @hui-click="save">Lưu hồ sơ</hui-button>
      </div>
    </template>
  </section>
</template>

<script setup>
import { message } from '@itmrhoang/h-ui/hui-message';
import { onMounted, reactive, ref } from 'vue';

import { teachingApi } from '../api';
import { useSubmit } from '../utils/form';
import { uploadFile } from '../utils/upload';
import MarkdownEditor from './teaching/MarkdownEditor.vue';

const { loading, error, fields, run } = useSubmit();
const profile = ref(null);
const avatarUrl = ref('');
const uploading = ref(false);
const avatarInput = ref(null);
const form = reactive({ headline: '', bio: '', avatarFileId: null, isPublic: true });

function apply(p) {
  profile.value = p;
  avatarUrl.value = p.avatarUrl ?? '';
  Object.assign(form, { headline: p.headline, bio: p.bio, avatarFileId: p.avatarFileId, isPublic: p.isPublic });
}

async function onAvatar(e) {
  const file = e.target.files[0];
  e.target.value = '';
  if (!file) return;
  uploading.value = true;
  try {
    const saved = await uploadFile(file, 'content');
    form.avatarFileId = saved.id;
    avatarUrl.value = saved.url;
  } catch (err) {
    message.error(err.message);
  } finally {
    uploading.value = false;
  }
}

function clearAvatar() {
  form.avatarFileId = null;
  avatarUrl.value = '';
}

async function save() {
  const res = await run(() => teachingApi.updateProfile({ ...form, headline: form.headline.trim(), bio: form.bio.trim() }));
  if (res.ok) {
    apply(res.data);
    message.success('Đã lưu hồ sơ giáo viên.');
  }
}

onMounted(async () => {
  try {
    apply(await teachingApi.profile());
  } catch (e) {
    error.value = e.message;
  }
});
</script>

<style scoped>
.avatar {
  width: 72px;
  height: 72px;
  border-radius: 50%;
  object-fit: cover;
  background: var(--hui-bg-secondary, #f5f6f8);
}
.avatar.empty {
  display: grid;
  place-items: center;
  font-size: 1.6rem;
  color: var(--hui-text-muted, #9ca3af);
}
</style>
