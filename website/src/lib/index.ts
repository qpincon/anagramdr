import * as gifenc from 'gifenc';

export async function loadAnagrams({ input = "", searchType = "ROOT", wordToInclude = null } = {}) {
  console.log(wordToInclude);
  const queryParams = new URLSearchParams({ input, 'search_type': searchType });
  if (wordToInclude) queryParams.append('word_to_include', wordToInclude);
  const res = await fetch(`query?${queryParams.toString()}`);
  return await res.json();
}

function download(buf, filename, type) {
  const blob = buf instanceof Blob ? buf : new Blob([buf], { type });
  const url = URL.createObjectURL(blob);
  const anchor = document.createElement("a");
  anchor.href = url;
  anchor.download = filename;
  anchor.click();
};

export async function encodeToGif({ ctx, renderFunction, duration }) {
  const width = ctx.canvas.width;
  const height = ctx.canvas.height;
  const fps = 30;
  const totalFrames = Math.ceil(duration * fps);

  const fpsInterval = 1 / fps;
  const delay = fpsInterval * 1000;

  // Setup an encoder that we will write frames into
  const gif = gifenc.GIFEncoder();
  let palette;
  for (let i = 0; i <= totalFrames; ++i) {
    // a t value 0..1 to animate the frame
    const progress = i / totalFrames;

    // Render to 2D context
    renderFunction(ctx, progress);

    // Get RGBA data from canvas
    const data = ctx.getImageData(0, 0, width, height).data;

    // Choose a pixel format: rgba4444, rgb444, rgb565
    const format = "rgba4444";
    if (i === 0) {
      palette = gifenc.quantize(data, 256, { format, oneBitAlpha: true });
    }

    // Apply palette to RGBA data to get an indexed bitmap
    const index = gifenc.applyPalette(data, palette, format);
    // Write frame into GIF
    gif.writeFrame(index, width, height, { palette, delay, transparent: true });

    // Wait a tick so that we don't lock up browser
    await new Promise(resolve => setTimeout(resolve, 0));
  }

  // Finalize stream
  gif.finish();

  // Get a direct typed array view into the buffer to avoid copying it
  const buffer = gif.bytesView();
  const blob = new Blob([buffer], { type: 'image/gif' });
  const url = URL.createObjectURL(blob);
  return url;
  // download(buffer, 'animation.gif', { type: 'image/gif' });
}

export function sortedStringNormalized(s: string): string {
  return s.normalize('NFKD').replace(/[\u0300-\u036f]/g, '').replace(/[^a-zA-Z0-9]/g, '').toLowerCase().split('').sort().join('').replaceAll(' ', '');
}

export function areStringsAnagrams(s1: string, s2: string): boolean {
  const diacriticsRemoved1 = sortedStringNormalized(s1);
  const diacriticsRemoved2 = sortedStringNormalized(s2);
  // console.log(diacriticsRemoved1, diacriticsRemoved2);
  return diacriticsRemoved1 === diacriticsRemoved2;
}
