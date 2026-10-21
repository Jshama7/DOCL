<script setup lang="ts">
import { onBeforeUnmount, onMounted } from "vue";

withDefaults(defineProps<{ title: string; width?: number }>(), { width: 440 });
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
        <header class="modal__head">
          <h3 class="modal__title">{{ title }}</h3>
          <button class="modal__close" aria-label="关闭" @click="$emit('close')">
            <svg width="12" height="12" viewBox="0 0 12 12">
              <path d="M2 2l8 8M10 2l-8 8" stroke="currentColor" stroke-width="1.6" />
            </svg>
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
  background: rgba(58, 44, 28, 0.28);
  backdrop-filter: blur(2px);
  display: grid;
  place-items: center;
  z-index: 80;
  animation: fade-in 0.15s ease;
}

.modal {
  background: var(--panel);
  border: 1px solid var(--line-strong);
  border-radius: 12px;
  box-shadow: 0 18px 50px rgba(60, 42, 22, 0.24);
  max-width: 88vw;
  max-height: 82vh;
  display: flex;
  flex-direction: column;
  animation: pop-in 0.18s cubic-bezier(0.2, 0.9, 0.3, 1.1);
}

.modal__head {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 16px 18px 12px;
  border-bottom: 1px solid var(--line);
}

.modal__title {
  font-size: 16px;
  font-weight: 700;
  color: var(--wood-dark);
}

.modal__close {
  width: 28px;
  height: 28px;
  border-radius: 7px;
  display: grid;
  place-items: center;
  color: var(--ink-soft);
  transition: background 0.13s ease;
}

.modal__close:hover {
  background: var(--panel-2);
}

.modal__body {
  padding: 18px;
  overflow-y: auto;
}

.modal__foot {
  padding: 12px 18px 16px;
  display: flex;
  justify-content: flex-end;
  gap: 10px;
  border-top: 1px solid var(--line);
}

@keyframes fade-in {
  from {
    opacity: 0;
  }
}

@keyframes pop-in {
  from {
    opacity: 0;
    transform: translateY(10px) scale(0.98);
  }
}
</style>
