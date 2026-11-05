import { describe, expect, it, vi } from 'vitest';

import { usePagedList } from './list';

describe('usePagedList', () => {
  it('tải trang, đổi bộ lọc thì về trang 1', async () => {
    const fetcher = vi.fn(async (q) => ({ items: [q], total: 42 }));
    const list = usePagedList(fetcher, { role: '' });

    await list.load();
    expect(fetcher).toHaveBeenLastCalledWith({ role: '', page: 1, pageSize: 20 });
    expect(list.total.value).toBe(42);

    await list.onPage({ current: 3, pageSize: 20 });
    expect(fetcher).toHaveBeenLastCalledWith({ role: '', page: 3, pageSize: 20 });

    await list.setFilter('role', 'teacher');
    expect(fetcher).toHaveBeenLastCalledWith({ role: 'teacher', page: 1, pageSize: 20 });

    await list.onPage({ current: 2, pageSize: 50 });
    expect(fetcher).toHaveBeenLastCalledWith({ role: 'teacher', page: 1, pageSize: 50 });
  });

  it('kết quả của request cũ về sau không ghi đè request mới', async () => {
    let resolveSlow;
    const fetcher = vi
      .fn()
      .mockImplementationOnce(() => new Promise((r) => (resolveSlow = r)))
      .mockImplementationOnce(async () => ({ items: ['mới'], total: 1 }));
    const list = usePagedList(fetcher);
    const slow = list.load();
    await list.setFilter('q', 'x');
    resolveSlow({ items: ['cũ'], total: 9 });
    await slow;
    expect(list.items.value).toEqual(['mới']);
    expect(list.loading.value).toBe(false);
  });

  it('lỗi tải → lưu thông báo lỗi', async () => {
    const list = usePagedList(async () => {
      throw new Error('Không có mạng');
    });
    await list.load();
    expect(list.error.value).toBe('Không có mạng');
  });
});
