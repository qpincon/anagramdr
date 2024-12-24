import * as gifenc from 'gifenc';

export async function loadAnagrams({input="", searchType="ROOT"} = {}) {
    const queryParams = new URLSearchParams({input, searchType});
    const res = await fetch(`query?${queryParams.toString()}`);
    return await res.json();
}

function download (buf, filename, type) {
    const blob = buf instanceof Blob ? buf : new Blob([buf], { type });
    const url = URL.createObjectURL(blob);
    const anchor = document.createElement("a");
    anchor.href = url;
    anchor.download = filename;
    anchor.click();
};
  
export async function encodeToGif ({ctx, renderFunction }) {
    // const context = canvas.getContext('2d');
    const width = ctx.canvas.width;
    const height = ctx.canvas.height;
    const fps = 30;
    const duration = 5;
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
        palette = gifenc.quantize(data, 256, { format, oneBitAlpha: true});
      }
  
      // Apply palette to RGBA data to get an indexed bitmap
      const index = gifenc.applyPalette(data, palette, format);
      // Write frame into GIF
      gif.writeFrame(index, width, height, { palette, delay, transparent: true });
      // gif.writeFrame(index, width, height, { palette, delay });
  
      // Wait a tick so that we don't lock up browser
      await new Promise(resolve => setTimeout(resolve, 0));
    }
  
    // Finalize stream
    gif.finish();
  
    // Get a direct typed array view into the buffer to avoid copying it
    const buffer = gif.bytesView();
    download(buffer, 'animation.gif', { type: 'image/gif' });
}

export function areStringsAnagrams(s1: string, s2: string): boolean {
  const diacriticsRemoved1 = s1.normalize('NFKD').replace(/[\u0300-\u036f]/g, '').split('').sort().join('').replaceAll(' ', '');
  const diacriticsRemoved2 = s2.normalize('NFKD').replace(/[\u0300-\u036f]/g, '').split('').sort().join('').replaceAll(' ', '');
  return diacriticsRemoved1 === diacriticsRemoved2;
}
