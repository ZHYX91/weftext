/** Byte positions for one immutable source snapshot. A prefix ending inside a
 * surrogate pair encodes its unmatched high surrogate as U+FFFD, like TextEncoder. */
export function sourceByteOffsets(source: string): Uint32Array {
  const offsets = new Uint32Array(source.length + 1);
  let bytes = 0;
  for (let i = 0; i < source.length; i++) {
    const unit = source.charCodeAt(i);
    if (unit >= 0xd800 && unit <= 0xdbff && source.charCodeAt(i + 1) >= 0xdc00 && source.charCodeAt(i + 1) <= 0xdfff) {
      offsets[i + 1] = bytes + 3;
      bytes += 4;
      offsets[++i + 1] = bytes;
    } else {
      bytes += unit < 0x80 ? 1 : unit < 0x800 ? 2 : 3;
      offsets[i + 1] = bytes;
    }
  }
  return offsets;
}
