import { ref } from 'vue';

import { catalogApi } from '../api';

/** Master data (môn học, lớp, mức độ) — tải một lần, dùng chung giữa các trang. */
const data = ref(null);
let pending = null;

export function useMasterData() {
  async function load(force = false) {
    if (data.value && !force) return data.value;
    if (!pending || force) {
      pending = catalogApi
        .masterData()
        .then((d) => {
          data.value = d;
          return d;
        })
        .finally(() => {
          pending = null;
        });
    }
    return pending;
  }
  return { data, load };
}
