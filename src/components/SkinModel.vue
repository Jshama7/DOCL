<script setup lang="ts">
import * as THREE from "three";
import { computed, onBeforeUnmount, onMounted, ref, watch } from "vue";
import { store } from "../store";
import { capeCanvasFromSource, createDefaultSkin, seedFromString, skinCanvasFromSource } from "../skin";

const props = withDefaults(
  defineProps<{
    skin?: string;
    cape?: string;
    seed?: string;
    model?: string;
    walking?: boolean;
    interactive?: boolean;
    height?: number;
  }>(),
  {
    skin: "",
    cape: "",
    seed: "docl",
    model: "classic",
    walking: true,
    interactive: true,
    height: 380,
  }
);

const host = ref<HTMLDivElement | null>(null);
let renderer: THREE.WebGLRenderer | null = null;
let scene: THREE.Scene | null = null;
let camera: THREE.PerspectiveCamera | null = null;
let group: THREE.Group | null = null;
let armLeft: THREE.Group | null = null;
let armRight: THREE.Group | null = null;
let legLeft: THREE.Group | null = null;
let legRight: THREE.Group | null = null;
let head: THREE.Group | null = null;
let capeGroup: THREE.Group | null = null;
let frame = 0;
let clock: THREE.Clock | null = null;
let resizeObserver: ResizeObserver | null = null;
let buildToken = 0;

const textures: THREE.Texture[] = [];
const materials: THREE.Material[] = [];
const geometries: THREE.BufferGeometry[] = [];

let rotationY = 0.45;
let rotationX = 0.05;
let dragging = false;
let lastX = 0;
let lastY = 0;

const animated = computed(() => store.config?.animations !== false);

interface FaceRegions {
  right: [number, number, number, number];
  left: [number, number, number, number];
  top: [number, number, number, number];
  bottom: [number, number, number, number];
  front: [number, number, number, number];
  back: [number, number, number, number];
}

const HEAD: FaceRegions = {
  right: [0, 8, 8, 8],
  left: [16, 8, 8, 8],
  top: [8, 0, 8, 8],
  bottom: [16, 0, 8, 8],
  front: [8, 8, 8, 8],
  back: [24, 8, 8, 8],
};

const BODY: FaceRegions = {
  right: [16, 20, 4, 12],
  left: [28, 20, 4, 12],
  top: [20, 16, 8, 4],
  bottom: [28, 16, 8, 4],
  front: [20, 20, 8, 12],
  back: [32, 20, 8, 12],
};

const LEG_RIGHT: FaceRegions = {
  right: [0, 20, 4, 12],
  left: [8, 20, 4, 12],
  top: [4, 16, 4, 4],
  bottom: [8, 16, 4, 4],
  front: [4, 20, 4, 12],
  back: [12, 20, 4, 12],
};

const LEG_LEFT: FaceRegions = {
  right: [16, 52, 4, 12],
  left: [24, 52, 4, 12],
  top: [20, 48, 4, 4],
  bottom: [24, 48, 4, 4],
  front: [20, 52, 4, 12],
  back: [28, 52, 4, 12],
};

/* ---------- 第二层（覆盖层）：游戏里让皮肤有立体感的部分，64x64 皮肤才有 ---------- */

const HAT: FaceRegions = {
  right: [32, 8, 8, 8],
  left: [48, 8, 8, 8],
  top: [40, 0, 8, 8],
  bottom: [48, 0, 8, 8],
  front: [40, 8, 8, 8],
  back: [56, 8, 8, 8],
};

const JACKET: FaceRegions = {
  right: [16, 36, 4, 12],
  left: [28, 36, 4, 12],
  top: [20, 32, 8, 4],
  bottom: [28, 32, 8, 4],
  front: [20, 36, 8, 12],
  back: [32, 36, 8, 12],
};

const PANTS_RIGHT: FaceRegions = {
  right: [0, 36, 4, 12],
  left: [8, 36, 4, 12],
  top: [4, 32, 4, 4],
  bottom: [8, 32, 4, 4],
  front: [4, 36, 4, 12],
  back: [12, 36, 4, 12],
};

const PANTS_LEFT: FaceRegions = {
  right: [0, 52, 4, 12],
  left: [8, 52, 4, 12],
  top: [4, 48, 4, 4],
  bottom: [8, 48, 4, 4],
  front: [4, 52, 4, 12],
  back: [12, 52, 4, 12],
};

/// 手臂的 UV 布局随模型不同：纤薄(Alex)模型的正面/顶面/底面只有 3 像素宽，
/// 内外侧与背面整体左移 1 像素；经典(Steve)模型全部 4 像素宽。
/// 区域原点两种模型相同（右臂 40,16 / 左臂 32,48 / 右袖 40,32 / 左袖 48,48）。
function armFaces(slim: boolean) {
  const fw = slim ? 3 : 4;
  const rightArm: FaceRegions = {
    right: [40, 20, 4, 12],
    left: [44 + fw, 20, 4, 12],
    top: [44, 16, fw, 4],
    bottom: [44 + fw, 16, fw, 4],
    front: [44, 20, fw, 12],
    back: [48 + fw, 20, 4, 12],
  };
  const leftArm: FaceRegions = {
    right: [32, 52, 4, 12],
    left: [36 + fw, 52, 4, 12],
    top: [36, 48, fw, 4],
    bottom: [36 + fw, 48, fw, 4],
    front: [36, 52, fw, 12],
    back: [40 + fw, 52, 4, 12],
  };
  const sleeveRight: FaceRegions = {
    right: [40, 36, 4, 12],
    left: [44 + fw, 36, 4, 12],
    top: [44, 32, fw, 4],
    bottom: [44 + fw, 32, fw, 4],
    front: [44, 36, fw, 12],
    back: [48 + fw, 36, 4, 12],
  };
  const sleeveLeft: FaceRegions = {
    right: [48, 52, 4, 12],
    left: [52 + fw, 52, 4, 12],
    top: [52, 48, fw, 4],
    bottom: [52 + fw, 48, fw, 4],
    front: [52, 52, fw, 12],
    back: [56 + fw, 52, 4, 12],
  };
  return { rightArm, leftArm, sleeveRight, sleeveLeft };
}

/// 披风是 10×16×1 的盒子，UV 区域与游戏内盒子展开一致
const CAPE: FaceRegions = {
  right: [0, 1, 1, 16],
  left: [11, 1, 1, 16],
  top: [1, 0, 10, 1],
  bottom: [11, 0, 10, 1],
  front: [1, 1, 10, 16],
  back: [12, 1, 10, 16],
};

function faceTexture(
  source: HTMLCanvasElement,
  region: [number, number, number, number],
  flipY = false
) {
  const [x, y, w, h] = region;
  const canvas = document.createElement("canvas");
  canvas.width = w;
  canvas.height = h;
  const ctx = canvas.getContext("2d")!;
  ctx.imageSmoothingEnabled = false;
  if (flipY) {
    // 游戏内盒子底面的展开方向与 three.js 默认朝向上下相反，翻转后与游戏一致
    ctx.translate(0, h);
    ctx.scale(1, -1);
  }
  ctx.drawImage(source, x, y, w, h, 0, 0, w, h);
  const texture = new THREE.CanvasTexture(canvas);
  texture.magFilter = THREE.NearestFilter;
  texture.minFilter = THREE.NearestFilter;
  texture.colorSpace = THREE.SRGBColorSpace;
  textures.push(texture);
  return texture;
}

function part(
  source: HTMLCanvasElement,
  regions: FaceRegions,
  width: number,
  height: number,
  depth: number
): THREE.Mesh {
  const order = [
    regions.right,
    regions.left,
    regions.top,
    regions.bottom,
    regions.front,
    regions.back,
  ];
  const mats = order.map(
    (region, index) =>
      new THREE.MeshStandardMaterial({
        map: faceTexture(source, region, index === 3),
        transparent: true,
        alphaTest: 0.02,
        roughness: 0.95,
        metalness: 0,
      })
  );
  materials.push(...mats);
  const geometry = new THREE.BoxGeometry(width, height, depth);
  geometry.translate(0, -height / 2, 0);
  geometries.push(geometry);
  return new THREE.Mesh(geometry, mats);
}

function clearScene() {
  if (!scene || !group) return;
  scene.remove(group);
  group.traverse((object) => {
    const mesh = object as THREE.Mesh;
    if (mesh.isMesh) {
      mesh.geometry?.dispose();
    }
  });
  materials.forEach((m) => m.dispose());
  textures.forEach((t) => t.dispose());
  materials.length = 0;
  textures.length = 0;
  geometries.length = 0;
  group = null;
  head = null;
  armLeft = null;
  armRight = null;
  legLeft = null;
  legRight = null;
  capeGroup = null;
}

async function build() {
  if (!scene) return;
  const token = ++buildToken;
  let canvas: HTMLCanvasElement;
  if (props.skin) {
    try {
      canvas = await skinCanvasFromSource(props.skin);
    } catch {
      canvas = createDefaultSkin(seedFromString(props.seed));
    }
  } else {
    canvas = createDefaultSkin(seedFromString(props.seed));
  }
  let capeCanvas: HTMLCanvasElement | null = null;
  if (props.cape) {
    try {
      capeCanvas = await capeCanvasFromSource(props.cape);
    } catch {
      capeCanvas = null;
    }
  }
  if (token !== buildToken) return;

  clearScene();
  const root = new THREE.Group();
  root.position.y = -16;

  const headMesh = part(canvas, HEAD, 8, 8, 8);
  head = new THREE.Group();
  head.position.y = 32;
  head.add(headMesh);
  // 第二层外扩量与游戏一致：头 0.5，身体四肢 0.25；透明像素会被 alphaTest 丢弃
  const hatMesh = part(canvas, HAT, 9, 9, 9);
  hatMesh.position.y = 0.5;
  head.add(hatMesh);

  const bodyMesh = part(canvas, BODY, 8, 12, 4);
  bodyMesh.position.y = 24;
  const jacketMesh = part(canvas, JACKET, 8.5, 12.5, 4.5);
  jacketMesh.position.y = 24.25;

  const armWidth = props.model === "slim" ? 3 : 4;
  const arms = armFaces(props.model === "slim");
  const armLeftGroup = new THREE.Group();
  armLeftGroup.position.set(-(4 + armWidth / 2), 24, 0);
  armLeftGroup.add(part(canvas, arms.leftArm, armWidth, 12, 4));
  const sleeveLeft = part(canvas, arms.sleeveLeft, armWidth + 0.5, 12.5, 4.5);
  sleeveLeft.position.y = 0.25;
  armLeftGroup.add(sleeveLeft);
  armLeft = armLeftGroup;

  const armRightGroup = new THREE.Group();
  armRightGroup.position.set(4 + armWidth / 2, 24, 0);
  armRightGroup.add(part(canvas, arms.rightArm, armWidth, 12, 4));
  const sleeveRight = part(canvas, arms.sleeveRight, armWidth + 0.5, 12.5, 4.5);
  sleeveRight.position.y = 0.25;
  armRightGroup.add(sleeveRight);
  armRight = armRightGroup;

  const legLeftGroup = new THREE.Group();
  legLeftGroup.position.set(-2, 12, 0);
  legLeftGroup.add(part(canvas, LEG_LEFT, 4, 12, 4));
  const pantsLeft = part(canvas, PANTS_LEFT, 4.5, 12.5, 4.5);
  pantsLeft.position.y = 0.25;
  legLeftGroup.add(pantsLeft);
  legLeft = legLeftGroup;

  const legRightGroup = new THREE.Group();
  legRightGroup.position.set(2, 12, 0);
  legRightGroup.add(part(canvas, LEG_RIGHT, 4, 12, 4));
  const pantsRight = part(canvas, PANTS_RIGHT, 4.5, 12.5, 4.5);
  pantsRight.position.y = 0.25;
  legRightGroup.add(pantsRight);
  legRight = legRightGroup;

  root.add(head, bodyMesh, jacketMesh, armLeft, armRight, legLeft, legRight);
  if (capeCanvas) {
    const capeRoot = new THREE.Group();
    // 挂在肩膀高度、身体背面（+Z 是脸的方向，披风在 -Z）；
    // 旋转 180° 让纹理的外侧面朝后，和游戏内一致
    capeRoot.position.set(0, 24, -2.5);
    capeRoot.rotation.y = Math.PI;
    capeRoot.add(part(capeCanvas, CAPE, 10, 16, 1));
    capeGroup = capeRoot;
    root.add(capeRoot);
  }
  group = root;
  scene.add(root);
}

function onPointerDown(event: PointerEvent) {
  if (!props.interactive) return;
  dragging = true;
  lastX = event.clientX;
  lastY = event.clientY;
  (event.currentTarget as HTMLElement).setPointerCapture?.(event.pointerId);
}

function onPointerMove(event: PointerEvent) {
  if (!dragging) return;
  rotationY += (event.clientX - lastX) * 0.008;
  rotationX = Math.max(-0.5, Math.min(0.5, rotationX + (event.clientY - lastY) * 0.004));
  lastX = event.clientX;
  lastY = event.clientY;
}

function onPointerUp() {
  dragging = false;
}

function animate() {
  frame = requestAnimationFrame(animate);
  if (!renderer || !scene || !camera || !clock) return;
  const t = clock.getElapsedTime();
  const walking = props.walking && animated.value;
  if (group) {
    group.rotation.y = rotationY;
    group.rotation.x = rotationX;
    if (walking) {
      const swing = Math.sin(t * 4);
      if (armLeft) armLeft.rotation.x = swing * 0.65;
      if (armRight) armRight.rotation.x = -swing * 0.65;
      if (legLeft) legLeft.rotation.x = -swing * 0.7;
      if (legRight) legRight.rotation.x = swing * 0.7;
      if (head) head.rotation.y = Math.sin(t * 1.3) * 0.08;
      group.position.y = -16 + Math.abs(Math.sin(t * 4)) * 0.55;
    } else {
      if (armLeft) armLeft.rotation.x = 0;
      if (armRight) armRight.rotation.x = 0;
      if (legLeft) legLeft.rotation.x = 0;
      if (legRight) legRight.rotation.x = 0;
      if (head) head.rotation.y = 0;
      group.position.y = -16;
    }
    if (capeGroup) {
      capeGroup.rotation.x = walking ? Math.sin(t * 2.2) * 0.08 : 0;
    }
  }
  renderer.render(scene, camera);
}

onMounted(async () => {
  const element = host.value;
  if (!element) return;
  scene = new THREE.Scene();
  camera = new THREE.PerspectiveCamera(32, 1, 0.1, 300);
  camera.position.set(0, 6, 64);
  camera.lookAt(0, 0, 0);

  renderer = new THREE.WebGLRenderer({ alpha: true, antialias: true });
  renderer.setPixelRatio(Math.min(window.devicePixelRatio, 2));
  renderer.outputColorSpace = THREE.SRGBColorSpace;
  renderer.domElement.style.width = "100%";
  renderer.domElement.style.height = "100%";
  renderer.domElement.style.touchAction = "none";
  element.appendChild(renderer.domElement);

  const ambient = new THREE.HemisphereLight(0xfff5e0, 0x3b2b1c, 1.15);
  scene.add(ambient);
  const key = new THREE.DirectionalLight(0xfff1d8, 1.5);
  key.position.set(14, 30, 26);
  scene.add(key);
  const fill = new THREE.DirectionalLight(0xa8c0d8, 0.55);
  fill.position.set(-20, 12, -18);
  scene.add(fill);

  clock = new THREE.Clock();

  renderer.domElement.addEventListener("pointerdown", onPointerDown);
  renderer.domElement.addEventListener("pointermove", onPointerMove);
  renderer.domElement.addEventListener("pointerup", onPointerUp);
  renderer.domElement.addEventListener("pointerleave", onPointerUp);

  resizeObserver = new ResizeObserver(() => {
    if (!renderer || !camera || !element) return;
    const width = element.clientWidth;
    const height = element.clientHeight || props.height;
    if (!width || !height) return;
    renderer.setSize(width, height, false);
    camera.aspect = width / height;
    camera.updateProjectionMatrix();
  });
  resizeObserver.observe(element);

  await build();
  animate();
});

watch(
  () => [props.skin, props.cape, props.seed, props.model],
  () => {
    void build();
  }
);

onBeforeUnmount(() => {
  cancelAnimationFrame(frame);
  resizeObserver?.disconnect();
  if (renderer) {
    renderer.domElement.removeEventListener("pointerdown", onPointerDown);
    renderer.domElement.removeEventListener("pointermove", onPointerMove);
    renderer.domElement.removeEventListener("pointerup", onPointerUp);
    renderer.domElement.removeEventListener("pointerleave", onPointerUp);
    renderer.dispose();
    renderer.domElement.remove();
    renderer = null;
  }
  clearScene();
  scene = null;
  camera = null;
  clock = null;
});
</script>

<template>
  <div
    ref="host"
    class="skin-model"
    :style="{ height: `${height}px` }"
    :class="{ 'skin-model--static': !interactive }"
  />
</template>

<style scoped>
.skin-model {
  width: 100%;
  border-radius: 14px;
  background: radial-gradient(circle at 50% 32%, rgba(var(--ambient-a), 0.18), transparent 66%),
    linear-gradient(180deg, var(--panel-2), var(--panel));
  border: 1px solid var(--line);
  overflow: hidden;
  cursor: grab;
  position: relative;
}

.skin-model:active {
  cursor: grabbing;
}

.skin-model--static {
  cursor: default;
}
</style>
