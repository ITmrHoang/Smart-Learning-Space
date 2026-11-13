<template>
  <div class="thumbs">
    <figure v-for="(f, i) in files" :key="f.id ?? i" class="thumb" :class="{ marked: !!f.annotated }">
      <a :href="(f.annotated ?? f.file ?? f).url" target="_blank" rel="noopener" :title="name(f)">
        <img v-if="(f.annotated ?? f.file ?? f).isImage" :src="(f.annotated ?? f.file ?? f).url" :alt="name(f)" loading="lazy" />
        <span v-else class="pdf">PDF</span>
      </a>
      <figcaption class="small">
        <span class="name">{{ name(f) }}</span>
        <span v-if="f.annotated" class="tag-marked">Có nhận xét</span>
        <a v-if="f.annotated" :href="f.file.url" target="_blank" rel="noopener" class="muted">Ảnh gốc</a>
        <button v-if="removable" type="button" class="remove" title="Bỏ file này" @click="$emit('remove', i)">✕</button>
      </figcaption>
      <hui-progress v-if="f.progress != null && f.progress < 100" :value="f.progress" size="sm" />
    </figure>
  </div>
</template>

<script setup>
/**
 * Danh sách ảnh / PDF dạng ô nhỏ. Mỗi phần tử là FileView (`{ name, url, isImage }`) hoặc file của bài nộp
 * (`{ file, annotated }` — có ảnh giáo viên vẽ nhận xét thì hiện ảnh đó, kèm link ảnh gốc).
 */
defineProps({
  files: { type: Array, default: () => [] },
  removable: { type: Boolean, default: false },
});
defineEmits(['remove']);

const name = (f) => (f.file ?? f).name;
</script>

<style scoped>
.thumbs {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(140px, 1fr));
  gap: 10px;
}
.thumb {
  margin: 0;
  display: grid;
  gap: 4px;
  padding: 6px;
  border: 1px solid var(--hui-border-secondary, #eceef1);
  border-radius: var(--hui-radius-md, 8px);
  background: var(--hui-bg-container, #fff);
}
.thumb.marked {
  border-color: var(--hui-danger, #dc2626);
}
.thumb img,
.pdf {
  display: block;
  width: 100%;
  aspect-ratio: 4 / 3;
  object-fit: cover;
  border-radius: 6px;
  background: var(--hui-bg-secondary, #f5f6f8);
}
.pdf {
  display: grid;
  place-items: center;
  font-weight: 700;
  color: var(--hui-danger, #dc2626);
  text-decoration: none;
}
figcaption {
  display: flex;
  flex-wrap: wrap;
  gap: 2px 6px;
  align-items: center;
}
.name {
  flex: 1 1 100%;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.tag-marked {
  color: var(--hui-danger, #dc2626);
  font-weight: 600;
}
.remove {
  margin-left: auto;
  border: 0;
  background: none;
  cursor: pointer;
  color: var(--hui-text-secondary, #6b7280);
}
</style>
