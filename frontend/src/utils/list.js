import { reactive, ref } from 'vue';

/**
 * Danh sách phân trang phía server.
 *
 *   const list = usePagedList((query) => adminApi.users(query), { role: '' });
 *   list.load();  list.setFilter('role', 'teacher');  list.onPage({ current: 2, pageSize: 20 });
 *
 * Request cũ trả về sau request mới thì bị bỏ qua (không ghi đè kết quả mới).
 */
export function usePagedList(fetcher, initialFilters = {}, { pageSize = 20 } = {}) {
  const filters = reactive({ ...initialFilters });
  const page = ref(1);
  const size = ref(pageSize);
  const items = ref([]);
  const total = ref(0);
  const loading = ref(false);
  const error = ref('');
  let seq = 0;

  async function load() {
    const current = ++seq;
    loading.value = true;
    error.value = '';
    try {
      const res = await fetcher({ ...filters, page: page.value, pageSize: size.value });
      if (current !== seq) return;
      items.value = res.items;
      total.value = res.total;
    } catch (e) {
      if (current === seq) error.value = e.message || 'Không tải được dữ liệu.';
    } finally {
      if (current === seq) loading.value = false;
    }
  }

  function setFilter(key, value) {
    filters[key] = value ?? '';
    page.value = 1;
    return load();
  }

  function onPage({ current, pageSize: newSize }) {
    if (newSize && newSize !== size.value) {
      size.value = newSize;
      page.value = 1;
    } else {
      page.value = current;
    }
    return load();
  }

  return { filters, page, pageSize: size, items, total, loading, error, load, setFilter, onPage };
}
