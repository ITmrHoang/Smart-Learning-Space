<template>
  <div class="center">
    <hui-result status="404" title="Không tìm thấy trang" sub-title="Đường dẫn có thể đã thay đổi hoặc không tồn tại.">
      <RouterLink slot="extra" to="/">Về trang chủ</RouterLink>
    </hui-result>
  </div>
</template>

<style scoped>
.center {
  min-height: 100vh;
  display: grid;
  place-items: center;
  padding: 16px;
}
</style>
