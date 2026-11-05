<template>
  <div class="auth-layout">
    <main class="auth-card">
      <RouterLink to="/" class="brand">Smart Learning Space</RouterLink>
      <RouterView />
    </main>
    <p class="muted small footer">Nền tảng học và dạy trực tuyến cho học sinh lớp 1–12</p>
  </div>
</template>

<style scoped>
.auth-layout {
  min-height: 100vh;
  display: grid;
  place-items: center;
  align-content: center;
  gap: 16px;
  padding: 24px 16px;
}
.auth-card {
  width: min(440px, 100%);
  background: var(--hui-bg-container, #fff);
  border: 1px solid var(--hui-border-secondary, #eceef1);
  border-radius: var(--hui-radius-xl, 16px);
  box-shadow: var(--hui-shadow-md, 0 6px 24px rgb(0 0 0 / 6%));
  padding: 32px 28px;
}
.brand {
  display: block;
  margin-bottom: 20px;
  font-weight: 700;
  font-size: 1.1rem;
}
.footer {
  margin: 0;
  text-align: center;
}
:deep(.auth-form) {
  display: grid;
  gap: 14px;
}
:deep(.auth-form h1) {
  font-size: 1.4rem;
  margin: 0;
}
:deep(.auth-links) {
  display: flex;
  justify-content: space-between;
  gap: 12px;
  flex-wrap: wrap;
  font-size: 0.9rem;
}
</style>
