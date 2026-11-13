<template>
  <div class="tile" :class="{ speaking, contain: fit === 'contain' }">
    <video v-show="hasVideo" ref="video" autoplay playsinline muted />
    <div v-if="!hasVideo" class="avatar" aria-hidden="true">{{ initials }}</div>
    <span class="label">
      <span v-if="hand" title="Đang giơ tay">✋</span>
      {{ label }}
      <span v-if="micMuted" class="mic-off" title="Đang tắt mic">🔇</span>
    </span>
  </div>
</template>

<script setup>
/** Ô video của một người (camera hoặc chia sẻ màn hình). `version` đổi → gắn lại track. */
import { Track } from 'livekit-client';
import { computed, onBeforeUnmount, ref, watch } from 'vue';

const props = defineProps({
  participant: { type: Object, required: true },
  /** camera | screen_share */
  source: { type: String, default: 'camera' },
  version: { type: Number, default: 0 },
  label: { type: String, default: '' },
  hand: { type: Boolean, default: false },
  /** cover (lấp đầy ô) | contain (giữ nguyên khung — màn hình) */
  fit: { type: String, default: 'cover' },
});

const video = ref(null);
let attached = null;

const pub = computed(() => {
  void props.version;
  const src = props.source === 'screen_share' ? Track.Source.ScreenShare : Track.Source.Camera;
  return props.participant.getTrackPublication(src);
});
const track = computed(() => {
  void props.version;
  const p = pub.value;
  return p && !p.isMuted ? p.videoTrack ?? null : null;
});
const hasVideo = computed(() => !!track.value);
const speaking = computed(() => {
  void props.version;
  return props.participant.isSpeaking;
});
const micMuted = computed(() => {
  void props.version;
  const mic = props.participant.getTrackPublication(Track.Source.Microphone);
  return !mic || mic.isMuted;
});
const initials = computed(() =>
  (props.label || props.participant.name || '?')
    .replace(/\(Khách\)/, '')
    .trim()
    .split(/\s+/)
    .slice(-2)
    .map((w) => w[0]?.toUpperCase() ?? '')
    .join(''),
);

watch(
  [track, video],
  ([t, el]) => {
    if (attached && attached !== t) attached.detach(el ?? undefined);
    attached = null;
    if (t && el) {
      t.attach(el);
      attached = t;
    }
  },
  { immediate: true },
);

onBeforeUnmount(() => {
  attached?.detach(video.value ?? undefined);
});
</script>

<style scoped>
.tile {
  position: relative;
  overflow: hidden;
  border-radius: 10px;
  background: #1f2937;
  aspect-ratio: 16 / 9;
  outline: 2px solid transparent;
  transition: outline-color 0.15s;
}
.tile.speaking {
  outline-color: #22c55e;
}
video {
  width: 100%;
  height: 100%;
  object-fit: cover;
  display: block;
}
.contain video {
  object-fit: contain;
  background: #000;
}
.avatar {
  position: absolute;
  inset: 0;
  display: grid;
  place-items: center;
  font-size: 1.6rem;
  font-weight: 700;
  color: #e5e7eb;
}
.label {
  position: absolute;
  left: 6px;
  bottom: 6px;
  max-width: calc(100% - 12px);
  padding: 2px 8px;
  border-radius: 999px;
  background: rgba(0, 0, 0, 0.55);
  color: #fff;
  font-size: 0.78rem;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}
.mic-off {
  margin-left: 2px;
}
</style>
