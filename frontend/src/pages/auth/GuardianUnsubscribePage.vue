<template>
  <div class="stack">
    <h1>Nhận báo cáo học tập</h1>
    <hui-alert v-if="error" severity="error" :description="error" />
    <template v-if="info">
      <p>
        Báo cáo học tập của <b>{{ info.studentName }}</b> gửi tới <b>{{ info.email }}</b>.
      </p>
      <hui-alert v-if="info.receiveReports" severity="info" description="Bạn đang nhận báo cáo tuần vào tối Chủ nhật." />
      <hui-alert v-else severity="success" description="Bạn đã hủy nhận báo cáo. Sẽ không còn email báo cáo gửi tới địa chỉ này." />
      <div class="row">
        <hui-button v-if="info.receiveReports" variant="danger" :loading="busy" @hui-click="set(false)">Hủy nhận báo cáo</hui-button>
        <hui-button v-else-if="info.consentStatus === 'confirmed'" variant="outline" :loading="busy" @hui-click="set(true)">Nhận lại báo cáo</hui-button>
      </div>
    </template>
  </div>
</template>

<script setup>
import { onMounted, ref } from 'vue';
import { useRoute } from 'vue-router';

import { guardianApi } from '../../api';

const route = useRoute();
const info = ref(null);
const error = ref('');
const busy = ref(false);
const token = () => route.query.token ?? '';

async function set(subscribe) {
  busy.value = true;
  error.value = '';
  try {
    info.value = subscribe ? await guardianApi.subscribe(token()) : await guardianApi.unsubscribe(token());
  } catch (e) {
    error.value = e.message;
  } finally {
    busy.value = false;
  }
}

onMounted(async () => {
  try {
    info.value = await guardianApi.subscription(token());
  } catch (e) {
    error.value = e.message;
  }
});
</script>
