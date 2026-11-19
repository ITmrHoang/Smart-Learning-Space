<template>
  <div class="room">
    <header class="bar">
      <div class="title">
        <b>{{ info.session.title }}</b>
        <span class="muted small">{{ info.session.teacherName }} · {{ elapsed }}</span>
      </div>
      <div class="row">
        <hui-tag v-if="state !== 'connected'" size="sm" :color="state === 'disconnected' ? 'danger' : 'warning'">{{ STATE_LABELS[state] }}</hui-tag>
        <span class="small">{{ people.length }} người</span>
        <hui-button v-if="!canPlayAudio" size="sm" @hui-click="startAudio">🔊 Bật âm thanh</hui-button>
        <hui-button v-if="meOwner" size="sm" variant="danger" :loading="ending" @hui-click="endClass">Kết thúc buổi</hui-button>
        <hui-button v-else size="sm" variant="outline" @hui-click="leave">Rời phòng</hui-button>
      </div>
    </header>

    <hui-alert v-if="error" severity="error" :description="error" />

    <!-- Bị ngắt khỏi phòng: buổi kết thúc / bị mời ra / mất mạng -->
    <div v-if="ended" class="ended">
      <div class="panel stack ended-box">
        <h2 style="margin: 0">{{ ended.title }}</h2>
        <p class="muted" style="margin: 0">{{ ended.text }}</p>
        <div class="row">
          <hui-button v-if="ended.retry" @hui-click="reload">Vào lại</hui-button>
          <hui-button variant="outline" @hui-click="leave">{{ info.role === 'guest' ? 'Đóng' : 'Về trang buổi học' }}</hui-button>
        </div>
      </div>
    </div>

    <div class="body">
      <main class="stage-area">
        <div v-if="canDraw && !meTeacher" class="banner">✍ Giáo viên mời bạn lên bảng — viết lời giải trên bảng, cả lớp đang xem.</div>
        <div v-show="stageMode === 'board'" class="board-wrap" :class="{ drawing: canDraw && !meTeacher }">
          <hui-whiteboard
            ref="board"
            :readonly="!canDraw"
            :author="info.identity"
            :color="meTeacher ? '#111827' : '#2563eb'"
            @hui-change="onBoardChange"
            @hui-page="onBoardPage"
          />
        </div>
        <div v-if="stageMode === 'slides'" class="slides-wrap">
          <div v-if="meTeacher" class="row slides-pick">
            <hui-select size="sm" placeholder="Chọn bài để trình chiếu" :options="deckOptions" :value="slideDeck?.lessonId" @change="chooseDeck($event.detail.value)" />
            <span v-if="loadingDeck" class="muted small">Đang tải bài…</span>
          </div>
          <LessonSlides v-if="slideDeck" :content="slideDeck.content" :index="slideIndex" :readonly="!meTeacher" :noControls="!meTeacher" @change="onSlide" />
          <p v-else class="panel empty muted">{{ meTeacher ? 'Chọn một bài đã gắn vào buổi học để trình chiếu (bài phải đã xuất bản).' : 'Giáo viên chưa chọn bài trình chiếu.' }}</p>
        </div>
        <VideoTile
          v-if="stageMode === 'screen' && screenOwner"
          class="main-video"
          :participant="screenOwner"
          source="screen_share"
          fit="contain"
          :version="version"
          :label="`Màn hình của ${screenOwner.name}`"
        />
        <p v-else-if="stageMode === 'screen'" class="panel empty muted">Chưa có ai chia sẻ màn hình.</p>
        <VideoTile
          v-if="stageMode === 'camera' && teacher"
          class="main-video"
          :participant="teacher"
          :version="version"
          :label="teacher.name"
        />
        <div v-if="tiles.length" class="tiles">
          <VideoTile
            v-for="t in tiles"
            :key="t.identity"
            :participant="t.p"
            :version="version"
            :label="shortName(t.name)"
            :hand="hands.some((h) => h.identity === t.identity)"
          />
        </div>
      </main>

      <aside class="side">
        <hui-segmented :options="tabOptions" :value="tab" size="sm" @change="tab = $event.detail.value" />

        <!-- Mọi người -->
        <section v-if="tab === 'people'" class="stack scroll">
          <div v-if="hands.length" class="stack">
            <div class="row between">
              <b class="small">✋ Đang giơ tay ({{ hands.length }})</b>
              <hui-button v-if="meTeacher" size="sm" variant="text" @hui-click="clearHands">Hạ tay tất cả</hui-button>
            </div>
            <div v-for="(h, i) in hands" :key="h.identity" class="person hand">
              <span>{{ i + 1 }}. {{ h.name }} <span class="muted small">· {{ HAND_REASONS[h.reason] }} · {{ waited(h.at) }}</span></span>
              <span v-if="meTeacher" class="row actions">
                <hui-button size="sm" @hui-click="toggleWriter(h.identity)">Mời lên bảng</hui-button>
                <hui-button size="sm" variant="outline" @hui-click="setMic(h.identity, true)">Bật mic</hui-button>
              </span>
            </div>
          </div>

          <div v-if="meTeacher && waitingGuests.length" class="stack">
            <div class="row between">
              <b class="small">Khách đang chờ ({{ waitingGuests.length }})</b>
              <hui-button size="sm" variant="text" @hui-click="decideAll(true)">Cho vào tất cả</hui-button>
            </div>
            <div v-for="g in waitingGuests" :key="g.id" class="person">
              <span>{{ g.name }} <span class="muted small">(Khách)</span></span>
              <span class="row actions">
                <hui-button size="sm" @hui-click="decide(g, true)">Cho vào</hui-button>
                <hui-button size="sm" variant="text" @hui-click="decide(g, false)">Từ chối</hui-button>
              </span>
            </div>
          </div>

          <b class="small">Trong phòng ({{ people.length }})</b>
          <div v-for="x in people" :key="x.identity" class="person">
            <span>
              {{ x.name }}
              <hui-tag v-if="x.role === 'teacher'" size="sm" color="primary">Giáo viên</hui-tag>
              <hui-tag v-else-if="x.role === 'assistant'" size="sm" color="info">Trợ giảng</hui-tag>
              <hui-tag v-else-if="x.role === 'guest'" size="sm" color="secondary">Khách</hui-tag>
              <hui-tag v-if="writers.includes(x.identity)" size="sm" color="warning">Đang lên bảng</hui-tag>
              <span v-if="x.isLocal" class="muted small">(bạn)</span>
            </span>
            <span v-if="meTeacher && !x.isLocal" class="row actions">
              <template v-if="x.role === 'student'">
                <hui-button size="sm" variant="text" @hui-click="toggleWriter(x.identity)">{{ writers.includes(x.identity) ? 'Mời về chỗ' : 'Lên bảng' }}</hui-button>
                <hui-button size="sm" variant="text" @hui-click="setMic(x.identity, !micGranted.has(x.identity))">{{ micGranted.has(x.identity) ? 'Tắt mic' : 'Cho bật mic' }}</hui-button>
              </template>
              <hui-button size="sm" variant="text" @hui-click="kick(x)">Mời ra</hui-button>
            </span>
          </div>

          <div v-if="meOwner && info.session.access === 'public_link'" class="stack guest-link">
            <hui-button size="sm" variant="outline" @hui-click="makeGuestLink">{{ guestLink ? 'Đổi link khách' : 'Tạo link cho khách' }}</hui-button>
            <template v-if="guestLink">
              <code class="small">{{ guestLink }}</code>
              <hui-copy :value="guestLink" size="sm" />
            </template>
          </div>
        </section>

        <!-- Chat -->
        <section v-else-if="tab === 'chat'" class="chat">
          <div ref="chatBox" class="messages scroll">
            <p v-if="!chat.length" class="muted small">Chưa có tin nhắn.</p>
            <div v-for="m in chat" :key="m.id" class="message" :class="{ mine: m.from === info.identity, teacher: m.role === 'teacher' || m.role === 'assistant' }">
              <b class="small">{{ m.name }}</b>
              <span>{{ m.text }}</span>
            </div>
          </div>
          <form v-if="info.role !== 'guest'" class="row compose" @submit.prevent="sendChat">
            <input v-model="draft" class="native-input grow" maxlength="500" :disabled="chatLocked && !meTeacher" :placeholder="chatLocked && !meTeacher ? 'Giáo viên đang tắt chat' : 'Nhắn cho cả lớp…'" />
            <hui-button size="sm" type="submit" :disabled="!draft.trim() || (chatLocked && !meTeacher)">Gửi</hui-button>
          </form>
          <label v-if="meTeacher" class="small"><input :checked="chatLocked" type="checkbox" @change="lockChat($event.target.checked)" /> Tắt chat của học sinh</label>
        </section>

        <!-- Bình chọn -->
        <section v-else class="scroll">
          <LivePoll
            :isTeacher="meTeacher"
            :poll="poll"
            :results="results"
            :myVote="myVote"
            :canVoteRole="info.role === 'student'"
            :history="pollHistory"
            @start="startPoll"
            @vote="vote"
            @close="closePoll"
            @clear="poll = null"
          />
        </section>
      </aside>
    </div>

    <footer class="controls">
      <template v-if="canMic">
        <button type="button" class="ctl" :class="{ on: micOn }" @click="toggleMic">{{ micOn ? '🎙 Tắt mic' : '🎙 Bật mic' }}</button>
      </template>
      <template v-if="canCam">
        <button type="button" class="ctl" :class="{ on: camOn }" @click="toggleCam">{{ camOn ? '📷 Tắt camera' : '📷 Bật camera' }}</button>
      </template>
      <button v-if="canScreen" type="button" class="ctl" :class="{ on: screenOn }" @click="toggleScreen">{{ screenOn ? '🖥 Dừng chia sẻ' : '🖥 Chia sẻ màn hình' }}</button>
      <template v-if="meTeacher">
        <span class="sep" />
        <button v-for="m in STAGE_MODES" :key="m.value" type="button" class="ctl" :class="{ on: stageMode === m.value }" @click="setStage(m.value)">{{ m.label }}</button>
        <button type="button" class="ctl" :disabled="savingBoard" @click="saveBoard(true)">💾 {{ savingBoard ? 'Đang lưu…' : 'Lưu bảng' }}</button>
      </template>
      <template v-else-if="info.role === 'student'">
        <span class="sep" />
        <button type="button" class="ctl" :class="{ on: myHand }" @click="toggleHand('solve')">✋ {{ myHand ? 'Hạ tay' : 'Giơ tay — em giải' }}</button>
        <button v-if="!myHand" type="button" class="ctl" @click="toggleHand('ask')">❓ Em hỏi</button>
      </template>
    </footer>
  </div>
</template>

<script setup>
/**
 * Phòng học trực tuyến: khung chính (bảng / màn hình / camera giáo viên), ô video, giơ tay → mời lên bảng,
 * chat, bình chọn / quiz, khách trong phòng chờ. Giao thức tin nhắn: `utils/live.js`.
 */
import { message } from '@itmrhoang/h-ui/hui-message';
import { RoomEvent } from 'livekit-client';
import { computed, nextTick, onBeforeUnmount, onMounted, ref } from 'vue';

import { liveApi, sessionApi } from '../../api';
import LessonSlides from '../teaching/LessonSlides.vue';
import { canPublishSource, useLiveRoom } from '../../composables/useLiveRoom';
import { BOARD_SNAPSHOT_TOPIC, HAND_REASONS, SLIDES_TOPIC, formatElapsed, isTeacher, roleOf, shortName, tally, updateHands } from '../../utils/live';
import { uploadFile } from '../../utils/upload';
import LivePoll from './LivePoll.vue';
import VideoTile from './VideoTile.vue';

const STATE_LABELS = { idle: 'Đang chuẩn bị', connecting: 'Đang kết nối…', connected: 'Đã kết nối', reconnecting: 'Đang kết nối lại…', disconnected: 'Mất kết nối' };
const BASE_STAGE_MODES = [
  { value: 'board', label: '🧑‍🏫 Bảng' },
  { value: 'screen', label: '🖥 Màn hình' },
  { value: 'camera', label: '📷 Camera' },
];

/** Trình chiếu bài trong phòng (M36): bài đã gắn vào buổi học; giáo viên chọn bài → gửi nội dung cho cả lớp. */
const sessionLessons = ref([]);
const slideDeck = ref(null);
const slideIndex = ref(0);
const loadingDeck = ref(false);
const STAGE_MODES = computed(() =>
  sessionLessons.value.length ? [...BASE_STAGE_MODES, { value: 'slides', label: '📑 Slide' }] : BASE_STAGE_MODES,
);
const deckOptions = computed(() => sessionLessons.value.map((l) => ({ value: l.id, label: l.name })));

async function chooseDeck(lessonId) {
  if (!lessonId || lessonId === slideDeck.value?.lessonId) return;
  loadingDeck.value = true;
  try {
    const l = await sessionApi.lesson(props.info.session.id, lessonId);
    slideDeck.value = { lessonId, title: l.title, content: l.content };
    slideIndex.value = 0;
    live.sendText(JSON.stringify(slideDeck.value), SLIDES_TOPIC).catch(() => {});
  } catch (e) {
    message.error(e.message);
  } finally {
    loadingDeck.value = false;
  }
}

function onSlide(i) {
  slideIndex.value = i;
  if (meTeacher) live.send({ t: 'slide', index: i });
}

const props = defineProps({
  /** Thông tin vào phòng từ api: { url, token, identity, name, role, session } */
  info: { type: Object, required: true },
});
const emit = defineEmits(['left']);

const live = useLiveRoom();
const { version, state, canPlayAudio } = live;
/** Giáo viên chủ buổi */
const meOwner = props.info.role === 'teacher';
/** Người điều khiển phòng: chủ buổi hoặc trợ giảng của lớp (trừ kết thúc buổi, link khách — chỉ chủ buổi) */
const meTeacher = meOwner || props.info.role === 'assistant';
const board = ref(null);
const chatBox = ref(null);
const error = ref('');
const tab = ref('people');
const hands = ref([]);
const myHand = ref(false);
const writers = ref([]);
const stageMode = ref('board');
const chat = ref([]);
const chatLocked = ref(false);
const draft = ref('');
const poll = ref(null);
const results = ref(null);
const myVote = ref(null);
const pollHistory = ref([]);
const guests = ref([]);
const guestLink = ref('');
const micGranted = ref(new Set());
const savingBoard = ref(false);
const ending = ref(false);
const now = ref(Date.now());
let votes = new Map();
let pollCorrect = [];
let resultsTimer = null;
let guestTimer = null;
let clock = null;
let seq = 0;

// ---------- Người trong phòng ----------

const people = computed(() => {
  void version.value;
  const r = live.room.value;
  if (!r) return [];
  const all = [r.localParticipant, ...r.remoteParticipants.values()];
  return all.map((p) => ({ p, identity: p.identity, name: p.name || p.identity, role: roleOf(p), isLocal: p === r.localParticipant }));
});
const teacher = computed(() => people.value.find((x) => x.role === 'teacher')?.p ?? null);
const screenOwner = computed(() => {
  void version.value;
  return people.value.find((x) => x.p.getTrackPublication('screen_share') && x.p.isScreenShareEnabled)?.p ?? null;
});
/** Ô video nhỏ: ai đang bật camera (khung chính đang là camera giáo viên thì bỏ giáo viên). */
const tiles = computed(() => {
  void version.value;
  return people.value.filter((x) => x.p.isCameraEnabled && !(stageMode.value === 'camera' && x.role === 'teacher'));
});
const local = computed(() => {
  void version.value;
  return live.room.value?.localParticipant ?? null;
});
/** Đọc thẳng `version`: `local` luôn là cùng một đối tượng nên computed của nó không báo thay đổi. */
const fromLocal = (fn) =>
  computed(() => {
    void version.value;
    return fn(live.room.value?.localParticipant ?? null);
  });
const canMic = fromLocal((p) => canPublishSource(p, 'microphone'));
const canCam = fromLocal((p) => canPublishSource(p, 'camera'));
const canScreen = fromLocal((p) => canPublishSource(p, 'screen_share'));
const micOn = fromLocal((p) => !!p?.isMicrophoneEnabled);
const camOn = fromLocal((p) => !!p?.isCameraEnabled);
const screenOn = fromLocal((p) => !!p?.isScreenShareEnabled);
const canDraw = computed(() => meTeacher || writers.value.includes(props.info.identity));
const waitingGuests = computed(() => guests.value.filter((g) => g.status === 'waiting'));
const tabOptions = computed(() => [
  { value: 'people', label: `Mọi người${hands.value.length ? ` ✋${hands.value.length}` : ''}${waitingGuests.value.length ? ` · ${waitingGuests.value.length} chờ` : ''}` },
  { value: 'chat', label: 'Chat' },
  { value: 'poll', label: poll.value && !results.value?.closed ? 'Bình chọn •' : 'Bình chọn' },
]);
const elapsed = computed(() => formatElapsed((now.value - new Date(props.info.session.startsAt).getTime()) / 1000));
/** Lý do bị ngắt (DisconnectReason của LiveKit: 4 = bị mời ra, 5 = phòng đã đóng). */
const ended = computed(() => {
  if (state.value !== 'disconnected' || leaving.value) return null;
  const reason = live.disconnectReason.value;
  if (reason === 5) return { title: 'Buổi học đã kết thúc', text: 'Giáo viên đã kết thúc buổi học. Bảng và kết quả bình chọn được lưu ở trang buổi học.' };
  if (reason === 4) return { title: 'Bạn đã rời phòng', text: 'Giáo viên đã mời bạn ra khỏi phòng học.' };
  if (reason === 2) return { title: 'Bạn đã vào phòng ở nơi khác', text: 'Tài khoản này vừa vào phòng từ thiết bị / tab khác.' };
  return { title: 'Mất kết nối', text: 'Kết nối tới phòng học bị gián đoạn.', retry: true };
});
const leaving = ref(false);
const reload = () => window.location.reload();

const waited = (at) => `${Math.max(0, Math.round((now.value - at) / 1000))} giây`;

function participantOf(identity) {
  const r = live.room.value;
  if (!r) return null;
  return identity === r.localParticipant.identity ? r.localParticipant : r.remoteParticipants.get(identity) ?? null;
}

// ---------- Tin nhắn ----------

live.onMessage((msg, from) => {
  const fromTeacher = isTeacher(from);
  switch (msg.t) {
    case 'hand':
      if (roleOf(from) === 'student') hands.value = updateHands(hands.value, from.identity, from.name, !!msg.up, msg.reason);
      break;
    case 'hands-clear':
      if (fromTeacher) {
        hands.value = [];
        myHand.value = false;
      }
      break;
    case 'chat':
      if (!(chatLocked.value && !fromTeacher) && typeof msg.text === 'string') pushChat(from, msg.text);
      break;
    case 'chat-lock':
      if (fromTeacher) chatLocked.value = !!msg.locked;
      break;
    case 'stage':
      if (fromTeacher) stageMode.value = msg.mode;
      break;
    case 'slide':
      if (fromTeacher && Number.isInteger(msg.index)) slideIndex.value = msg.index;
      break;
    case 'board-op':
      if (fromTeacher || writers.value.includes(from.identity)) board.value?.applyRemote(msg.op);
      break;
    case 'board-grant':
      if (fromTeacher) writers.value = Array.isArray(msg.writers) ? msg.writers : [];
      break;
    case 'board-page':
      if (fromTeacher && board.value) board.value.page = msg.page;
      break;
    case 'poll':
      if (fromTeacher) {
        poll.value = msg.poll;
        results.value = { counts: msg.poll.options.map(() => 0), voters: 0, closed: false };
        myVote.value = null;
        tab.value = 'poll';
      }
      break;
    case 'vote':
      if (meTeacher && poll.value && msg.id === poll.value.id && roleOf(from) === 'student') {
        votes.set(from.identity, msg.choice);
        results.value = { counts: tally(votes, poll.value.options.length), voters: votes.size, closed: false };
        scheduleResults();
      }
      break;
    case 'poll-results':
      if (fromTeacher && poll.value && msg.id === poll.value.id) {
        results.value = msg;
        if (msg.closed) pollHistory.value = [...pollHistory.value, { ...poll.value, counts: msg.counts }];
      }
      break;
    case 'invite':
      if (fromTeacher && msg.board) {
        myHand.value = false;
        hands.value = updateHands(hands.value, props.info.identity, props.info.name, false);
        live.send({ t: 'hand', up: false });
        stageMode.value = 'board';
        message.info('Giáo viên mời bạn lên bảng.');
      }
      break;
    default:
  }
});

/** Bài trình chiếu do giáo viên / trợ giảng gửi. */
live.onText(SLIDES_TOPIC, (text, info) => {
  if (info.identity === props.info.identity || !isTeacher(participantOf(info.identity))) return;
  try {
    const d = JSON.parse(text);
    slideDeck.value = { lessonId: d.lessonId, title: d.title, content: String(d.content ?? '') };
    slideIndex.value = 0;
  } catch {
    /* bỏ qua gói hỏng */
  }
});

/** Người vào sau nhận trạng thái bảng / bình chọn / chat từ giáo viên. */
live.onText(BOARD_SNAPSHOT_TOPIC, (text, info) => {
  // Trợ giảng vào sau cũng nhận trạng thái từ giáo viên chủ buổi
  if (meOwner || !isTeacher(participantOf(info.identity))) return;
  try {
    const s = JSON.parse(text);
    board.value?.loadSnapshot(s.board);
    writers.value = s.writers ?? [];
    stageMode.value = s.stage ?? 'board';
    chatLocked.value = !!s.chatLocked;
    if (s.poll) {
      poll.value = s.poll;
      results.value = s.results;
    }
    pollHistory.value = s.pollHistory ?? [];
    if (s.slides) {
      slideDeck.value = { lessonId: s.slides.lessonId, title: s.slides.title, content: String(s.slides.content ?? '') };
      slideIndex.value = s.slides.index ?? 0;
    }
  } catch {
    /* bỏ qua ảnh chụp hỏng */
  }
});

function sendState(to) {
  if (!meTeacher || !board.value) return;
  // Có giáo viên chủ buổi trong phòng thì chỉ giáo viên gửi (tránh hai bản trạng thái lệch nhau)
  if (!meOwner && teacher.value) return;
  const s = {
    board: board.value.getSnapshot(),
    writers: writers.value,
    stage: stageMode.value,
    chatLocked: chatLocked.value,
    poll: poll.value,
    results: poll.value && (results.value?.closed || poll.value.kind === 'poll') ? results.value : null,
    pollHistory: pollHistory.value,
    slides: slideDeck.value ? { ...slideDeck.value, index: slideIndex.value } : null,
  };
  live.sendText(JSON.stringify(s), BOARD_SNAPSHOT_TOPIC, to).catch(() => {});
}

function pushChat(from, text) {
  chat.value = [...chat.value.slice(-199), { id: ++seq, from: from.identity, name: from.name, role: roleOf(from), text }];
  nextTick(() => chatBox.value?.scrollTo({ top: chatBox.value.scrollHeight }));
}

// ---------- Bảng ----------

function onBoardChange(e) {
  live.send({ t: 'board-op', op: e.detail.op });
}

function onBoardPage(e) {
  if (meTeacher) live.send({ t: 'board-page', page: e.detail.page });
}

async function toggleWriter(identity) {
  const id = props.info.session.id;
  if (writers.value.includes(identity)) {
    writers.value = writers.value.filter((w) => w !== identity);
    liveApi.turnEnd(id, identity).catch(() => {});
    if (micGranted.value.has(identity)) setMic(identity, false);
  } else {
    writers.value = [...writers.value, identity];
    hands.value = hands.value.filter((h) => h.identity !== identity);
    live.send({ t: 'invite', board: true }, identity);
    liveApi.turnStart(id, identity).catch(() => {});
    setStage('board');
  }
  live.send({ t: 'board-grant', writers: writers.value });
}

async function saveBoard(notify) {
  if (!board.value) return;
  savingBoard.value = true;
  try {
    for (let page = 0; page < board.value.pageCount; page += 1) {
      const blob = await (await fetch(board.value.toDataURL(page))).blob();
      const file = new File([blob], `bang-trang-${page + 1}.png`, { type: 'image/png' });
      const saved = await uploadFile(file, 'content');
      await liveApi.saveBoard(props.info.session.id, { page, fileId: saved.id, data: board.value.elements(page) });
    }
    if (notify) message.success('Đã lưu bảng — học sinh xem lại được trong trang buổi học.');
  } catch (e) {
    message.error(`Lưu bảng thất bại: ${e.message}`);
  } finally {
    savingBoard.value = false;
  }
}

// ---------- Giáo viên điều khiển ----------

function setStage(mode) {
  stageMode.value = mode;
  live.send({ t: 'stage', mode });
}

async function setMic(identity, on) {
  try {
    await liveApi.media(props.info.session.id, { identity, microphone: on });
    const next = new Set(micGranted.value);
    if (on) next.add(identity);
    else next.delete(identity);
    micGranted.value = next;
  } catch (e) {
    message.error(e.message);
  }
}

async function kick(x) {
  if (!window.confirm(`Mời ${x.name} ra khỏi phòng?`)) return;
  try {
    await liveApi.remove(props.info.session.id, x.identity);
    writers.value = writers.value.filter((w) => w !== x.identity);
    live.send({ t: 'board-grant', writers: writers.value });
  } catch (e) {
    message.error(e.message);
  }
}

function clearHands() {
  hands.value = [];
  live.send({ t: 'hands-clear' });
}

function lockChat(locked) {
  chatLocked.value = locked;
  live.send({ t: 'chat-lock', locked });
}

async function loadGuests() {
  guests.value = await liveApi.guests(props.info.session.id).catch(() => guests.value);
}

async function decide(g, admit) {
  guests.value = await liveApi.decide(props.info.session.id, g.id, admit).catch((e) => {
    message.error(e.message);
    return guests.value;
  });
}

async function decideAll(admit) {
  guests.value = await liveApi.decideAll(props.info.session.id, admit).catch(() => guests.value);
}

async function makeGuestLink() {
  try {
    guestLink.value = (await liveApi.guestLink(props.info.session.id)).url;
  } catch (e) {
    message.error(e.message);
  }
}

// ---------- Bình chọn ----------

function startPoll(def) {
  const p = { id: `${Date.now().toString(36)}${++seq}`, kind: def.kind, question: def.question, options: def.options };
  pollCorrect = def.correct;
  votes = new Map();
  poll.value = p;
  results.value = { counts: p.options.map(() => 0), voters: 0, closed: false };
  live.send({ t: 'poll', poll: p });
}

/** Bình chọn thường: công bố số phiếu liên tục (gom 1 giây); quiz: chỉ công bố khi đóng. */
function scheduleResults() {
  if (poll.value?.kind !== 'poll' || resultsTimer) return;
  resultsTimer = setTimeout(() => {
    resultsTimer = null;
    if (poll.value && !results.value?.closed) live.send({ t: 'poll-results', id: poll.value.id, ...results.value });
  }, 1000);
}

async function closePoll() {
  const p = poll.value;
  if (!p) return;
  const final = { counts: tally(votes, p.options.length), voters: votes.size, correct: pollCorrect, closed: true };
  results.value = final;
  live.send({ t: 'poll-results', id: p.id, ...final });
  pollHistory.value = [...pollHistory.value, { ...p, counts: final.counts }];
  liveApi
    .savePoll(props.info.session.id, { kind: p.kind, question: p.question, options: p.options, correct: pollCorrect, counts: final.counts, voters: final.voters })
    .catch((e) => message.error(`Không lưu được kết quả: ${e.message}`));
}

function vote(choice) {
  if (!poll.value || results.value?.closed) return;
  myVote.value = choice;
  const t = teacher.value;
  if (t) live.send({ t: 'vote', id: poll.value.id, choice }, t.identity);
}

// ---------- Học sinh ----------

function toggleHand(reason) {
  myHand.value = !myHand.value;
  live.send({ t: 'hand', up: myHand.value, reason });
  hands.value = updateHands(hands.value, props.info.identity, props.info.name, myHand.value, reason);
}

function sendChat() {
  const text = draft.value.trim();
  if (!text) return;
  live.send({ t: 'chat', text });
  pushChat(live.room.value.localParticipant, text);
  draft.value = '';
}

// ---------- Thiết bị ----------

async function toggleMic() {
  try {
    await local.value.setMicrophoneEnabled(!micOn.value);
  } catch (e) {
    message.error(`Không bật được mic: ${e.message}`);
  }
}

async function toggleCam() {
  try {
    await local.value.setCameraEnabled(!camOn.value);
  } catch (e) {
    message.error(`Không bật được camera: ${e.message}`);
  }
}

async function toggleScreen() {
  try {
    const on = !screenOn.value;
    await local.value.setScreenShareEnabled(on);
    if (meTeacher) setStage(on ? 'screen' : 'board');
  } catch (e) {
    if (e?.name !== 'NotAllowedError') message.error(`Không chia sẻ được màn hình: ${e.message}`);
  }
}

const startAudio = () => live.startAudio();

// ---------- Vào / ra ----------

async function leave() {
  leaving.value = true;
  await live.disconnect();
  emit('left');
}

async function endClass() {
  if (!window.confirm('Kết thúc buổi học? Mọi người sẽ rời phòng. Bảng được lưu lại để học sinh xem sau.')) return;
  ending.value = true;
  if (board.value && board.value.elements(0).length + board.value.pageCount > 1) await saveBoard(false);
  try {
    await liveApi.end(props.info.session.id);
  } catch (e) {
    message.error(e.message);
  }
  ending.value = false;
  await leave();
}

onMounted(async () => {
  clock = setInterval(() => (now.value = Date.now()), 1000);
  if (meTeacher) {
    // Bài đã gắn vào buổi học → chế độ "Slide"
    sessionApi
      .detail(props.info.session.id)
      .then((d) => (sessionLessons.value = d.lessons ?? []))
      .catch(() => {});
  }
  try {
    const r = await live.connect(props.info.url, props.info.token);
    if (meTeacher) {
      r.on(RoomEvent.ParticipantConnected, (p) => sendState(p.identity));
      const saved = await liveApi.boardData(props.info.session.id).catch(() => []);
      if (saved.length && board.value) board.value.loadSnapshot({ pages: saved, page: 0 });
      for (const p of r.remoteParticipants.values()) sendState(p.identity);
      if (props.info.session.access === 'public_link') {
        loadGuests();
        guestTimer = setInterval(loadGuests, 4000);
      }
    }
    r.on(RoomEvent.ParticipantPermissionsChanged, (prev, participant) => {
      if (participant !== r.localParticipant || meTeacher) return;
      const nowMic = canPublishSource(participant, 'microphone');
      if (nowMic && !prev?.canPublishSources?.includes(2)) {
        message.info('Giáo viên cho phép bạn bật mic.');
        participant.setMicrophoneEnabled(true).catch(() => {});
      }
    });
    r.on(RoomEvent.ParticipantDisconnected, (p) => {
      hands.value = hands.value.filter((h) => h.identity !== p.identity);
    });
  } catch (e) {
    error.value = `Không vào được phòng học: ${e.message}. Kiểm tra mạng rồi tải lại trang.`;
  }
});

onBeforeUnmount(() => {
  clearInterval(clock);
  clearInterval(guestTimer);
  clearTimeout(resultsTimer);
});

</script>

<style scoped>
.room {
  display: flex;
  flex-direction: column;
  height: 100vh;
  height: 100dvh;
  background: var(--hui-bg-layout, #eef2f7);
}
.bar {
  display: flex;
  flex-wrap: wrap;
  gap: 8px;
  justify-content: space-between;
  align-items: center;
  padding: 8px 14px;
  background: var(--hui-bg-container, #fff);
  border-bottom: 1px solid var(--hui-border-secondary, #eceef1);
}
.title {
  display: grid;
}
.body {
  flex: 1;
  display: grid;
  grid-template-columns: minmax(0, 1fr) 320px;
  gap: 10px;
  min-height: 0;
  padding: 10px;
}
.stage-area {
  display: grid;
  align-content: start;
  gap: 8px;
  min-height: 0;
  overflow-x: hidden;
  overflow-y: auto;
}
.ended {
  position: fixed;
  inset: 0;
  z-index: 50;
  display: grid;
  place-items: center;
  padding: 16px;
  background: rgba(17, 24, 39, 0.55);
}
.ended-box {
  width: min(440px, 100%);
}
.banner {
  padding: 6px 10px;
  border-radius: 8px;
  background: #fef3c7;
  color: #92400e;
  font-weight: 600;
}
.board-wrap.drawing {
  outline: 3px solid #2563eb;
  border-radius: 10px;
}
.slides-wrap {
  display: grid;
  gap: 8px;
}
.slides-pick hui-select {
  min-width: 260px;
}
.main-video {
  width: 100%;
}
.tiles {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(150px, 1fr));
  gap: 8px;
}
.side {
  display: grid;
  grid-template-rows: auto 1fr;
  gap: 8px;
  min-height: 0;
  padding: 10px;
  border-radius: 10px;
  background: var(--hui-bg-container, #fff);
}
.scroll {
  min-height: 0;
  overflow: auto;
  align-content: start;
}
.person {
  display: flex;
  flex-wrap: wrap;
  gap: 4px;
  justify-content: space-between;
  align-items: center;
  padding: 6px 0;
  border-bottom: 1px solid var(--hui-border-secondary, #eceef1);
}
.person.hand {
  padding: 6px 8px;
  border-radius: 8px;
  background: #fffbeb;
}
.actions {
  gap: 2px;
}
.guest-link code {
  word-break: break-all;
}
.chat {
  display: grid;
  grid-template-rows: 1fr auto auto;
  gap: 6px;
  min-height: 0;
}
.messages {
  display: grid;
  align-content: start;
  gap: 6px;
}
.message {
  display: grid;
  padding: 6px 8px;
  border-radius: 8px;
  background: var(--hui-bg-secondary, #f5f6f8);
}
.message.mine {
  background: color-mix(in srgb, var(--hui-primary, #0864ad) 10%, transparent);
}
.message.teacher b {
  color: var(--hui-primary, #0864ad);
}
.compose {
  flex-wrap: nowrap;
}
.grow {
  flex: 1;
  min-width: 0;
}
.controls {
  display: flex;
  flex-wrap: wrap;
  gap: 6px;
  justify-content: center;
  padding: 8px;
  background: #111827;
}
.ctl {
  padding: 8px 12px;
  border: 0;
  border-radius: 999px;
  background: #374151;
  color: #fff;
  font: inherit;
  font-size: 0.88rem;
  cursor: pointer;
}
.ctl.on {
  background: #2563eb;
}
.ctl:disabled {
  opacity: 0.6;
}
.sep {
  width: 1px;
  background: #4b5563;
  margin: 0 4px;
}
@media (max-width: 900px) {
  .body {
    grid-template-columns: 1fr;
    grid-template-rows: auto minmax(260px, 1fr);
  }
}
</style>
