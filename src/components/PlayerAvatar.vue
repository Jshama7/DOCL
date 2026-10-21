<script setup lang="ts">
import { computed, ref, watch } from "vue";
import type { AccountView } from "../api";

const props = withDefaults(
  defineProps<{ account: AccountView | null; size?: number; radius?: number }>(),
  { size: 40, radius: 10 }
);

const failed = ref(false);
const sourceIndex = ref(0);

watch(
  () => props.account?.uuid,
  () => {
    failed.value = false;
    sourceIndex.value = 0;
  }
);

const sources = computed(() => {
  const account = props.account;
  if (!account || account.kind !== "microsoft") return [] as string[];
  const size = Math.max(64, Math.round(props.size * 2));
  const uuid = encodeURIComponent(account.uuid);
  return [
    `https://mc-heads.net/avatar/${uuid}/${size}`,
    `https://crafatar.com/avatars/${uuid}?size=${size}&overlay`,
  ];
});

const remote = computed(() => sources.value[sourceIndex.value] ?? "");

function onError() {
  if (sourceIndex.value + 1 < sources.value.length) {
    sourceIndex.value += 1;
  } else {
    failed.value = true;
  }
}

const seed = computed(() => {
  const text = props.account?.uuid ?? "docl";
  let hash = 2166136261;
  for (let index = 0; index < text.length; index += 1) {
    hash ^= text.charCodeAt(index);
    hash = Math.imul(hash, 16777619);
  }
  return Math.abs(hash);
});

const palettes = [
  { hair: "#4a3220", skin: "#e3b48a", eye: "#2b2118", shirt: "#5e7b49" },
  { hair: "#2f2a25", skin: "#c98d62", eye: "#241d16", shirt: "#8a5731" },
  { hair: "#7a5230", skin: "#f0c9a2", eye: "#3a3128", shirt: "#6f8f5a" },
  { hair: "#b08b56", skin: "#d9a878", eye: "#2b2118", shirt: "#47623a" },
  { hair: "#3b2a1d", skin: "#b97f55", eye: "#1d1812", shirt: "#c08a55" },
];

const fallback = computed(() => {
  const palette = palettes[seed.value % palettes.length];
  const cells: string[] = [];
  for (let y = 0; y < 8; y += 1) {
    for (let x = 0; x < 8; x += 1) {
      let color = palette.skin;
      if (y < 2) color = palette.hair;
      if (y === 0 && x > 2 && x < 5) color = palette.hair;
      if (y === 3 && (x === 1 || x === 2 || x === 5 || x === 6)) color = palette.eye;
      if (y === 5 && x >= 2 && x <= 5) color = palette.shirt;
      if (y === 6) color = palette.shirt;
      if (y === 7) color = palette.shirt;
      if ((x + y + seed.value) % 11 === 0) color = palette.hair;
      cells.push(`<rect x="${x}" y="${y}" width="1" height="1" fill="${color}"/>`);
    }
  }
  const svg = `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 8 8" shape-rendering="crispEdges">${cells.join(
    ""
  )}</svg>`;
  return `data:image/svg+xml;charset=utf-8,${encodeURIComponent(svg)}`;
});
</script>

<template>
  <span
    class="avatar"
    :style="{
      width: `${size}px`,
      height: `${size}px`,
      borderRadius: `${radius}px`,
    }"
  >
    <img
      v-if="remote && !failed"
      :src="remote"
      alt=""
      draggable="false"
      @error="onError"
    />
    <img v-else :src="fallback" alt="" draggable="false" class="avatar__pixel" />
  </span>
</template>

<style scoped>
.avatar {
  display: inline-grid;
  place-items: center;
  overflow: hidden;
  flex-shrink: 0;
  background: linear-gradient(160deg, var(--moss-bright), var(--moss));
  box-shadow: 0 4px 12px rgba(70, 48, 26, 0.18), 0 1px 0 rgba(255, 255, 255, 0.28) inset;
}

.avatar img {
  width: 100%;
  height: 100%;
  object-fit: cover;
  image-rendering: pixelated;
  display: block;
}

.avatar__pixel {
  image-rendering: pixelated;
  transform: scale(1.04);
}
</style>
