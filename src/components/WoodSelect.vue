<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref } from "vue";

const props = defineProps<{
  modelValue: string;
  options: { value: string; label: string }[];
  disabled?: boolean;
}>();
const emit = defineEmits<{ (e: "update:modelValue", value: string): void }>();

const root = ref<HTMLElement | null>(null);
const open = ref(false);
const current = computed(
  () => props.options.find((o) => o.value === props.modelValue)?.label ?? props.modelValue
);

function pick(value: string) {
  emit("update:modelValue", value);
  open.value = false;
}

function onDocClick(event: MouseEvent) {
  if (root.value && !root.value.contains(event.target as Node)) open.value = false;
}

onMounted(() => document.addEventListener("mousedown", onDocClick));
onBeforeUnmount(() => document.removeEventListener("mousedown", onDocClick));
</script>

<template>
  <div ref="root" class="select" :class="{ 'select--open': open }">
    <button class="select__value" :disabled="disabled" @click.stop="open = !open">
      <span class="select__text">{{ current }}</span>
      <span class="select__chev" />
    </button>
    <transition name="drop">
      <div v-if="open" class="select__menu">
        <button
          v-for="option in options"
          :key="option.value"
          class="select__option"
          :class="{ 'select__option--active': option.value === modelValue }"
          @click.stop="pick(option.value)"
        >
          {{ option.label }}
        </button>
      </div>
    </transition>
  </div>
</template>

<style scoped>
.select {
  position: relative;
  min-width: 120px;
}

.select__value {
  width: 100%;
  height: 36px;
  border-radius: 8px;
  border: 1px solid var(--line-strong);
  background: var(--panel);
  padding: 0 32px 0 12px;
  text-align: left;
  position: relative;
  color: var(--ink);
  transition: border-color 0.15s ease, box-shadow 0.15s ease;
}

.select__value:hover,
.select--open .select__value {
  border-color: var(--wood-light);
  box-shadow: 0 0 0 3px rgba(139, 94, 52, 0.08);
}

.select__text {
  display: block;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.select__chev {
  position: absolute;
  right: 12px;
  top: 50%;
  width: 7px;
  height: 7px;
  border-right: 1.5px solid var(--ink-soft);
  border-bottom: 1.5px solid var(--ink-soft);
  transform: translateY(-70%) rotate(45deg);
  transition: transform 0.15s ease;
}

.select--open .select__chev {
  transform: translateY(-30%) rotate(225deg);
}

.select__menu {
  position: absolute;
  z-index: 30;
  left: 0;
  right: 0;
  top: calc(100% + 6px);
  background: var(--panel);
  border: 1px solid var(--line-strong);
  border-radius: 8px;
  padding: 4px;
  box-shadow: 0 10px 26px rgba(80, 58, 34, 0.16);
  max-height: 240px;
  overflow-y: auto;
}

.select__option {
  width: 100%;
  height: 32px;
  padding: 0 10px;
  border-radius: 6px;
  text-align: left;
  color: var(--ink-soft);
  transition: background 0.12s ease, color 0.12s ease;
}

.select__option:hover {
  background: #f6ecda;
  color: var(--wood-dark);
}

.select__option--active {
  background: var(--grass-soft);
  color: var(--grass-dark);
  font-weight: 600;
}

.drop-enter-active,
.drop-leave-active {
  transition: opacity 0.13s ease, transform 0.13s ease;
}

.drop-enter-from,
.drop-leave-to {
  opacity: 0;
  transform: translateY(-4px);
}
</style>
