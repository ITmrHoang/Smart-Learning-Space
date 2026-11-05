/**
 * Quyết định điều hướng theo trạng thái đăng nhập và quyền.
 * meta trên route: `guest` (chỉ khi chưa đăng nhập), `auth` (phải đăng nhập), `permission` (phải có quyền).
 *
 * @param {{ fullPath: string, matched: Array<{ meta: object }> }} to
 * @param {{ isLoggedIn: boolean, can: (permission: string) => boolean }} auth
 */
export function resolveGuard(to, auth) {
  const needsAuth = to.matched.some((r) => r.meta.auth);
  const guestOnly = to.matched.some((r) => r.meta.guest);
  const permission = [...to.matched].reverse().find((r) => r.meta.permission)?.meta.permission;

  if (needsAuth && !auth.isLoggedIn) return { name: 'login', query: { redirect: to.fullPath } };
  if (guestOnly && auth.isLoggedIn) return { name: 'home' };
  if (permission && !auth.can(permission)) return { name: 'forbidden' };
  return true;
}
