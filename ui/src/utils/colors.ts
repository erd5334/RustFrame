export type Rgba = [number, number, number, number];

export const rgbaToHex = (rgba: Rgba): string => {
  return `#${rgba[0].toString(16).padStart(2, "0")}${rgba[1]
    .toString(16)
    .padStart(2, "0")}${rgba[2].toString(16).padStart(2, "0")}`;
};

export const hexToRgba = (hex: string): Rgba => {
  const r = parseInt(hex.slice(1, 3), 16);
  const g = parseInt(hex.slice(3, 5), 16);
  const b = parseInt(hex.slice(5, 7), 16);
  return [r, g, b, 255];
};

export const rgbaToBgrU32 = (rgba: Rgba): number => {
  return rgba[0] | (rgba[1] << 8) | (rgba[2] << 16);
};
