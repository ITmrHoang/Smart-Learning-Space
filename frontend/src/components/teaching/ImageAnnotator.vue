<template>
  <hui-modal :open="open" title="Vẽ nhận xét lên ảnh" size="full" ok-text="Lưu ảnh đã vẽ" cancel-text="Hủy" :confirmLoading="saving" @hui-ok="onOk" @hui-close="close">
    <div class="annotator">
      <div class="annot-tools">
        <span class="small muted">Màu</span>
        <button
          v-for="c in COLORS"
          :key="c.value"
          type="button"
          class="swatch"
          :class="{ active: color === c.value }"
          :style="{ background: c.value }"
          :title="c.label"
          :aria-label="c.label"
          @click="color = c.value"
        />
        <span class="small muted">Nét</span>
        <select v-model.number="width" class="native-select small">
          <option :value="3">Mảnh</option>
          <option :value="6">Vừa</option>
          <option :value="12">Đậm</option>
        </select>
        <hui-button size="sm" variant="text" :disabled="!strokes.length" @hui-click="undo">Hoàn tác</hui-button>
        <hui-button size="sm" variant="text" :disabled="!strokes.length" @hui-click="clearAll">Xóa hết nét</hui-button>
        <span class="muted small">Kéo chuột / ngón tay / bút trên ảnh để viết.</span>
      </div>
      <p v-if="loadError" class="field-error">{{ loadError }}</p>
      <div class="stage">
        <canvas
          ref="canvas"
          @pointerdown="start"
          @pointermove="moveTo"
          @pointerup="end"
          @pointercancel="end"
          @pointerleave="end"
        />
      </div>
    </div>
  </hui-modal>
</template>

<script setup>
/**
 * Giáo viên vẽ nét nhận xét lên ảnh bài làm (bút, nhiều màu, hoàn tác). Lưu = ghép ảnh gốc + nét vẽ thành
 * một ảnh mới, upload (file `homework`) rồi phát `saved` với FileView của ảnh mới.
 */
import { message } from '@itmrhoang/h-ui/hui-message';
import { nextTick, ref, watch } from 'vue';

import { uploadFile } from '../../utils/upload';

const COLORS = [
  { value: '#dc2626', label: 'Đỏ' },
  { value: '#2563eb', label: 'Xanh dương' },
  { value: '#16a34a', label: 'Xanh lá' },
  { value: '#111827', label: 'Đen' },
];
/** Cạnh dài tối đa của ảnh xuất ra (px). */
const MAX_SIDE = 2000;

const props = defineProps({
  open: { type: Boolean, default: false },
  src: { type: String, default: '' },
  name: { type: String, default: 'bai-lam' },
});
const emit = defineEmits(['update:open', 'saved']);

const canvas = ref(null);
const color = ref(COLORS[0].value);
const width = ref(6);
const strokes = ref([]);
const saving = ref(false);
const loadError = ref('');
let image = null;
let current = null;

function scaleOf() {
  const el = canvas.value;
  const rect = el.getBoundingClientRect();
  return el.width / rect.width;
}

function redraw() {
  const el = canvas.value;
  if (!el || !image) return;
  const ctx = el.getContext('2d');
  ctx.drawImage(image, 0, 0, el.width, el.height);
  ctx.lineCap = 'round';
  ctx.lineJoin = 'round';
  for (const s of strokes.value) {
    ctx.strokeStyle = s.color;
    ctx.lineWidth = s.width;
    ctx.beginPath();
    s.points.forEach(([x, y], i) => (i ? ctx.lineTo(x, y) : ctx.moveTo(x, y)));
    if (s.points.length === 1) ctx.lineTo(s.points[0][0] + 0.1, s.points[0][1]);
    ctx.stroke();
  }
}

async function load() {
  loadError.value = '';
  strokes.value = [];
  image = null;
  const img = new Image();
  img.decoding = 'async';
  img.onload = async () => {
    image = img;
    const scale = Math.min(1, MAX_SIDE / Math.max(img.naturalWidth, img.naturalHeight));
    await nextTick();
    canvas.value.width = Math.round(img.naturalWidth * scale);
    canvas.value.height = Math.round(img.naturalHeight * scale);
    redraw();
  };
  img.onerror = () => (loadError.value = 'Không tải được ảnh. Hãy đóng và mở lại.');
  img.src = props.src;
}

function point(e) {
  const rect = canvas.value.getBoundingClientRect();
  const k = scaleOf();
  return [(e.clientX - rect.left) * k, (e.clientY - rect.top) * k];
}

function start(e) {
  if (!image) return;
  canvas.value.setPointerCapture?.(e.pointerId);
  // Nét vẽ theo tỉ lệ ảnh để trông như nhau trên ảnh lớn / nhỏ.
  current = { color: color.value, width: width.value * Math.max(1, canvas.value.width / 1000), points: [point(e)] };
  strokes.value.push(current);
  redraw();
}

function moveTo(e) {
  if (!current) return;
  current.points.push(point(e));
  redraw();
}

function end() {
  current = null;
}

function undo() {
  strokes.value.pop();
  redraw();
}

function clearAll() {
  strokes.value = [];
  redraw();
}

function close() {
  emit('update:open', false);
}

async function onOk(e) {
  e.preventDefault();
  if (!strokes.value.length) {
    message.warning('Chưa có nét vẽ nào.');
    return;
  }
  saving.value = true;
  try {
    const blob = await new Promise((resolve) => canvas.value.toBlob(resolve, 'image/jpeg', 0.9));
    const file = new File([blob], `${props.name.replace(/\.[a-z0-9]+$/i, '')}-nhan-xet.jpg`, { type: 'image/jpeg' });
    const saved = await uploadFile(file, 'homework');
    emit('saved', saved);
    close();
  } catch (err) {
    message.error(err.message);
  } finally {
    saving.value = false;
  }
}

watch(
  () => props.open,
  (open) => open && load(),
);
</script>

<style scoped>
.annotator {
  display: grid;
  gap: 8px;
}
.annot-tools {
  display: flex;
  align-items: center;
  gap: 6px;
  flex-wrap: wrap;
}
.swatch {
  flex: 0 0 22px;
  padding: 0;
  width: 22px;
  height: 22px;
  border-radius: 50%;
  border: 2px solid transparent;
  cursor: pointer;
}
.swatch.active {
  border-color: var(--hui-text, #111827);
  box-shadow: 0 0 0 2px var(--hui-bg-container, #fff) inset;
}
.stage {
  max-height: 70vh;
  overflow: auto;
  border-radius: var(--hui-radius-md, 8px);
  background: var(--hui-bg-secondary, #f5f6f8);
}
canvas {
  display: block;
  width: 100%;
  height: auto;
  cursor: crosshair;
  touch-action: none;
}
</style>
