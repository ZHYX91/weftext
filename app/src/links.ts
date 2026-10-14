/** Source-relative file URLs preserve literal Unicode, # and % in filenames. */
export function relativeFileTarget(sourcePath: string, targetPath: string): string {
  const source = sourcePath.replaceAll("\\", "/").split("/");
  const target = targetPath.replaceAll("\\", "/").split("/");
  if (source[0].toLowerCase() !== target[0].toLowerCase()) return absoluteFileTarget(targetPath);
  source.pop();
  let common = 0;
  while (common < source.length && common < target.length && source[common].toLowerCase() === target[common].toLowerCase()) common++;
  return [...source.slice(common).map(() => ".."), ...target.slice(common).map(segment => encodeURIComponent(segment))].join("/");
}

export function absoluteFileTarget(path: string): string {
  const normalized = path.replaceAll("\\", "/");
  if (!/^[a-zA-Z]:\//.test(normalized)) return path;
  const segments = normalized.split("/");
  return `file:///${segments[0]}/${segments.slice(1).map(segment => encodeURIComponent(segment)).join("/")}`;
}

/** Typst string escapes, including control characters unsupported by JSON's codec. */
export function quoteTypstString(text: string): string {
  let encoded = '"';
  for (const character of text) {
    const point = character.codePointAt(0)!;
    if (character === "\\" || character === '"') encoded += `\\${character}`;
    else if (point < 32 || point >= 0x7f && point <= 0x9f || point === 0x2028 || point === 0x2029) encoded += `\\u{${point.toString(16)}}`;
    else encoded += character;
  }
  return `${encoded}"`;
}
