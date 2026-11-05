import { reactive, ref } from 'vue';

import { ApiError } from '../api/http';

/**
 * Trạng thái gửi form: đang gửi, lỗi chung, lỗi từng trường (từ `error.fields` của API).
 *
 *   const { loading, error, fields, run } = useSubmit();
 *   const res = await run(() => api.save(form));
 *   if (res.ok) …
 */
export function useSubmit() {
  const loading = ref(false);
  const error = ref('');
  const fields = reactive({});

  function reset() {
    error.value = '';
    for (const key of Object.keys(fields)) delete fields[key];
  }

  async function run(fn) {
    loading.value = true;
    reset();
    try {
      return { ok: true, data: await fn() };
    } catch (e) {
      if (e instanceof ApiError) {
        const fieldErrors = e.fields ? Object.entries(e.fields) : [];
        for (const [key, messages] of fieldErrors) fields[key] = messages[0];
        if (fieldErrors.length === 0) error.value = e.message;
      } else {
        console.error(e);
        error.value = 'Có lỗi xảy ra, vui lòng thử lại.';
      }
      return { ok: false, error: e };
    } finally {
      loading.value = false;
    }
  }

  return { loading, error, fields, run, reset };
}

/** Định dạng ngày giờ theo tiếng Việt. */
export function formatDateTime(value) {
  if (!value) return '—';
  return new Intl.DateTimeFormat('vi-VN', { dateStyle: 'short', timeStyle: 'short' }).format(new Date(value));
}
