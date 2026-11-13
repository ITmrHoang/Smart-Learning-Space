<template>
  <section class="panel stack">
    <div class="row between">
      <h2 style="margin: 0">Phụ huynh nhận báo cáo học tập</h2>
      <hui-button v-if="list.length < 3 && !formOpen" size="sm" @hui-click="openForm()">Thêm phụ huynh</hui-button>
    </div>
    <p class="muted small" style="margin: 0">
      Mỗi tối Chủ nhật phụ huynh nhận email báo cáo tuần (tiến độ, điểm, buổi học). Phụ huynh phải bấm link xác nhận trong email
      thì mới nhận báo cáo, và có thể hủy nhận bất cứ lúc nào. Tối đa 3 người.
    </p>
    <hui-alert v-if="error" severity="error" :description="error" />

    <ul v-if="list.length" class="clean-list">
      <li v-for="g in list" :key="g.id" class="guardian">
        <div class="stack" style="gap: 2px">
          <span><b>{{ g.fullName }}</b><span v-if="g.relation" class="muted"> ({{ g.relation }})</span></span>
          <span class="muted small">{{ g.email }}<template v-if="g.phone"> · {{ g.phone }}</template></span>
          <span class="small">
            <hui-tag size="sm" :color="STATUS_COLORS[g.consentStatus]">{{ STATUS_LABELS[g.consentStatus] }}</hui-tag>
            <span v-if="g.consentStatus === 'confirmed' && !g.receiveReports" class="muted"> · phụ huynh đã hủy nhận báo cáo</span>
            <span v-else-if="g.lastReportAt" class="muted"> · báo cáo gần nhất {{ formatDateTime(g.lastReportAt) }}</span>
            <span v-else-if="g.consentStatus === 'pending' && g.consentSentAt" class="muted"> · đã gửi email xác nhận {{ formatDateTime(g.consentSentAt) }}</span>
          </span>
        </div>
        <span class="row">
          <hui-button v-if="g.consentStatus !== 'confirmed'" size="sm" variant="text" :loading="busy === g.id" @hui-click="resend(g)">Gửi lại email xác nhận</hui-button>
          <hui-button size="sm" variant="text" @hui-click="openForm(g)">Sửa</hui-button>
          <hui-button size="sm" variant="text" @hui-click="remove(g)">Xóa</hui-button>
        </span>
      </li>
    </ul>
    <p v-else-if="loaded && !formOpen" class="muted small" style="margin: 0">Chưa khai phụ huynh nào.</p>

    <form v-if="formOpen" class="form-grid add" novalidate @submit.prevent="save">
      <div class="grid-2">
        <hui-input v-model="form.fullName" label="Họ tên phụ huynh" :error="fields.fullName" required />
        <hui-input v-model="form.relation" label="Quan hệ" placeholder="Bố, mẹ, ông, bà…" :error="fields.relation" />
      </div>
      <div class="grid-2">
        <hui-input v-model="form.email" type="email" label="Email" :disabled="!!editing" :error="fields.email" required />
        <hui-input v-model="form.phone" label="Số điện thoại (không bắt buộc)" :error="fields.phone" />
      </div>
      <p v-if="editing" class="muted small" style="margin: 0">Muốn đổi email: xóa rồi thêm lại để phụ huynh xác nhận email mới.</p>
      <div class="row">
        <hui-button type="submit" :loading="loading">{{ editing ? 'Lưu' : 'Thêm và gửi email xác nhận' }}</hui-button>
        <hui-button variant="text" @hui-click="formOpen = false">Hủy</hui-button>
      </div>
    </form>
  </section>
</template>

<script setup>
import { message } from '@itmrhoang/h-ui/hui-message';
import { onMounted, reactive, ref } from 'vue';

import { guardianApi } from '../api';
import { formatDateTime, useSubmit } from '../utils/form';

const STATUS_LABELS = { pending: 'Chờ phụ huynh xác nhận', confirmed: 'Đã xác nhận', declined: 'Phụ huynh từ chối' };
const STATUS_COLORS = { pending: 'warning', confirmed: 'success', declined: 'danger' };

const list = ref([]);
const loaded = ref(false);
const error = ref('');
const busy = ref('');
const formOpen = ref(false);
const editing = ref(null);
const form = reactive({ fullName: '', relation: '', email: '', phone: '' });
const { loading, fields, run, reset, error: formError } = useSubmit();

async function load() {
  try {
    list.value = await guardianApi.mine();
  } catch (e) {
    error.value = e.message;
  } finally {
    loaded.value = true;
  }
}

function openForm(g = null) {
  reset();
  editing.value = g;
  Object.assign(form, { fullName: g?.fullName ?? '', relation: g?.relation ?? '', email: g?.email ?? '', phone: g?.phone ?? '' });
  formOpen.value = true;
}

async function save() {
  const res = await run(() =>
    editing.value
      ? guardianApi.update(editing.value.id, { fullName: form.fullName, relation: form.relation, phone: form.phone })
      : guardianApi.add({ ...form }),
  );
  if (!res.ok) {
    if (formError.value) message.error(formError.value);
    return;
  }
  message.success(editing.value ? 'Đã lưu.' : `Đã gửi email xác nhận tới ${res.data.email}.`);
  formOpen.value = false;
  load();
}

async function resend(g) {
  busy.value = g.id;
  try {
    await guardianApi.resend(g.id);
    message.success('Đã gửi lại email xác nhận.');
    load();
  } catch (e) {
    message.error(e.message);
  } finally {
    busy.value = '';
  }
}

async function remove(g) {
  if (!window.confirm(`Xóa ${g.fullName}? Phụ huynh sẽ không nhận báo cáo nữa.`)) return;
  try {
    await guardianApi.remove(g.id);
    load();
  } catch (e) {
    message.error(e.message);
  }
}

onMounted(load);
</script>

<style scoped>
.guardian {
  display: flex;
  flex-wrap: wrap;
  justify-content: space-between;
  gap: 8px;
  padding: 10px 0;
  border-bottom: 1px solid var(--hui-border-secondary, #eceef1);
}
.add {
  padding: 12px;
  border-radius: var(--hui-radius-md, 8px);
  background: var(--hui-fill-tertiary, #f5f8ff);
}
</style>
