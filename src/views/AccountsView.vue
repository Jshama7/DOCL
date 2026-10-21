<script setup lang="ts">
import { computed, ref, watch } from "vue";
import { openExternal } from "../api";
import AppIcon from "../components/AppIcon.vue";
import IconButton from "../components/IconButton.vue";
import WoodButton from "../components/WoodButton.vue";
import WoodInput from "../components/WoodInput.vue";
import WoodModal from "../components/WoodModal.vue";
import PlayerAvatar from "../components/PlayerAvatar.vue";
import {
  beginMsLogin,
  createOffline,
  notify,
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
</script>

<template>
  <div class="page">
    <header class="page-head">
      <div>
        <span class="eyebrow">DOCL / ACCOUNTS</span>
        <h1 class="page-title">账号</h1>
        <p class="page-sub">离线账号用于本地游玩，微软账号可进入正版服务器</p>
      </div>
      <span class="tag" :class="{ 'tag--grass': !!selectedAccount }">
        当前 {{ selectedAccount?.name ?? "未登录" }}
      </span>
    </header>

    <div class="accounts">
      <section class="card accounts__list reveal">
        <div class="panel-head">
          <div>
            <span class="eyebrow">PROFILES</span>
            <div class="section-title">账号列表</div>
          </div>
          <span class="muted small">{{ store.accounts.length }} 个</span>
        </div>

        <div v-if="!store.accounts.length" class="empty">
          <span class="empty__mark display">?</span>
          <p class="muted small">还没有账号。右侧可以创建离线账号或登录微软账号。</p>
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
            @click="selectAccount(account.id)"
            @keydown.enter="selectAccount(account.id)"
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
            <span class="account__actions">
              <IconButton
                icon="trash"
                :label="confirmId === account.id ? '确认移除账号' : '移除账号'"
                variant="danger"
                :size="32"
                @click.stop="requestDelete(account.id)"
              />
            </span>
          </div>
        </div>
      </section>

      <div class="stack">
        <section class="card card-pad reveal" style="--i: 1">
          <span class="eyebrow">OFFLINE</span>
          <div class="section-title add-title">添加离线账号</div>
          <p class="muted small">用户名 1–16 个字符，无需密码。</p>
          <div class="row add-row">
            <WoodInput v-model="name" placeholder="例如：Steve" @keyup.enter="addOffline" />
            <WoodButton variant="primary" @click="addOffline">
              <AppIcon name="plus" :size="14" />
              添加
            </WoodButton>
          </div>
        </section>

        <section class="card card-pad ms-card reveal" style="--i: 2">
          <span class="eyebrow">MICROSOFT</span>
          <div class="section-title add-title">微软账号</div>
          <p class="muted small">
            设备码登录，不需要在启动器里输入密码。凭据只保存在本机，并用 Windows DPAPI 加密。
          </p>
          <WoodButton variant="primary" block sheen @click="beginMsLogin">
            <AppIcon name="external" :size="15" />
            登录微软账号
          </WoodButton>
        </section>
      </div>
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
          <IconButton
            icon="copy"
            label="复制验证码"
            variant="soft"
            :size="36"
            @click="copyCode"
          />
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
</template>

<style scoped>
.accounts {
  display: grid;
  grid-template-columns: minmax(0, 1.35fr) minmax(300px, 1fr);
  gap: 16px;
  align-items: start;
}

.accounts__list {
  padding: 20px 22px;
}

.panel-head {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  margin-bottom: 14px;
}

.empty {
  display: flex;
  align-items: center;
  gap: 16px;
  padding: 26px 6px;
}

.empty__mark {
  font-size: 48px;
  line-height: 1;
  color: var(--line-strong);
}

.account-list {
  display: flex;
  flex-direction: column;
  gap: 9px;
}

.account {
  display: flex;
  align-items: center;
  gap: 13px;
  padding: 12px 13px;
  border: 1px solid var(--line);
  border-radius: 11px;
  text-align: left;
  background: linear-gradient(180deg, #fffdf6, #fcf6ea);
  position: relative;
  overflow: hidden;
  transition: transform var(--dur-2) var(--ease-out-expo),
    border-color var(--dur-2) ease, box-shadow var(--dur-2) ease;
  animation: rise-in 480ms var(--ease-out-expo) both;
  animation-delay: calc(var(--i, 0) * 50ms);
}

.account:hover {
  transform: translateY(-2px);
  border-color: var(--walnut-light);
  box-shadow: var(--shadow-1);
}

.account--active {
  border-color: var(--moss);
  box-shadow: 0 0 0 3px rgba(94, 123, 73, 0.1);
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

.account__avatar {
  width: 42px;
  height: 42px;
  border-radius: 10px;
  display: grid;
  place-items: center;
  background: linear-gradient(160deg, var(--moss-bright), var(--moss) 62%, #47623a);
  color: #fffdf6;
  font-family: var(--font-display);
  font-weight: 700;
  font-size: 20px;
  flex-shrink: 0;
  box-shadow: 0 4px 10px rgba(70, 48, 26, 0.18), 0 1px 0 rgba(255, 255, 255, 0.25) inset;
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

.account__actions {
  flex-shrink: 0;
}

.add-title {
  margin: 2px 0 6px;
}

.add-row {
  margin-top: 10px;
}

.ms-card {
  background: linear-gradient(150deg, #fffdf6, #f6ecd9);
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
</style>
