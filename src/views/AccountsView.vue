<script setup lang="ts">
import { computed, onMounted, ref, watch } from "vue";
import { api, openExternal, saveSkinDialog } from "../api";
import AppIcon from "../components/AppIcon.vue";
import IconButton from "../components/IconButton.vue";
import PlayerAvatar from "../components/PlayerAvatar.vue";
import SkinModel from "../components/SkinModel.vue";
import WoodButton from "../components/WoodButton.vue";
import WoodInput from "../components/WoodInput.vue";
import WoodModal from "../components/WoodModal.vue";
import SkinEditorView from "./SkinEditorView.vue";
import {
  beginMsLogin,
  createOffline,
  loadSelectedSkin,
  notify,
  refreshAccounts,
  removeAccount,
  selectAccount,
  selectedAccount,
  store,
} from "../store";

const name = ref("");
const confirmId = ref<string | null>(null);

const loginVisible = computed(() => store.msLogin !== null);
const code = computed(() => store.msLogin?.code ?? null);
const status = computed(() => store.msLogin?.status ?? "loading");

const skinSource = computed(() => store.accountSkin?.data_url ?? "");
const skinModel = computed(
  () => store.accountSkin?.model ?? selectedAccount.value?.skin_model ?? "classic"
);
const skinSourceLabel = computed(() => {
  switch (store.accountSkin?.source) {
    case "local":
      return "本地自定义";
    case "official":
      return "Mojang 官方";
    default:
      return "默认皮肤";
  }
});

function addOffline() {
  const value = name.value.trim();
  if (!value) {
    notify("请输入离线用户名", "err");
    return;
  }
  createOffline(value);
  name.value = "";
}

function requestDelete(id: string) {
  if (confirmId.value === id) {
    confirmId.value = null;
    removeAccount(id);
  } else {
    confirmId.value = id;
    window.setTimeout(() => {
      if (confirmId.value === id) confirmId.value = null;
    }, 3000);
  }
}

async function pickAccount(id: string) {
  if (id === selectedAccount.value?.id) return;
  await selectAccount(id);
}

async function copyCode() {
  if (!code.value) return;
  try {
    await navigator.clipboard.writeText(code.value.user_code);
    notify("验证码已复制");
  } catch {
    notify("复制失败，请手动选中复制", "err");
  }
}

function openLink() {
  if (code.value) openExternal(code.value.verification_uri);
}

function closeLogin() {
  store.msLogin = null;
}

async function refreshOfficial() {
  await loadSelectedSkin();
  notify("已重新获取皮肤");
}

async function exportSkin() {
  const account = selectedAccount.value;
  if (!account) return;
  if (!store.accountSkin?.data_url) {
    notify("当前没有可导出的皮肤", "err");
    return;
  }
  const path = await saveSkinDialog(`${account.name}-skin.png`);
  if (!path) return;
  try {
    await api.saveAccountSkin(account.id, store.accountSkin.data_url, skinModel.value);
    await api.exportAccountSkin(account.id, path);
    await refreshAccounts();
    await loadSelectedSkin();
    notify("皮肤已导出");
  } catch (err) {
    notify(`导出失败：${String(err)}`, "err");
  }
}

async function clearLocalSkin() {
  const account = selectedAccount.value;
  if (!account) return;
  try {
    await api.clearAccountSkin(account.id);
    await refreshAccounts();
    await loadSelectedSkin();
    notify("已清除本地皮肤覆盖");
  } catch (err) {
    notify(`清除失败：${String(err)}`, "err");
  }
}

function editSkin() {
  if (!selectedAccount.value) return;
  store.skinEditorOpen = true;
}

function closeEditor() {
  store.skinEditorOpen = false;
  void loadSelectedSkin();
}

watch(
  () => selectedAccount.value?.id,
  () => {
    void loadSelectedSkin();
  }
);

watch(
  () => store.msLogin?.status,
  (value) => {
    if (value === "ok") {
      window.setTimeout(() => {
        store.msLogin = null;
      }, 1400);
    }
  }
);

onMounted(() => {
  if (!store.accountSkin && selectedAccount.value) void loadSelectedSkin();
});
</script>

<template>
  <div class="accounts-root">
    <SkinEditorView
      v-if="store.skinEditorOpen && selectedAccount"
      :account="selectedAccount"
      @close="closeEditor"
    />

    <div v-else class="page">
    <header class="page-head">
      <div>
        <span class="eyebrow">DOCL / ACCOUNTS</span>
        <h1 class="page-title">账号</h1>
        <p class="page-sub">切换账号、查看皮肤、编辑并保存自己的角色外观</p>
      </div>
      <span class="tag" :class="{ 'tag--grass': !!selectedAccount }">
        当前 {{ selectedAccount?.name ?? "未登录" }}
      </span>
    </header>

    <div class="accounts-layout">
      <section class="card accounts-list">
        <div class="panel-head">
          <div>
            <span class="eyebrow">PROFILES</span>
            <div class="section-title">账号列表</div>
          </div>
          <span class="muted small">{{ store.accounts.length }} 个</span>
        </div>

        <div v-if="!store.accounts.length" class="empty">
          <span class="empty__mark display">?</span>
          <p class="muted small">还没有账号，先在下方创建或登录。</p>
        </div>

        <div v-else class="account-list">
          <div
            v-for="(account, index) in store.accounts"
            :key="account.id"
            class="account"
            :class="{ 'account--active': account.id === selectedAccount?.id }"
            :style="{ '--i': index }"
            role="button"
            tabindex="0"
            @click="pickAccount(account.id)"
            @keydown.enter="pickAccount(account.id)"
          >
            <PlayerAvatar :account="account" :size="42" :radius="10" />
            <span class="account__info">
              <span class="row">
                <strong>{{ account.name }}</strong>
                <span class="tag" :class="{ 'tag--grass': account.kind === 'microsoft' }">
                  {{ account.kind === "microsoft" ? "微软" : "离线" }}
                </span>
              </span>
              <span class="muted small mono account__uuid">{{ account.uuid }}</span>
            </span>
            <span class="account__right">
              <AppIcon
                v-if="account.id === selectedAccount?.id"
                name="check"
                :size="16"
                class="account__check"
              />
              <IconButton
                icon="trash"
                :label="confirmId === account.id ? '确认移除账号' : '移除账号'"
                variant="danger"
                :size="28"
                @click.stop="requestDelete(account.id)"
              />
            </span>
          </div>
        </div>

        <div class="add-panel">
          <div class="row add-row">
            <WoodInput v-model="name" placeholder="离线用户名，例如 Steve" @keyup.enter="addOffline" />
            <WoodButton variant="primary" size="sm" @click="addOffline">
              <AppIcon name="plus" :size="14" />
              添加
            </WoodButton>
          </div>
          <WoodButton variant="soft" block @click="beginMsLogin">
            <AppIcon name="external" :size="15" />
            登录微软账号
          </WoodButton>
        </div>
      </section>

      <section v-if="selectedAccount" class="card player-panel">
        <div class="player-head">
          <div>
            <span class="eyebrow">SELECTED PROFILE</span>
            <h2 class="player-name display">{{ selectedAccount.name }}</h2>
            <p class="muted small mono">{{ selectedAccount.uuid }}</p>
          </div>
          <div class="row">
            <span class="tag" :class="{ 'tag--grass': selectedAccount.kind === 'microsoft' }">
              {{ selectedAccount.kind === "microsoft" ? "微软账号" : "离线账号" }}
            </span>
            <span class="tag">{{ skinSourceLabel }}</span>
          </div>
        </div>

        <div class="model-wrap">
          <div v-if="store.skinLoading" class="model-loading">
            <span class="skeleton" />
            <p class="muted small">正在获取皮肤…</p>
          </div>
          <SkinModel
            v-else
            :skin="skinSource"
            :seed="selectedAccount.uuid"
            :model="skinModel"
            :height="380"
          />
        </div>

        <div class="player-actions">
          <WoodButton variant="primary" sheen @click="editSkin">
            编辑皮肤
          </WoodButton>
          <WoodButton variant="soft" @click="exportSkin">
            <AppIcon name="download" :size="15" />
            导出 PNG
          </WoodButton>
          <WoodButton v-if="selectedAccount.kind === 'microsoft'" variant="ghost" @click="refreshOfficial">
            <AppIcon name="refresh" :size="14" />
            重新拉取官方皮肤
          </WoodButton>
          <WoodButton
            v-if="selectedAccount.kind === 'microsoft' && store.accountSkin?.source === 'local'"
            variant="danger"
            @click="clearLocalSkin"
          >
            清除本地覆盖
          </WoodButton>
        </div>

        <p class="player-note muted small">
          离线账号的自定义皮肤只用于启动器内预览与导出，游戏内原版不会显示；
          微软账号可保存本地，也可在编辑页选择上传到 Mojang。
        </p>
      </section>

      <section v-else class="card player-panel player-panel--empty">
        <p class="muted">选择或创建一个账号后，这里会显示 3D 皮肤。</p>
      </section>
    </div>

    <WoodModal v-if="loginVisible" title="微软账号登录" :width="470" @close="closeLogin">
      <div v-if="status === 'loading'" class="login-state">正在向微软申请登录码…</div>

      <div v-else-if="status === 'err'" class="login-state login-state--err">
        <p>{{ store.msLogin?.message }}</p>
        <WoodButton variant="soft" @click="beginMsLogin">重试</WoodButton>
      </div>

      <div v-else-if="status === 'ok'" class="login-state login-state--ok">
        登录成功，正在同步账号信息…
      </div>

      <div v-else class="login-code">
        <p class="muted">在浏览器中打开下面的地址，输入验证码：</p>
        <div class="login-code__row">
          <code class="login-code__code display">{{ code?.user_code }}</code>
          <IconButton icon="copy" label="复制验证码" variant="soft" :size="36" @click="copyCode" />
        </div>
        <p class="login-code__url mono">{{ code?.verification_uri }}</p>
        <WoodButton variant="primary" block sheen @click="openLink">
          <AppIcon name="external" :size="15" />
          打开登录页面
        </WoodButton>
        <p class="muted small">完成后请保持此窗口打开，启动器会自动完成登录。</p>
      </div>
    </WoodModal>
    </div>
  </div>
</template>

<style scoped>
.accounts-layout {
  display: grid;
  grid-template-columns: 330px minmax(0, 1fr);
  gap: 16px;
  align-items: start;
}

.accounts-list {
  padding: 18px 16px;
  display: flex;
  flex-direction: column;
  gap: 12px;
  position: sticky;
  top: 0;
}

.panel-head {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
}

.empty {
  display: flex;
  align-items: center;
  gap: 14px;
  padding: 20px 8px;
}

.empty__mark {
  font-size: 44px;
  line-height: 1;
  color: var(--line-strong);
}

.account-list {
  display: flex;
  flex-direction: column;
  gap: 7px;
  max-height: 460px;
  overflow-y: auto;
  padding-right: 2px;
}

.account {
  display: flex;
  align-items: center;
  gap: 11px;
  padding: 10px 10px;
  border: 1px solid var(--line);
  border-radius: 10px;
  text-align: left;
  background: linear-gradient(180deg, var(--panel), var(--panel-2));
  position: relative;
  overflow: hidden;
  transition: transform var(--dur-2) var(--ease-out-expo),
    border-color var(--dur-2) ease, box-shadow var(--dur-2) ease;
  animation: rise-in 480ms var(--ease-out-expo) both;
  animation-delay: calc(var(--i, 0) * 45ms);
}

.account:hover {
  transform: translateY(-2px);
  border-color: var(--walnut-light);
  box-shadow: var(--shadow-1);
}

.account--active {
  border-color: var(--moss);
  box-shadow: 0 0 0 3px rgba(var(--accent-rgb), 0.1);
}

.account--active::after {
  content: "";
  position: absolute;
  left: 0;
  top: 0;
  bottom: 0;
  width: 3px;
  background: var(--moss);
}

.account__info {
  flex: 1;
  min-width: 0;
  display: flex;
  flex-direction: column;
  gap: 2px;
}

.account__uuid {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.account__right {
  display: flex;
  align-items: center;
  gap: 2px;
  flex-shrink: 0;
}

.account__check {
  color: var(--moss);
}

.add-panel {
  border-top: 1px dashed var(--line);
  padding-top: 12px;
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.add-row {
  gap: 8px;
}

.player-panel {
  padding: 20px 22px 22px;
  display: flex;
  flex-direction: column;
  gap: 14px;
}

.player-panel--empty {
  min-height: 420px;
  display: grid;
  place-items: center;
}

.player-head {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: 16px;
}

.player-name {
  font-size: 34px;
  line-height: 1.1;
  color: var(--walnut-deep);
}

.model-wrap {
  position: relative;
}

.model-loading {
  height: 380px;
  border-radius: 14px;
  border: 1px solid var(--line);
  background: var(--panel-2);
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: 10px;
}

.model-loading .skeleton {
  width: 120px;
  height: 160px;
}

.player-actions {
  display: flex;
  flex-wrap: wrap;
  gap: 9px;
}

.player-note {
  padding-top: 8px;
  border-top: 1px dashed var(--line);
  line-height: 1.6;
}

.login-state {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 14px;
  padding: 20px 6px;
  text-align: center;
  color: var(--ink-soft);
}

.login-state--err {
  color: var(--danger);
}

.login-state--ok {
  color: var(--moss);
  font-weight: 700;
}

.login-code {
  display: flex;
  flex-direction: column;
  gap: 12px;
}

.login-code__row {
  display: flex;
  align-items: center;
  gap: 10px;
}

.login-code__code {
  flex: 1;
  height: 54px;
  display: grid;
  place-items: center;
  border-radius: 10px;
  background: var(--panel-2);
  border: 1px dashed var(--line-strong);
  font-size: 26px;
  letter-spacing: 0.24em;
  color: var(--walnut-deep);
  user-select: text;
}

.login-code__url {
  font-size: 12px;
  color: var(--ink-soft);
  word-break: break-all;
  user-select: text;
}

@media (max-width: 1080px) {
  .accounts-layout {
    grid-template-columns: 280px minmax(0, 1fr);
  }
}
</style>
