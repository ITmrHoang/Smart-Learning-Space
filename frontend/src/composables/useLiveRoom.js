/**
 * Kết nối phòng LiveKit cho Vue: giữ `Room` (không bọc reactive), đếm `version` tăng mỗi khi phòng đổi
 * (người vào / ra, bật / tắt camera, quyền thay đổi…) để computed / watch chạy lại; tự gắn âm thanh của người
 * khác; chuyển tin trên kênh dữ liệu cho các handler.
 */
import { ConnectionState, Room, RoomEvent, Track } from 'livekit-client';
import { markRaw, onBeforeUnmount, ref, shallowRef } from 'vue';

import { decodeMessage, encodeMessage } from '../utils/live';

/** TrackSource của LiveKit (số) theo nguồn. */
const SOURCE_NUM = { camera: 1, microphone: 2, screen_share: 3 };

/** Người tham gia có được phát nguồn này không (quyền do server cấp). */
export function canPublishSource(participant, source) {
  const p = participant?.permissions;
  if (!p?.canPublish) return false;
  const list = p.canPublishSources ?? [];
  return list.length === 0 || list.includes(SOURCE_NUM[source]);
}

export function useLiveRoom() {
  const room = shallowRef(null);
  const version = ref(0);
  /** idle | connecting | connected | reconnecting | disconnected */
  const state = ref('idle');
  const canPlayAudio = ref(true);
  const disconnectReason = ref(null);
  const handlers = new Set();
  const textHandlers = new Map();
  let audioBox = null;

  const bump = () => {
    version.value += 1;
  };

  function onMessage(handler) {
    handlers.add(handler);
    return () => handlers.delete(handler);
  }

  /** Nhận text stream theo topic (gói lớn như ảnh chụp bảng). */
  function onText(topic, handler) {
    textHandlers.set(topic, handler);
    room.value?.registerTextStreamHandler(topic, async (reader, info) => handler(await reader.readAll(), info));
  }

  async function connect(url, token) {
    const r = markRaw(new Room({ adaptiveStream: true, dynacast: true }));
    room.value = r;
    state.value = 'connecting';
    audioBox = document.createElement('div');
    audioBox.hidden = true;
    document.body.append(audioBox);

    for (const ev of [
      RoomEvent.ParticipantConnected,
      RoomEvent.ParticipantDisconnected,
      RoomEvent.TrackSubscribed,
      RoomEvent.TrackUnsubscribed,
      RoomEvent.TrackMuted,
      RoomEvent.TrackUnmuted,
      RoomEvent.LocalTrackPublished,
      RoomEvent.LocalTrackUnpublished,
      RoomEvent.ActiveSpeakersChanged,
      RoomEvent.ParticipantPermissionsChanged,
      RoomEvent.ParticipantMetadataChanged,
    ]) {
      r.on(ev, bump);
    }
    r.on(RoomEvent.TrackSubscribed, (track) => {
      if (track.kind === Track.Kind.Audio) audioBox.append(track.attach());
    });
    r.on(RoomEvent.TrackUnsubscribed, (track) => {
      if (track.kind === Track.Kind.Audio) track.detach().forEach((el) => el.remove());
    });
    r.on(RoomEvent.ConnectionStateChanged, (s) => {
      state.value =
        s === ConnectionState.Connected
          ? 'connected'
          : s === ConnectionState.Reconnecting || s === ConnectionState.SignalReconnecting
            ? 'reconnecting'
            : s === ConnectionState.Disconnected
              ? 'disconnected'
              : 'connecting';
      bump();
    });
    r.on(RoomEvent.Disconnected, (reason) => {
      disconnectReason.value = reason ?? null;
      state.value = 'disconnected';
    });
    r.on(RoomEvent.AudioPlaybackStatusChanged, () => {
      canPlayAudio.value = r.canPlaybackAudio;
    });
    r.on(RoomEvent.DataReceived, (payload, participant) => {
      const msg = decodeMessage(payload);
      if (msg && participant) handlers.forEach((h) => h(msg, participant));
    });
    for (const [topic, handler] of textHandlers) {
      r.registerTextStreamHandler(topic, async (reader, info) => handler(await reader.readAll(), info));
    }

    await r.connect(url, token);
    state.value = 'connected';
    canPlayAudio.value = r.canPlaybackAudio;
    bump();
    return r;
  }

  /** Gửi tin (reliable) cho cả phòng hoặc một số người. */
  async function send(msg, to) {
    const r = room.value;
    if (!r || r.state !== ConnectionState.Connected) return;
    await r.localParticipant.publishData(encodeMessage(msg), {
      reliable: true,
      ...(to ? { destinationIdentities: [].concat(to) } : {}),
    });
  }

  async function sendText(text, topic, to) {
    const r = room.value;
    if (!r || r.state !== ConnectionState.Connected) return;
    await r.localParticipant.sendText(text, { topic, ...(to ? { destinationIdentities: [].concat(to) } : {}) });
  }

  async function startAudio() {
    await room.value?.startAudio();
    canPlayAudio.value = room.value?.canPlaybackAudio ?? true;
  }

  async function disconnect() {
    const r = room.value;
    room.value = null;
    await r?.disconnect();
    audioBox?.remove();
    audioBox = null;
  }

  onBeforeUnmount(disconnect);

  return { room, version, state, canPlayAudio, disconnectReason, connect, disconnect, send, sendText, onMessage, onText, startAudio };
}
