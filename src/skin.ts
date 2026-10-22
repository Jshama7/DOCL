export const SKIN_SIZE = 64;

export function seedFromString(text: string): number {
  let hash = 2166136261;
  for (let index = 0; index < text.length; index += 1) {
    hash ^= text.charCodeAt(index);
    hash = Math.imul(hash, 16777619);
  }
  return Math.abs(hash);
}

const palettes = [
  { hair: "#4a3220", skin: "#e3b48a", eye: "#2b2118", shirt: "#5e7b49", pants: "#3f4a58" },
  { hair: "#2f2a25", skin: "#c98d62", eye: "#241d16", shirt: "#8a5731", pants: "#3a3128" },
  { hair: "#7a5230", skin: "#f0c9a2", eye: "#3a3128", shirt: "#6f8f5a", pants: "#44523e" },
  { hair: "#b08b56", skin: "#d9a878", eye: "#2b2118", shirt: "#47623a", pants: "#4a3b2c" },
  { hair: "#3b2a1d", skin: "#b97f55", eye: "#1d1812", shirt: "#c08a55", pants: "#3c3a33" },
];

function rect(
  ctx: CanvasRenderingContext2D,
  x: number,
  y: number,
  w: number,
  h: number,
  color: string
) {
  ctx.fillStyle = color;
  ctx.fillRect(x, y, w, h);
}

/** 生成 64×64 的基础皮肤，用于没有皮肤的离线账号 */
export function createDefaultSkin(seed: number): HTMLCanvasElement {
  const canvas = document.createElement("canvas");
  canvas.width = SKIN_SIZE;
  canvas.height = SKIN_SIZE;
  const ctx = canvas.getContext("2d")!;
  ctx.clearRect(0, 0, SKIN_SIZE, SKIN_SIZE);
  const palette = palettes[seed % palettes.length];

  // 头部六面
  rect(ctx, 8, 0, 8, 8, palette.hair);
  rect(ctx, 16, 0, 8, 8, palette.skin);
  rect(ctx, 0, 8, 8, 8, palette.hair);
  rect(ctx, 8, 8, 8, 8, palette.skin);
  rect(ctx, 16, 8, 8, 8, palette.skin);
  rect(ctx, 24, 8, 8, 8, palette.hair);
  // 脸部：刘海 + 眼睛 + 嘴
  rect(ctx, 8, 8, 8, 2, palette.hair);
  rect(ctx, 10, 12, 1, 1, palette.eye);
  rect(ctx, 13, 12, 1, 1, palette.eye);
  rect(ctx, 11, 14, 2, 1, "#9c6a52");

  // 身体六面
  rect(ctx, 20, 16, 8, 4, palette.shirt);
  rect(ctx, 28, 16, 8, 4, palette.shirt);
  rect(ctx, 16, 20, 4, 12, palette.shirt);
  rect(ctx, 20, 20, 8, 12, palette.shirt);
  rect(ctx, 28, 20, 4, 12, palette.shirt);
  rect(ctx, 32, 20, 8, 12, palette.shirt);
  // 腰带
  rect(ctx, 20, 28, 8, 2, palette.pants);

  // 右腿
  rect(ctx, 4, 16, 4, 4, palette.pants);
  rect(ctx, 8, 16, 4, 4, palette.pants);
  rect(ctx, 0, 20, 4, 12, palette.pants);
  rect(ctx, 4, 20, 4, 12, palette.pants);
  rect(ctx, 8, 20, 4, 12, palette.pants);
  rect(ctx, 12, 20, 4, 12, palette.pants);

  // 右臂
  rect(ctx, 44, 16, 4, 4, palette.shirt);
  rect(ctx, 48, 16, 4, 4, palette.shirt);
  rect(ctx, 40, 20, 4, 12, palette.shirt);
  rect(ctx, 44, 20, 4, 12, palette.shirt);
  rect(ctx, 48, 20, 4, 12, palette.shirt);
  rect(ctx, 52, 20, 4, 12, palette.shirt);

  // 左腿（现代布局）
  rect(ctx, 20, 48, 4, 4, palette.pants);
  rect(ctx, 24, 48, 4, 4, palette.pants);
  rect(ctx, 16, 52, 4, 12, palette.pants);
  rect(ctx, 20, 52, 4, 12, palette.pants);
  rect(ctx, 24, 52, 4, 12, palette.pants);
  rect(ctx, 28, 52, 4, 12, palette.pants);

  // 左臂（现代布局）
  rect(ctx, 36, 48, 4, 4, palette.shirt);
  rect(ctx, 40, 48, 4, 4, palette.shirt);
  rect(ctx, 32, 52, 4, 12, palette.shirt);
  rect(ctx, 36, 52, 4, 12, palette.shirt);
  rect(ctx, 40, 52, 4, 12, palette.shirt);
  rect(ctx, 44, 52, 4, 12, palette.shirt);

  return canvas;
}

export function canvasToDataUrl(canvas: HTMLCanvasElement): string {
  return canvas.toDataURL("image/png");
}

export async function imageFromSource(source: string): Promise<HTMLImageElement> {
  return new Promise((resolve, reject) => {
    const image = new Image();
    image.onload = () => resolve(image);
    image.onerror = () => reject(new Error("皮肤图片加载失败"));
    image.src = source;
  });
}

export async function canvasFromSource(source: string): Promise<HTMLCanvasElement> {
  const image = await imageFromSource(source);
  const canvas = document.createElement("canvas");
  canvas.width = SKIN_SIZE;
  canvas.height = SKIN_SIZE;
  const ctx = canvas.getContext("2d")!;
  ctx.clearRect(0, 0, SKIN_SIZE, SKIN_SIZE);
  ctx.imageSmoothingEnabled = false;
  ctx.drawImage(image, 0, 0, SKIN_SIZE, SKIN_SIZE);
  return canvas;
}
