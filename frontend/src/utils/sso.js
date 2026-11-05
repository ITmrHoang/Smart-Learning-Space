/** Đăng nhập bằng Google / Microsoft (SSO). Luồng chi tiết: backend `modules/sso/flow.rs`. */
import { ssoApi } from '../api';

export const PROVIDER_LABELS = { google: 'Google', microsoft: 'Microsoft' };
export const PROVIDERS = Object.keys(PROVIDER_LABELS);

/** Thông báo cho mã lỗi `ssoError` mà callback gắn lên URL khi chuyển về. */
const SSO_ERRORS = {
  SSO_CANCELLED: 'Bạn đã hủy đăng nhập.',
  SSO_STATE_INVALID: 'Lượt đăng nhập đã hết hạn. Vui lòng bấm đăng nhập lại.',
  SSO_FAILED: 'Không đăng nhập được với nhà cung cấp. Vui lòng thử lại.',
  SSO_UNAVAILABLE: 'Không kết nối được tới nhà cung cấp đăng nhập. Vui lòng thử lại sau.',
  SSO_EMAIL_UNVERIFIED:
    'Tài khoản này chưa có email đã xác minh nên không dùng để đăng nhập được. Hãy đăng ký bằng email và mật khẩu.',
  SSO_ALREADY_LINKED: 'Tài khoản Google / Microsoft này đã liên kết với một tài khoản khác.',
  SSO_PROVIDER_ALREADY_LINKED: 'Tài khoản của bạn đã liên kết với một tài khoản khác của nhà cung cấp này. Hãy gỡ liên kết cũ trước.',
  ACCOUNT_LOCKED: 'Tài khoản đã bị khóa. Vui lòng liên hệ quản trị viên.',
  UNAUTHENTICATED: 'Phiên đăng nhập đã hết. Hãy đăng nhập rồi liên kết lại.',
  RATE_LIMITED: 'Bạn thao tác quá nhiều lần. Vui lòng thử lại sau ít phút.',
};

export function ssoErrorMessage(code) {
  if (!code) return '';
  return SSO_ERRORS[code] || 'Đăng nhập không thành công. Vui lòng thử lại.';
}

/** Thông báo khi phiên bị đăng xuất (lỗi refresh). Chỉ các lý do người dùng cần biết. */
export function sessionEndedMessage(code) {
  if (code === 'SESSION_LIMIT') {
    return 'Tài khoản vừa đăng nhập trên thiết bị khác nên thiết bị này đã bị đăng xuất (mỗi tài khoản dùng tối đa 2 thiết bị cùng lúc).';
  }
  if (code === 'REFRESH_TOKEN_REUSED') return 'Phiên đăng nhập đã bị thu hồi vì lý do an toàn. Vui lòng đăng nhập lại.';
  return '';
}

/** Chỉ nhận đường dẫn nội bộ (`/lop-hoc`), không nhận `//site-khac` hay URL đầy đủ. */
export function safeNext(value) {
  if (typeof value !== 'string') return undefined;
  if (!value.startsWith('/') || value.startsWith('//') || value.startsWith('/\\')) return undefined;
  return value;
}

/**
 * Đọc thông tin yêu cầu liên kết trong phần `#…` của URL (`/sso/lien-ket#token=…&provider=…`).
 * Token để ở `#` để không bị gửi lên server hay lộ qua header Referer.
 */
export function parseLinkFragment(hash) {
  const params = new URLSearchParams((hash || '').replace(/^#/, ''));
  const token = params.get('token');
  if (!token) return null;
  const provider = params.get('provider');
  return {
    token,
    provider: PROVIDERS.includes(provider) ? provider : null,
    emailHint: params.get('email') || '',
    canUsePassword: params.get('password') === '1',
    expiresIn: Number(params.get('expiresIn')) || 0,
    next: safeNext(params.get('next')),
  };
}

/** Chuyển trình duyệt sang trang đăng nhập của nhà cung cấp. */
export async function redirectToProvider(provider, opts = {}) {
  const { authorizeUrl } = await ssoApi.start(provider, opts);
  window.location.assign(authorizeUrl);
}
