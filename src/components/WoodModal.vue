<script setup lang="ts">
import { onBeforeUnmount, onMounted } from "vue";
import AppIcon from "./AppIcon.vue";

withDefaults(defineProps<{ title: string; width?: number }>(), { width: 460 });
const emit = defineEmits<{ (e: "close"): void }>();

function onKey(event: KeyboardEvent) {
  if (event.key === "Escape") emit("close");
}

onMounted(() => window.addEventListener("keydown", onKey));
onBeforeUnmount(() => window.removeEventListener("keydown", onKey));
</script>

<template>
  <teleport to="body">
    <div class="overlay" @mousedown.self="$emit('close')">
      <div class="modal" :style="{ width: `${width}px` }">
        <span class="modal__accent" />
        <header class="modal__head">
          <div>
            <span class="eyebrow">DOCL</span>
            <h3 class="modal__title">{{ title }}</h3>
          </div>
          <button class="modal__close" aria-label="关闭" @click="$emit('close')">
            <AppIcon name="close" :size="14" />
          </button>
        </header>
        <div class="modal__body"><slot /></div>
        <footer v-if="$slots.footer" class="modal__foot"><slot name="footer" /></footer>
      </div>
    </div>
  </teleport>
</template>

<style scoped>
.overlay {
  position: fixed;
  inset: 0;
  background: rgba(52, 38, 24, 0.32);
  backdrop-filter: blur(3px);
  display: grid;
  place-items: center;
  z-index: 80;
  animation: fade-in 180ms ease both;
}

.modal {
  position: relative;
  background: var(--panel);
  border: 1px solid var(--line-strong);
  border-radius: 16px;
  box-shadow: 0 30px 70px rgba(50, 34, 18, 0.32);
  max-width: 88vw;
  max-height: 84vh;
  display: flex;
  flex-direction: column;
  overflow: hidden;
  animation: pop-in 300ms var(--ease-out-expo) both;
}

.modal__accent {
  position: absolute;
  inset: 0 0 auto;
  height: 4px;
  background: linear-gradient(90deg, var(--moss), var(--moss-bright) 42%, var(--amber));
}

.modal__head {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  padding: 20px 20px 12px;
}

.modal__title {
  font-size: 18px;
  font-weight: 800;
  color: var(--walnut-deep);
  margin-top: 2px;
}

.modal__close {
  width: 30px;
  height: 30px;
  border-radius: 8px;
  display: grid;
  place-items: center;
  color: var(--ink-soft);
  transition: background var(--dur-1) ease, transform var(--dur-1) ease;
}

.modal__close:hover {
  background: var(--panel-2);
  transform: rotate(90deg);
}

.modal__body {
  padding: 8px 20px 20px;
  overflow-y: auto;
}

.modal__foot {
  padding: 13px 20px 17px;
  display: flex;
  justify-content: flex-end;
  gap: 10px;
  border-top: 1px solid var(--line);
  background: var(--panel-2);
}

@keyframes fade-in {
  from {
    opacity: 0;
  }
}

@keyframes pop-in {
  from {
    opacity: 0;
    transform: translateY(16px) scale(0.97);
  }
}
</style>
