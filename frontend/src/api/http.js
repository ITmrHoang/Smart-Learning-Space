/**
 * API client dùng chung.
 *
 * - Access token chỉ giữ trong bộ nhớ (không lưu localStorage) và gắn vào header Authorization.
 * - Refresh token nằm trong cookie HttpOnly do backend đặt; JS không đọc được.
 * - Access token hết hạn (401 TOKEN_EXPIRED / INVALID_TOKEN) → gọi /auth/refresh MỘT lần rồi gửi lại request.
 *   Nhiều request lỗi cùng lúc dùng chung một lần refresh.
 */

const BASE = '/api';
const RETRY_CODES = new Set(['TOKEN_EXPIRED', 'INVALID_TOKEN']);

let accessToken = null;
let refreshing = null;
const listeners = {
  /** Có phiên mới (sau refresh). */
  session: () => {},
  /** Refresh thất bại → phiên đã hết, cần đăng nhập lại. */
  expired: () => {},
};

export class ApiError extends Error {
  constructor(status, code, message, fields = null) {
    super(message);
    this.name = 'ApiError';
    this.status = status;
    this.code = code;
    /** Lỗi theo từng trường: { email: ['…'] } */
    this.fields = fields;
  }
}

export function onAuthEvents({ session, expired } = {}) {
  if (session) listeners.session = session;
  if (expired) listeners.expired = expired;
}

export function setAccessToken(token) {
  accessToken = token || null;
}

export function getAccessToken() {
  return accessToken;
}

export function buildUrl(path, query) {
  const qs = new URLSearchParams();
  for (const [key, value] of Object.entries(query || {})) {
    if (value !== undefined && value !== null && value !== '') qs.set(key, String(value));
  }
  const s = qs.toString();
  return BASE + path + (s ? `?${s}` : '');
}

function parseJson(text) {
  try {
    return JSON.parse(text);
  } catch {
    return null;
  }
}

async function send(method, path, { body, query, token, signal } = {}) {
  const headers = { Accept: 'application/json' };
  if (body !== undefined) headers['Content-Type'] = 'application/json';
  if (token) headers.Authorization = `Bearer ${token}`;

  let res;
  try {
    res = await fetch(buildUrl(path, query), {
      method,
      headers,
      body: body === undefined ? undefined : JSON.stringify(body),
      credentials: 'same-origin',
      signal,
    });
  } catch (e) {
    if (e?.name === 'AbortError') throw e;
    throw new ApiError(0, 'NETWORK_ERROR', 'Không kết nối được máy chủ. Kiểm tra mạng rồi thử lại.');
  }

  const text = await res.text();
  const data = text ? parseJson(text) : null;
  if (!res.ok) {
    const err = data?.error;
    throw new ApiError(
      res.status,
      err?.code || `HTTP_${res.status}`,
      err?.message || 'Có lỗi xảy ra, vui lòng thử lại.',
      err?.fields || null,
    );
  }
  return data;
}

/** Đổi refresh token (cookie) lấy access token mới. Gọi song song nhiều lần chỉ gửi một request. */
export function refreshSession() {
  if (!refreshing) {
    refreshing = send('POST', '/auth/refresh')
      .then((session) => {
        accessToken = session.accessToken;
        listeners.session(session);
        return session;
      })
      .finally(() => {
        refreshing = null;
      });
  }
  return refreshing;
}

export async function request(method, path, opts = {}) {
  try {
    return await send(method, path, { ...opts, token: accessToken });
  } catch (e) {
    const canRetry = e instanceof ApiError && e.status === 401 && accessToken && RETRY_CODES.has(e.code) && !opts.noRetry;
    if (!canRetry) throw e;
    try {
      await refreshSession();
    } catch (refreshError) {
      accessToken = null;
      listeners.expired(refreshError);
      throw e;
    }
    return send(method, path, { ...opts, token: accessToken });
  }
}

export const http = {
  get: (path, query, opts) => request('GET', path, { query, ...opts }),
  post: (path, body, opts) => request('POST', path, { body: body ?? {}, ...opts }),
  patch: (path, body, opts) => request('PATCH', path, { body, ...opts }),
  del: (path, opts) => request('DELETE', path, opts),
};
