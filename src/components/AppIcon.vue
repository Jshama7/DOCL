<script setup lang="ts">
import { computed } from "vue";

const props = withDefaults(
  defineProps<{ name: string; size?: number; stroke?: number }>(),
  { size: 18, stroke: 1.6 }
);

const icons: Record<string, string[]> = {
  home: [
    "M3.5 10.5 12 3.5l8.5 7",
    "M5.5 9.5V20a.5.5 0 0 0 .5.5h12a.5.5 0 0 0 .5-.5V9.5",
    "M9.5 20.5v-6h5v6",
  ],
  layers: [
    "M12 3.5 20 8l-8 4.5L4 8z",
    "M4 12l8 4.5 8-4.5",
    "M4 16l8 4.5 8-4.5",
  ],
  download: ["M12 4v10.5", "M7.5 10.5 12 15l4.5-4.5", "M4.5 19.5h15"],
  user: [
    "M12 4.5a3.5 3.5 0 1 0 0 7 3.5 3.5 0 0 0 0-7",
    "M5 20c0-3.6 3.1-6 7-6s7 2.4 7 6",
  ],
  settings: [
    "M4 8h9.5",
    "M18.5 8H20",
    "M4 16h3.5",
    "M12.5 16H20",
    "M16 5.8a2.2 2.2 0 1 0 0 4.4 2.2 2.2 0 0 0 0-4.4",
    "M8 13.8a2.2 2.2 0 1 0 0 4.4 2.2 2.2 0 0 0 0-4.4",
  ],
  log: [
    "M5.5 4.5h13a2 2 0 0 1 2 2v11a2 2 0 0 1-2 2h-13a2 2 0 0 1-2-2v-11a2 2 0 0 1 2-2z",
    "M7.5 9.5 10 12l-2.5 2.5",
    "M12.5 15h4",
  ],
  folder: [
    "M3.5 7.5a2 2 0 0 1 2-2h3.8l2 2.5h7.2a2 2 0 0 1 2 2v8a2 2 0 0 1-2 2H5.5a2 2 0 0 1-2-2z",
  ],
  play: ["M8 5.5v13l10-6.5z"],
  stop: ["M8 6.5h8a1.5 1.5 0 0 1 1.5 1.5v8a1.5 1.5 0 0 1-1.5 1.5H8A1.5 1.5 0 0 1 6.5 16V8A1.5 1.5 0 0 1 8 6.5z"],
  plus: ["M12 5v14", "M5 12h14"],
  trash: [
    "M4.5 7h15",
    "M9 7V4.5h6V7",
    "M6.5 7l1 12.5h9l1-12.5",
    "M10 11v5",
    "M14 11v5",
  ],
  check: ["M5 12.5 9.5 17 19 7.5"],
  refresh: ["M20 12a8 8 0 1 1-2.35-5.65", "M20 4.5V9h-4.5"],
  close: ["M6.5 6.5l11 11", "M17.5 6.5l-11 11"],
  minimize: ["M5 12h14"],
  maximize: ["M5.5 5.5h13v13h-13z"],
  "chevron-down": ["M6 9.5l6 6 6-6"],
  "chevron-up": ["M6 14.5l6-6 6 6"],
  copy: [
    "M8.5 8.5h9a1.5 1.5 0 0 1 1.5 1.5v9a1.5 1.5 0 0 1-1.5 1.5h-9A1.5 1.5 0 0 1 7 19v-9a1.5 1.5 0 0 1 1.5-1.5z",
    "M6 15.5H5a1.5 1.5 0 0 1-1.5-1.5V5A1.5 1.5 0 0 1 5 3.5h9A1.5 1.5 0 0 1 15.5 5v1",
  ],
  external: [
    "M14 4.5h5.5V10",
    "M19.5 4.5 11 13",
    "M18 14v4.5a1.5 1.5 0 0 1-1.5 1.5h-11A1.5 1.5 0 0 1 4 18.5v-11A1.5 1.5 0 0 1 5.5 6H10",
  ],
  image: [
    "M5.5 5.5h13a2 2 0 0 1 2 2v9a2 2 0 0 1-2 2h-13a2 2 0 0 1-2-2v-9a2 2 0 0 1 2-2z",
    "M3.8 16.5l4.4-4.8 3.3 3.6 2.5-2.7 4.2 4.3",
    "M8.6 8.3a1.2 1.2 0 1 1 0 2.4 1.2 1.2 0 0 1 0-2.4",
  ],
  cube: [
    "M12 3.5 20 8v8l-8 4.5L4 16V8z",
    "M4 8l8 4.5 8-4.5",
    "M12 12.5V20.5",
  ],
  grid: [
    "M4.5 4.5h6v6h-6z",
    "M13.5 4.5h6v6h-6z",
    "M4.5 13.5h6v6h-6z",
    "M13.5 13.5h6v6h-6z",
  ],
  package: [
    "M4.5 8 12 4l7.5 4v8L12 20l-7.5-4z",
    "M4.5 8 12 12l7.5-4",
    "M12 12v8",
    "M8 6 15.5 10",
  ],
  database: [
    "M5 6.5c0-1.4 3.1-2.5 7-2.5s7 1.1 7 2.5v11c0 1.4-3.1 2.5-7 2.5s-7-1.1-7-2.5z",
    "M5 6.5c0 1.4 3.1 2.5 7 2.5s7-1.1 7-2.5",
    "M5 12c0 1.4 3.1 2.5 7 2.5s7-1.1 7-2.5",
  ],
  sun: [
    "M12 7.5a4.5 4.5 0 1 0 0 9 4.5 4.5 0 0 0 0-9",
    "M12 3v2.2M12 18.8V21M3 12h2.2M18.8 12H21M5.6 5.6l1.6 1.6M16.8 16.8l1.6 1.6M18.4 5.6l-1.6 1.6M7.2 16.8l-1.6 1.6",
  ],
  "chevron-left": ["M14.5 6 9 12l5.5 6"],
  undo: ["M9 7 4.5 11.5 9 16", "M4.5 11.5H14a5 5 0 0 1 0 10h-2"],
  redo: ["M15 7l4.5 4.5L15 16", "M19.5 11.5H10a5 5 0 0 0 0 10h2"],
};

const paths = computed(() => icons[props.name] ?? []);
</script>

<template>
  <svg
    class="icon"
    :width="size"
    :height="size"
    viewBox="0 0 24 24"
    fill="none"
    :stroke-width="stroke"
    stroke="currentColor"
    stroke-linecap="round"
    stroke-linejoin="round"
    aria-hidden="true"
    focusable="false"
  >
    <path v-for="(d, index) in paths" :key="index" :d="d" />
  </svg>
</template>

<style scoped>
.icon {
  display: block;
  flex-shrink: 0;
  overflow: visible;
}
</style>
