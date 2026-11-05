<template>
  <div class="page">
    <div class="page-header">
      <h1>Tìm lớp học</h1>
    </div>

    <section class="panel row">
      <form class="row grow-form" @submit.prevent="join">
        <hui-input v-model="code" label="Có mã mời của giáo viên?" placeholder="Nhập mã mời, ví dụ ABCD2345" :error="codeError" />
        <hui-button type="submit" :loading="joining">Vào lớp</hui-button>
      </form>
    </section>

    <section class="panel toolbar">
      <hui-input class="grow" type="search" label="Tìm lớp học" placeholder="Tên lớp học" clearable @hui-search="list.setFilter('q', $event.detail.value)" @clear="list.setFilter('q', '')" />
      <hui-select label="Môn" allow-clear placeholder="Tất cả" :options="subjectOptions" @change="list.setFilter('subjectId', $event.detail.value)" />
      <hui-select label="Lớp" allow-clear placeholder="Tất cả" :options="gradeOptions" @change="list.setFilter('gradeId', $event.detail.value)" />
    </section>

    <hui-alert v-if="list.error.value" severity="error" :description="list.error.value" />
    <p v-if="!list.loading.value && !list.items.value.length" class="panel empty muted">Không có lớp học công khai nào phù hợp.</p>
    <div class="card-list">
      <ClassCard v-for="c in list.items.value" :key="c.id" :cls="c">
        <RouterLink :to="{ name: 'class', params: { id: c.id } }">{{ c.myStatus === 'active' ? 'Vào học →' : 'Xem lớp học →' }}</RouterLink>
      </ClassCard>
    </div>
    <hui-pagination class="pager" :total="list.total.value" :current="list.page.value" :pageSize="list.pageSize.value" hide-on-single-page @change="list.onPage($event.detail)" />
  </div>
</template>

<script setup>
import { computed, onMounted, ref } from 'vue';
import { useRouter } from 'vue-router';

import { learnApi } from '../../api';
import ClassCard from '../../components/learning/ClassCard.vue';
import { usePagedList } from '../../utils/list';
import { useMasterData } from '../../utils/masterData';

const router = useRouter();
const { data: master, load: loadMaster } = useMasterData();
const list = usePagedList((q) => learnApi.catalog(q), { q: '', subjectId: '', gradeId: '' }, { pageSize: 24 });
const code = ref('');
const codeError = ref('');
const joining = ref(false);
const subjectOptions = computed(() => (master.value?.subjects ?? []).map((s) => ({ value: s.id, label: s.name })));
const gradeOptions = computed(() => (master.value?.grades ?? []).filter((g) => g.isShared).map((g) => ({ value: g.id, label: g.name })));

async function join() {
  codeError.value = '';
  joining.value = true;
  try {
    const res = await learnApi.join(code.value.trim());
    router.push({ name: 'class', params: { id: res.classId } });
  } catch (e) {
    codeError.value = e.fields?.code?.[0] ?? e.message;
  } finally {
    joining.value = false;
  }
}

onMounted(() => {
  loadMaster();
  list.load();
});
</script>

<style scoped>
.grow-form {
  align-items: flex-end;
  width: 100%;
}
.grow-form hui-input {
  flex: 1 1 260px;
}
.pager {
  display: flex;
  justify-content: flex-end;
}
</style>
