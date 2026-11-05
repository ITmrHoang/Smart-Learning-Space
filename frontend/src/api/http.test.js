import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

import { ApiError, buildUrl, getAccessToken, http, onAuthEvents, refreshSession, setAccessToken } from './http';

function jsonResponse(status, body) {
  return { ok: status >= 200 && status < 300, status, text: async () => (body === undefined ? '' : JSON.stringify(body)) };
}

const session = (token) => ({ accessToken: token, user: { id: 'u1', permissions: [] } });

describe('api/http', () => {
  let fetchMock;

  beforeEach(() => {
    fetchMock = vi.fn();
    vi.stubGlobal('fetch', fetchMock);
    setAccessToken(null);
    onAuthEvents({ session: () => {}, expired: () => {} });
  });

  afterEach(() => {
    vi.unstubAllGlobals();
  });

  it('tạo URL kèm query, bỏ giá trị rỗng', () => {
    expect(buildUrl('/admin/users', { q: 'an', role: '', page: 2, status: null })).toBe('/api/admin/users?q=an&page=2');
    expect(buildUrl('/health')).toBe('/api/health');
  });

  it('gắn access token vào header và gửi JSON', async () => {
    setAccessToken('t1');
    fetchMock.mockResolvedValue(jsonResponse(200, { ok: true }));
    await http.post('/auth/login', { login: 'a', password: 'b' });
    const [url, init] = fetchMock.mock.calls[0];
    expect(url).toBe('/api/auth/login');
    expect(init.headers.Authorization).toBe('Bearer t1');
    expect(init.headers['Content-Type']).toBe('application/json');
    expect(JSON.parse(init.body)).toEqual({ login: 'a', password: 'b' });
    expect(init.credentials).toBe('same-origin');
  });

  it('chuyển lỗi API thành ApiError có code và lỗi từng trường', async () => {
    fetchMock.mockResolvedValue(
      jsonResponse(422, { error: { code: 'VALIDATION_FAILED', message: 'Dữ liệu chưa hợp lệ.', fields: { email: ['Email không hợp lệ.'] } } }),
    );
    const err = await http.post('/auth/register', {}).catch((e) => e);
    expect(err).toBeInstanceOf(ApiError);
    expect(err.status).toBe(422);
    expect(err.code).toBe('VALIDATION_FAILED');
    expect(err.fields.email[0]).toBe('Email không hợp lệ.');
  });

  it('mất mạng → NETWORK_ERROR', async () => {
    fetchMock.mockRejectedValue(new TypeError('Failed to fetch'));
    const err = await http.get('/health').catch((e) => e);
    expect(err.code).toBe('NETWORK_ERROR');
  });

  it('token hết hạn → refresh một lần rồi gửi lại request', async () => {
    setAccessToken('old');
    const onSession = vi.fn();
    onAuthEvents({ session: onSession });
    fetchMock
      .mockResolvedValueOnce(jsonResponse(401, { error: { code: 'TOKEN_EXPIRED', message: 'hết hạn' } }))
      .mockResolvedValueOnce(jsonResponse(200, session('new')))
      .mockResolvedValueOnce(jsonResponse(200, { id: 'u1' }));

    const me = await http.get('/auth/me');
    expect(me).toEqual({ id: 'u1' });
    expect(fetchMock.mock.calls.map((c) => c[0])).toEqual(['/api/auth/me', '/api/auth/refresh', '/api/auth/me']);
    expect(fetchMock.mock.calls[2][1].headers.Authorization).toBe('Bearer new');
    expect(getAccessToken()).toBe('new');
    expect(onSession).toHaveBeenCalledOnce();
  });

  it('nhiều request cùng hết hạn chỉ gọi refresh một lần', async () => {
    setAccessToken('old');
    fetchMock.mockImplementation(async (url, init) => {
      if (url === '/api/auth/refresh') return jsonResponse(200, session('new'));
      return init.headers.Authorization === 'Bearer new'
        ? jsonResponse(200, { url })
        : jsonResponse(401, { error: { code: 'TOKEN_EXPIRED', message: 'hết hạn' } });
    });
    const results = await Promise.all([http.get('/a'), http.get('/b'), http.get('/c')]);
    expect(results.map((r) => r.url)).toEqual(['/api/a', '/api/b', '/api/c']);
    expect(fetchMock.mock.calls.filter((c) => c[0] === '/api/auth/refresh')).toHaveLength(1);
  });

  it('refresh thất bại → báo phiên hết hạn và xóa token', async () => {
    setAccessToken('old');
    const onExpired = vi.fn();
    onAuthEvents({ expired: onExpired });
    fetchMock
      .mockResolvedValueOnce(jsonResponse(401, { error: { code: 'TOKEN_EXPIRED', message: 'hết hạn' } }))
      .mockResolvedValueOnce(jsonResponse(401, { error: { code: 'INVALID_REFRESH_TOKEN', message: 'phiên hỏng' } }));
    const err = await http.get('/auth/me').catch((e) => e);
    expect(err.code).toBe('TOKEN_EXPIRED');
    expect(onExpired).toHaveBeenCalledOnce();
    expect(getAccessToken()).toBeNull();
  });

  it('lỗi 401 khác (sai mật khẩu) không gọi refresh', async () => {
    fetchMock.mockResolvedValue(jsonResponse(401, { error: { code: 'INVALID_CREDENTIALS', message: 'sai' } }));
    await http.post('/auth/login', {}).catch(() => {});
    expect(fetchMock).toHaveBeenCalledOnce();
  });

  it('refreshSession trả về phiên mới', async () => {
    fetchMock.mockResolvedValue(jsonResponse(200, session('abc')));
    const s = await refreshSession();
    expect(s.accessToken).toBe('abc');
    expect(getAccessToken()).toBe('abc');
  });
});
