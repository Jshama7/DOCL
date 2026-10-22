<script setup lang="ts">
import { computed, nextTick, onBeforeUnmount, onMounted, ref, watch } from "vue";

const props = defineProps<{
  modelValue: string;
  options: { value: string; label: string }[];
  disabled?: boolean;
}>();
const emit = defineEmits<{ (e: "update:modelValue", value: string): void }>();

const root = ref<HTMLElement | null>(null);
const trigger = ref<HTMLElement | null>(null);
const menu = ref<HTMLElement | null>(null);
const open = ref(false);
const menuStyle = ref<Record<string, string>>({});
const current = computed(
  () => props.options.find((o) => o.value === props.modelValue)?.label ?? props.modelValue
);

function pick(value: string) {
  emit("update:modelValue", value);
  open.value = false;
}

function positionMenu() {
  const el = trigger.value;
  if (!el) return;
  const rect = el.getBoundingClientRect();
  const menuWidth = menu.value?.offsetWidth || rect.width;
  const menuHeight = menu.value?.offsetHeight || 0;
  let left = rect.right - menuWidth;
  if (left < 8) left = Math.max(8, Math.min(rect.left, window.innerWidth - menuWidth - 8));
  let top = rect.bottom + 7;
  if (menuHeight && top + menuHeight > window.innerHeight - 10) {
    top = Math.max(10, rect.top - menuHeight - 7);
  }
  menuStyle.value = {
    left: `${Math.round(left)}px`,
    top: `${Math.round(top)}px`,
    minWidth: `${Math.round(rect.width)}px`,
  };
}

function closeMenu() {
  open.value = false;
}

/// 只有菜单外部的滚动才关闭，否则菜单自己滚动选项时会误关。
function onWindowScroll(event: Event) {
  const target = event.target;
  if (menu.value && target instanceof Node && menu.value.contains(target)) return;
  closeMenu();
}

function toggle() {
  if (props.disabled) return;
  if (!open.value) {
    const rect = trigger.value?.getBoundingClientRect();
    if (rect) {
      menuStyle.value = {
        left: `${Math.round(rect.right - rect.width)}px`,
        top: `${Math.round(rect.bottom + 7)}px`,
        minWidth: `${Math.round(rect.width)}px`,
      };
    }
  }
  open.value = !open.value;
}

function onDocClick(event: MouseEvent) {
  const target = event.target as Node;
  if (root.value?.contains(target) || menu.value?.contains(target)) return;
  closeMenu();
}

watch(open, (value) => {
  if (value) {
    nextTick(positionMenu);
    window.addEventListener("scroll", onWindowScroll, true);
    window.addEventListener("resize", closeMenu);
  } else {
    window.removeEventListener("scroll", onWindowScroll, true);
    window.removeEventListener("resize", closeMenu);
  }
});

onMounted(() => document.addEventListener("mousedown", onDocClick));
onBeforeUnmount(() => {
  document.removeEventListener("mousedown", onDocClick);
  window.removeEventListener("scroll", onWindowScroll, true);
  window.removeEventListener("resize", closeMenu);
});
</script>

<template>
  <div ref="root" class="select" :class="{ 'select--open': open }">
    <button
      ref="trigger"
      class="select__value"
      :disabled="disabled"
      @click.stop="toggle"
    >
      <span class="select__text">{{ current }}</span>
      <span class="select__chev" />
    </button>

    <Teleport to="body">
      <transition name="drop">
        <div
          v-if="open"
          ref="menu"
          class="select__menu"
          :style="menuStyle"
        >
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
    </Teleport>
  </div>
</template>

<style scoped>
.select {
  position: relative;
  min-width: 160px;
}

.select__value {
  width: 100%;
  height: 38px;
  border-radius: 9px;
  border: 1px solid var(--line-strong);
  background: linear-gradient(180deg, var(--panel), var(--panel-2));
  padding: 0 34px 0 13px;
  text-align: left;
  position: relative;
  color: var(--ink);
  white-space: nowrap;
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
  position: fixed;
  z-index: 95;
  width: max-content;
  max-width: min(320px, calc(100vw - 16px));
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
  white-space: nowrap;
  display: flex;
  align-items: center;
  gap: 8px;
  transition: background var(--dur-1) ease, color var(--dur-1) ease;
}

.select__option:hover {
  background: var(--panel-2);
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
