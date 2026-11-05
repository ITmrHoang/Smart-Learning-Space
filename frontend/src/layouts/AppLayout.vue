<template>
  <div class="app-layout" :class="{ 'menu-open': menuOpen }">
    <aside class="sidebar" aria-label="Menu chính">
      <RouterLink to="/" class="brand" @click="menuOpen = false">Smart Learning Space</RouterLink>
      <nav>
        <section v-for="group in menu" :key="group.title ?? 'main'" class="menu-group">
          <h2 v-if="group.title" class="menu-title">{{ group.title }}</h2>
          <RouterLink
            v-for="item in group.items"
            :key="item.to"
            :to="item.to"
            class="menu-item"
            :exactActiveClass="item.to === '/' ? 'active' : ''"
            :activeClass="item.to === '/' ? '' : 'active'"
            @click="menuOpen = false"
          >
            {{ item.label }}
          </RouterLink>
        </section>
      </nav>
    </aside>

    <div class="main">
      <header class="topbar">
        <hui-button class="menu-toggle" variant="text" @hui-click="menuOpen = !menuOpen">Menu</hui-button>
        <div class="spacer" />
        <div v-if="auth.user" class="user">
          <span class="user-name">{{ auth.user.fullName }}</span>
          <hui-tag size="sm" :color="ROLE_COLORS[auth.user.role]">{{ ROLE_LABELS[auth.user.role] }}</hui-tag>
        </div>
        <hui-button variant="outline" size="sm" :loading="loggingOut" @hui-click="logout">Đăng xuất</hui-button>
      </header>
      <main class="content">
        <RouterView />
      </main>
    </div>
  </div>
</template>

<script setup>
import { computed, ref, watch } from 'vue';
import { useRouter } from 'vue-router';

import { useAuthStore } from '../stores/auth';
import { ROLE_COLORS, ROLE_LABELS, visibleMenu } from '../utils/permissions';

const auth = useAuthStore();
const router = useRouter();
const menuOpen = ref(false);
const loggingOut = ref(false);
const menu = computed(() => visibleMenu(auth.permissions));

async function logout() {
  loggingOut.value = true;
  try {
    await auth.logout();
  } finally {
    loggingOut.value = false;
  }
}

// Đăng xuất hoặc phiên hết hạn (refresh thất bại) → về trang đăng nhập.
watch(
  () => auth.isLoggedIn,
  (loggedIn) => {
    if (!loggedIn) router.replace({ name: 'login', query: { redirect: router.currentRoute.value.fullPath } });
  },
);
</script>

<style scoped>
.app-layout {
  min-height: 100vh;
  display: grid;
  grid-template-columns: 248px 1fr;
}
.sidebar {
  position: sticky;
  top: 0;
  height: 100vh;
  overflow-y: auto;
  background: var(--hui-bg-container, #fff);
  border-right: 1px solid var(--hui-border-secondary, #eceef1);
  padding: 20px 12px;
}
.brand {
  display: block;
  padding: 0 12px 16px;
  font-weight: 700;
}
.menu-group + .menu-group {
  margin-top: 16px;
}
.menu-title {
  margin: 0 0 6px;
  padding: 0 12px;
  font-size: 0.75rem;
  text-transform: uppercase;
  letter-spacing: 0.04em;
  color: var(--hui-text-secondary, #6b7280);
}
.menu-item {
  display: block;
  padding: 8px 12px;
  border-radius: var(--hui-radius-md, 8px);
  color: inherit;
}
.menu-item:hover {
  background: var(--hui-fill-tertiary, #f3f4f6);
  text-decoration: none;
}
.menu-item.active {
  background: var(--hui-primary-light, #e6f0ff);
  color: var(--hui-primary, #1677ff);
  font-weight: 600;
}
.main {
  min-width: 0;
}
.topbar {
  position: sticky;
  top: 0;
  z-index: 5;
  display: flex;
  align-items: center;
  gap: 12px;
  padding: 10px 24px;
  background: var(--hui-bg-container, #fff);
  border-bottom: 1px solid var(--hui-border-secondary, #eceef1);
}
.spacer {
  flex: 1;
}
.user {
  display: flex;
  align-items: center;
  gap: 8px;
}
.menu-toggle {
  display: none;
}
.content {
  padding: 24px;
}

@media (max-width: 860px) {
  .app-layout {
    grid-template-columns: 1fr;
  }
  .sidebar {
    position: fixed;
    inset: 0 auto 0 0;
    width: 260px;
    z-index: 20;
    transform: translateX(-100%);
    transition: transform 0.2s ease;
    box-shadow: var(--hui-shadow-lg, 0 8px 32px rgb(0 0 0 / 12%));
  }
  .menu-open .sidebar {
    transform: none;
  }
  .menu-toggle {
    display: inline-flex;
  }
  .user-name {
    display: none;
  }
  .content,
  .topbar {
    padding-inline: 16px;
  }
}
</style>
