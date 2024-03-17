<script>
    import '../app.scss';
    import { onMount } from 'svelte';
    const svgDefinition = `
    <svg viewBox='0 0 400 400' xmlns='http://www.w3.org/2000/svg'>
        <filter id='noiseFilter'>
            <feTurbulence type='fractalNoise' baseFrequency='0.5' numOctaves='2' stitchTiles='stitch'/>
        </filter>
        <rect width='100%' height='100%' filter='url(#noiseFilter)'/>
    </svg>`;

    /**
	 * @param {string} data
	 */
    function encodeSVGDataImage(data) {
        const symbols = /[\r\n%#()<>?[\\\]^`{|}]/g;
        if (data.indexOf(`http://www.w3.org/2000/svg`) < 0) {
        data = data.replace(/<svg/g, `<svg xmlns='http://www.w3.org/2000/svg'`);
        }
        data = data.replace(/"/g, `'`);
        data = data.replace(/>\s{1,}</g, `><`);
        data = data.replace(/\s{2,}/g, ` `);
        data = data.replace(symbols, encodeURIComponent);
        return `data:image/svg+xml,${data}`
    }
    /**
	 * @type {{ style: { background: string; }; }}
	 */
    let noise;
    onMount(() => {
        noise.style['background-image'] = `url("${encodeSVGDataImage(svgDefinition)}")`;
    });
</script>
  
<div class="bg">
    <div bind:this={noise} class="bg noise"> </div>
    <div class="bg bg-1"> </div>
</div>
<slot />
  