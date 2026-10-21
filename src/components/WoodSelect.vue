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
          <span class="select__dot" />
          {{ option.label }}
        </button>
      </div>
    </transition>
  </div>
</template>

<style scoped>
.select {
  position: relative;
  min-width: 130px;
}

.select__value {
  width: 100%;
  height: 38px;
  border-radius: 9px;
  border: 1px solid var(--line-strong);
  background: linear-gradient(180deg, #fffdf6, #faf2e3);
  padding: 0 34px 0 13px;
  text-align: left;
  position: relative;
  color: var(--ink);
  transition: border-color var(--dur-2) ease, box-shadow var(--dur-2) ease;
}

.select__value:hover,
.select--open .select__value {
  border-color: var(--walnut-light);
  box-shadow: 0 0 0 3px rgba(111, 66, 36, 0.08);
}

.select__text {
  display: block;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.select__chev {
  position: absolute;
  right: 13px;
  top: 50%;
  width: 7px;
  height: 7px;
  border-right: 1.6px solid var(--ink-soft);
  border-bottom: 1.6px solid var(--ink-soft);
  transform: translateY(-70%) rotate(45deg);
  transition: transform var(--dur-2) var(--ease-out-expo);
}

.select--open .select__chev {
  transform: translateY(-30%) rotate(225deg);
}

.select__menu {
  position: absolute;
  z-index: 30;
  left: 0;
  right: 0;
  top: calc(100% + 7px);
  background: var(--panel);
  border: 1px solid var(--line-strong);
  border-radius: 10px;
  padding: 5px;
  box-shadow: var(--shadow-3);
  max-height: 260px;
  overflow-y: auto;
}

.select__option {
  width: 100%;
  height: 34px;
  padding: 0 10px;
  border-radius: 7px;
  text-align: left;
  color: var(--ink-soft);
  display: flex;
  align-items: center;
  gap: 8px;
  transition: background var(--dur-1) ease, color var(--dur-1) ease;
}

.select__option:hover {
  background: #f7eddc;
  color: var(--walnut-deep);
}

.select__dot {
  width: 4px;
  height: 4px;
  border-radius: 50%;
  background: transparent;
  transition: background var(--dur-1) ease;
}

.select__option--active {
  background: var(--moss-soft);
  color: var(--moss);
  font-weight: 700;
}

.select__option--active .select__dot {
  background: var(--moss);
}

.drop-enter-active,
.drop-leave-active {
  transition: opacity 140ms ease, transform 140ms var(--ease-out-expo);
}

.drop-enter-from,
.drop-leave-to {
  opacity: 0;
  transform: translateY(-5px) scale(0.99);
}
</style>
