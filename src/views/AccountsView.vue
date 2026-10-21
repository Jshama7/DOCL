<script setup lang="ts">
import { computed, ref, watch } from "vue";
import { openExternal } from "../api";
import WoodButton from "../components/WoodButton.vue";
import WoodInput from "../components/WoodInput.vue";
import WoodModal from "../components/WoodModal.vue";
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
        <h1 class="page-title">账号</h1>
        <p class="page-sub">离线账号用于本地游玩，微软账号可进入正版服务器</p>
      </div>
      <span class="tag">当前 {{ selectedAccount?.name ?? "未登录" }}</span>
    </header>

    <div class="accounts">
      <section class="card card-pad">
        <div class="section-title">账号列表</div>

        <div v-if="!store.accounts.length" class="empty">
          还没有账号。右侧可以创建离线账号或登录微软账号。
        </div>

        <div v-else class="account-list">
          <div
            v-for="account in store.accounts"
            :key="account.id"
            class="account"
            :class="{ 'account--active': account.id === selectedAccount?.id }"
          >
            <div class="account__avatar">{{ account.name.slice(0, 1).toUpperCase() }}</div>
            <div class="account__info">
              <div class="row">
                <strong>{{ account.name }}</strong>
                <span class="tag" :class="{ 'tag--grass': account.kind === 'microsoft' }">
                  {{ account.kind === "microsoft" ? "微软" : "离线" }}
                </span>
              </div>
              <span class="muted small mono">{{ account.uuid }}</span>
            </div>
            <div class="account__actions">
              <WoodButton
                variant="ghost"
                size="sm"
                :disabled="account.id === selectedAccount?.id"
                @click="selectAccount(account.id)"
              >
                切换
              </WoodButton>
              <WoodButton variant="danger" size="sm" @click="requestDelete(account.id)">
                {{ confirmId === account.id ? "确认移除" : "移除" }}
              </WoodButton>
            </div>
          </div>
        </div>
      </section>

      <div class="stack">
        <section class="card card-pad">
          <div class="section-title">添加离线账号</div>
          <p class="muted small">用户名仅需 1–16 个字符，无需密码。</p>
          <div class="row add-row">
            <WoodInput v-model="name" placeholder="例如：Steve" @keyup.enter="addOffline" />
            <WoodButton variant="primary" @click="addOffline">添加</WoodButton>
          </div>
        </section>

        <section class="card card-pad">
          <div class="section-title">微软账号</div>
          <p class="muted small">
            使用设备码登录，不需要输入密码，登录信息仅保存在本机并以 Windows DPAPI 加密。
          </p>
          <WoodButton variant="primary" block @click="beginMsLogin">登录微软账号</WoodButton>
        </section>
      </div>
    </div>

    <WoodModal v-if="loginVisible" title="微软账号登录" :width="460" @close="closeLogin">
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
          <code class="login-code__code">{{ code?.user_code }}</code>
          <WoodButton variant="soft" size="sm" @click="copyCode">复制</WoodButton>
        </div>
        <p class="login-code__url mono">{{ code?.verification_uri }}</p>
        <WoodButton variant="primary" block @click="openLink">打开登录页面</WoodButton>
        <p class="muted small">完成后请保持此窗口打开，启动器会自动完成登录。</p>
      </div>
    </WoodModal>
  </div>
</template>

<style scoped>
.accounts {
  display: grid;
  grid-template-columns: minmax(0, 1.4fr) minmax(280px, 1fr);
  gap: 16px;
  align-items: start;
}

.section-title {
  margin-bottom: 12px;
}

.empty {
  padding: 30px 12px;
  text-align: center;
  color: var(--muted);
  font-size: 13px;
}

.account-list {
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.account {
  display: flex;
  align-items: center;
  gap: 12px;
  padding: 11px 12px;
  border: 1px solid var(--line);
  border-radius: 9px;
  transition: border-color 0.15s ease, background 0.15s ease;
}

.account:hover {
  border-color: var(--wood-light);
}

.account--active {
  border-color: #c9d9bf;
  background: #fcfdf8;
}

.account__avatar {
  width: 38px;
  height: 38px;
  border-radius: 8px;
  display: grid;
  place-items: center;
  background: linear-gradient(180deg, var(--grass), var(--grass-dark));
  color: #fffdf6;
  font-weight: 700;
  font-size: 16px;
  flex-shrink: 0;
}

.account__info {
  flex: 1;
  min-width: 0;
  display: flex;
  flex-direction: column;
  gap: 2px;
}

.account__info .mono {
  overflow: hidden;
  text-overflow: ellipsis;
}

.account__actions {
  display: flex;
  gap: 6px;
}

.add-row {
  margin-top: 10px;
}

.login-state {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 14px;
  padding: 18px 6px;
  text-align: center;
  color: var(--ink-soft);
}

.login-state--err {
  color: var(--danger);
}

.login-state--ok {
  color: var(--grass-dark);
  font-weight: 600;
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
  height: 48px;
  display: grid;
  place-items: center;
  border-radius: 9px;
  background: var(--panel-2);
  border: 1px dashed var(--line-strong);
  font-size: 24px;
  font-weight: 800;
  letter-spacing: 0.22em;
  color: var(--wood-dark);
  user-select: text;
}

.login-code__url {
  font-size: 12px;
  color: var(--ink-soft);
  word-break: break-all;
  user-select: text;
}
</style>
