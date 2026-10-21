<script setup lang="ts">
import { computed, reactive, ref, watch } from "vue";
import { api, pickDirectory, type Config } from "../api";
import WoodButton from "../components/WoodButton.vue";
import WoodInput from "../components/WoodInput.vue";
import WoodProgress from "../components/WoodProgress.vue";
import WoodSelect from "../components/WoodSelect.vue";
import WoodSlider from "../components/WoodSlider.vue";
import WoodToggle from "../components/WoodToggle.vue";
import {
  notify,
  refreshConfig,
  refreshJava,
  saveConfig,
  store,
} from "../store";

const form = reactive<Config>({
  game_dir: "",
  java_path: null,
  memory_mb: 4096,
  java_args: "",
  window_width: 1280,
  window_height: 720,
  fullscreen: false,
  download_source: "auto",
  concurrency: 8,
  animations: true,
  version_isolation: true,
  selected_version: null,
  selected_account: null,
});

watch(
  () => store.config,
  (config) => {
    if (config) Object.assign(form, config);
  },
  { immediate: true }
);

const runtimeMajor = ref("21");
const saving = ref(false);
const runtimeBusy = ref(false);

const sourceOptions = [
  { value: "auto", label: "自动（优先镜像）" },
  { value: "mirror", label: "仅 BMClAPI 镜像" },
  { value: "official", label: "仅官方源" },
];

const javaOptions = [
  { value: "8", label: "Java 8" },
  { value: "17", label: "Java 17" },
  { value: "21", label: "Java 21" },
  { value: "25", label: "Java 25" },
];

const javaProgress = computed(() => {
  const p = store.progress;
  if (!p || p.stage !== "java" || !p.active) return null;
  if (p.bytes_total > 0) return Math.min(100, (p.bytes_done / p.bytes_total) * 100);
  if (p.files_total > 0) return Math.min(100, (p.files_done / p.files_total) * 100);
  return 0;
});

async function chooseDir() {
  const dir = await pickDirectory();
  if (dir) form.game_dir = dir;
}

async function rescanJava() {
  await refreshJava();
  notify(`检测到 ${store.javaList.length} 个 Java`);
}

async function installRuntime() {
  if (runtimeBusy.value) return;
  runtimeBusy.value = true;
  try {
    const major = Number(runtimeMajor.value);
    const info = await api.installJava(major);
    await refreshJava();
    await refreshConfig();
    notify(`Java ${major} 已安装：${info.path}`);
  } catch (err) {
    notify(`安装 Java 失败：${String(err)}`, "err");
  } finally {
    runtimeBusy.value = false;
  }
}

async function save() {
  if (!form.game_dir.trim()) {
    notify("请先选择游戏目录", "err");
    return;
  }
  saving.value = true;
  try {
    await saveConfig({ ...form });
    notify("设置已保存");
  } finally {
    saving.value = false;
  }
}

function useAutoJava() {
  form.java_path = null;
  notify("已改为自动选择 Java");
}
</script>

<template>
  <div class="page">
    <header class="page-head">
      <div>
        <h1 class="page-title">设置</h1>
        <p class="page-sub">启动参数、Java 运行时与下载策略</p>
      </div>
      <WoodButton variant="primary" :disabled="saving" @click="save">
        {{ saving ? "保存中…" : "保存设置" }}
      </WoodButton>
    </header>

    <section class="card card-pad">
      <div class="section-title">游戏与启动</div>
      <div class="setting">
        <div class="setting__label">
          <strong>游戏目录</strong>
          <span>versions / assets / libraries 都放在这里</span>
        </div>
        <div class="setting__control setting__control--wide">
          <div class="row">
            <WoodInput v-model="form.game_dir" mono />
            <WoodButton variant="ghost" size="sm" @click="chooseDir">选择</WoodButton>
          </div>
        </div>
      </div>
      <div class="setting">
        <div class="setting__label">
          <strong>版本隔离</strong>
          <span>每个版本使用独立的游戏目录，存档互不影响</span>
        </div>
        <div class="setting__control">
          <WoodToggle v-model="form.version_isolation" />
        </div>
      </div>
      <div class="setting">
        <div class="setting__label">
          <strong>启动窗口尺寸</strong>
          <span>游戏内分辨率，按需调整</span>
        </div>
        <div class="setting__control">
          <div class="row">
            <WoodInput
              :model-value="String(form.window_width)"
              placeholder="宽"
              @update:model-value="form.window_width = Number($event) || 1280"
            />
            <span class="muted">×</span>
            <WoodInput
              :model-value="String(form.window_height)"
              placeholder="高"
              @update:model-value="form.window_height = Number($event) || 720"
            />
          </div>
        </div>
      </div>
    </section>

    <section class="card card-pad">
      <div class="row-between">
        <div class="section-title">Java 运行时</div>
        <WoodButton variant="ghost" size="sm" @click="rescanJava">重新检测</WoodButton>
      </div>
      <p class="muted small">
        启动器会按版本要求自动挑选 Java；手动选择后将优先使用指定路径。
      </p>

      <div class="java-list">
        <button
          v-for="java in store.javaList"
          :key="java.path"
          class="java-item"
          :class="{ 'java-item--active': form.java_path === java.path }"
          @click="form.java_path = java.path"
        >
          <span class="java-item__major">Java {{ java.major }}</span>
          <span class="java-item__path mono">{{ java.path }}</span>
          <span class="tag">{{ java.source }}</span>
        </button>
        <div v-if="!store.javaList.length" class="java-empty">
          未检测到 Java。可以在下方直接下载官方运行时。
        </div>
      </div>

      <div class="row java-actions">
        <WoodButton variant="ghost" size="sm" @click="useAutoJava">恢复自动选择</WoodButton>
        <span class="muted small" v-if="form.java_path">指定：{{ form.java_path }}</span>
        <span class="muted small" v-else>当前：自动选择</span>
      </div>

      <div class="divider" />

      <div class="setting">
        <div class="setting__label">
          <strong>下载官方 Java 运行时</strong>
          <span>由 Mojang 提供，安装到启动器数据目录，不影响系统环境</span>
        </div>
        <div class="setting__control">
          <div class="row">
            <WoodSelect v-model="runtimeMajor" :options="javaOptions" />
            <WoodButton variant="soft" size="sm" :disabled="runtimeBusy" @click="installRuntime">
              {{ runtimeBusy ? "安装中…" : "下载并安装" }}
            </WoodButton>
          </div>
        </div>
      </div>

      <div v-if="javaProgress !== null" class="java-progress">
        <WoodProgress :value="javaProgress" />
        <span class="muted small">{{ store.progress?.message }}</span>
      </div>
    </section>

    <section class="card card-pad">
      <div class="section-title">性能与参数</div>
      <div class="setting">
        <div class="setting__label">
          <strong>最大内存</strong>
          <span>{{ form.memory_mb }} MB（约 {{ (form.memory_mb / 1024).toFixed(1) }} GB）</span>
        </div>
        <div class="setting__control setting__control--wide">
          <WoodSlider v-model="form.memory_mb" :min="1024" :max="16384" :step="512" />
        </div>
      </div>
      <div class="setting">
        <div class="setting__label">
          <strong>额外 JVM 参数</strong>
          <span>空格分隔，例如 -XX:+UseZGC</span>
        </div>
        <div class="setting__control setting__control--wide">
          <WoodInput v-model="form.java_args" mono placeholder="-XX:+UseG1GC" />
        </div>
      </div>
    </section>

    <section class="card card-pad">
      <div class="section-title">下载与界面</div>
      <div class="setting">
        <div class="setting__label">
          <strong>下载源</strong>
          <span>镜像失败时会自动回退到官方源</span>
        </div>
        <div class="setting__control">
          <WoodSelect v-model="form.download_source" :options="sourceOptions" />
        </div>
      </div>
      <div class="setting">
        <div class="setting__label">
          <strong>并发下载数</strong>
          <span>当前 {{ form.concurrency }}；网络不稳定时建议降低</span>
        </div>
        <div class="setting__control setting__control--wide">
          <WoodSlider v-model="form.concurrency" :min="1" :max="16" :step="1" />
        </div>
      </div>
      <div class="setting">
        <div class="setting__label">
          <strong>界面动效</strong>
          <span>关闭后页面切换与控件过渡立即完成</span>
        </div>
        <div class="setting__control">
          <WoodToggle v-model="form.animations" />
        </div>
      </div>
    </section>

    <div class="savebar">
      <WoodButton variant="primary" :disabled="saving" @click="save">
        {{ saving ? "保存中…" : "保存设置" }}
      </WoodButton>
    </div>
  </div>
</template>

<style scoped>
.setting {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 24px;
  padding: 13px 0;
  border-bottom: 1px dashed var(--line);
}

.setting:last-child {
  border-bottom: none;
}

.setting__label {
  display: flex;
  flex-direction: column;
  gap: 3px;
  min-width: 0;
}

.setting__label strong {
  font-size: 13.5px;
  color: var(--ink);
}

.setting__label span {
  font-size: 12px;
  color: var(--muted);
}

.setting__control {
  flex-shrink: 0;
  min-width: 150px;
  display: flex;
  justify-content: flex-end;
}

.setting__control--wide {
  flex: 1;
  max-width: 420px;
}

.java-list {
  display: flex;
  flex-direction: column;
  gap: 7px;
  margin: 12px 0;
}

.java-item {
  display: flex;
  align-items: center;
  gap: 12px;
  padding: 9px 12px;
  border-radius: 8px;
  border: 1px solid var(--line);
  background: var(--panel);
  text-align: left;
  transition: border-color 0.15s ease, background 0.15s ease;
}

.java-item:hover {
  border-color: var(--wood-light);
}

.java-item--active {
  border-color: #c9d9bf;
  background: #fbfdf8;
}

.java-item__major {
  font-weight: 700;
  color: var(--wood-dark);
  min-width: 72px;
}

.java-item__path {
  flex: 1;
  font-size: 12px;
  color: var(--ink-soft);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.java-empty {
  padding: 16px;
  border-radius: 8px;
  background: var(--panel-2);
  color: var(--muted);
  font-size: 12.5px;
  text-align: center;
}

.java-actions {
  justify-content: space-between;
}

.java-progress {
  display: flex;
  flex-direction: column;
  gap: 6px;
  margin-top: 10px;
}

.savebar {
  display: flex;
  justify-content: flex-end;
  padding-bottom: 6px;
}
</style>
