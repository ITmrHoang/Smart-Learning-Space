<template>
  <hui-modal
    :open="open"
    :title="title"
    size="sm"
    :okText="okText"
    cancel-text="Hủy"
    :okType="danger ? 'danger' : 'primary'"
    :confirmLoading="loading"
    @hui-ok="onOk"
    @hui-close="close"
  >
    <p class="message">{{ message }}</p>
  </hui-modal>
</template>

<script setup>
/** Hộp xác nhận cho thao tác quan trọng (khóa tài khoản, duyệt hồ sơ…). */
defineProps({
  open: { type: Boolean, default: false },
  title: { type: String, default: 'Xác nhận' },
  message: { type: String, default: '' },
  okText: { type: String, default: 'Đồng ý' },
  danger: { type: Boolean, default: false },
  loading: { type: Boolean, default: false },
});
const emit = defineEmits(['update:open', 'confirm']);

function onOk(e) {
  // Giữ hộp mở; trang cha tự đóng khi thao tác xong.
  e.preventDefault();
  emit('confirm');
}

function close() {
  emit('update:open', false);
}
</script>

<style scoped>
.message {
  margin: 0;
}
</style>
