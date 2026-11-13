<template>
  <div class="stack">
    <h1>Báo cáo học tập cho phụ huynh</h1>
    <hui-alert v-if="error" severity="error" :description="error" />
    <template v-if="info">
      <p>
        Kính gửi <b>{{ info.guardianName }}</b> ({{ info.email }}),<br />
        học sinh <b>{{ info.studentName }}</b> đã khai bạn là {{ info.relation || 'phụ huynh' }} và muốn bạn nhận
        <b>báo cáo học tập hằng tuần</b> qua email: tiến độ học, điểm các bài kiểm tra, điểm danh buổi học.
      </p>
      <p class="muted small">
        Khi đồng ý, bạn cho phép Smart Learning Space gửi cho bạn thông tin học tập của học sinh. Bạn có thể hủy nhận bất cứ lúc nào
        bằng link ở cuối mỗi email báo cáo.
      </p>
      <hui-alert v-if="info.consentStatus === 'confirmed'" severity="success" description="Bạn đã đồng ý nhận báo cáo. Báo cáo tuần gửi vào tối Chủ nhật." />
      <hui-alert v-else-if="info.consentStatus === 'declined'" severity="info" description="Bạn đã từ chối nhận báo cáo. Có thể đổi ý bằng nút bên dưới." />
      <div class="row">
        <hui-button v-if="info.consentStatus !== 'confirmed'" :loading="busy === 'yes'" @hui-click="respond(true)">Đồng ý nhận báo cáo</hui-button>
        <hui-button v-if="info.consentStatus !== 'declined'" variant="outline" :loading="busy === 'no'" @hui-click="respond(false)">Từ chối</hui-button>
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
const busy = ref('');

async function respond(accept) {
  busy.value = accept ? 'yes' : 'no';
  error.value = '';
  try {
    info.value = await guardianApi.respond(route.query.token, accept);
  } catch (e) {
    error.value = e.message;
  } finally {
    busy.value = '';
  }
}

onMounted(async () => {
  try {
    info.value = await guardianApi.consentInfo(route.query.token ?? '');
  } catch (e) {
    error.value = e.message;
  }
});
</script>
