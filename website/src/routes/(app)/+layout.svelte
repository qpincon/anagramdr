<script>
    import '../../app.scss';
    import { onMount } from 'svelte';
    const svgDefinition = `
    <svg viewBox='0 0 400 400' xmlns='http://www.w3.org/2000/svg'>
        <filter id='noiseFilter'>
            <feTurbulence type='fractalNoise' baseFrequency='0.3' numOctaves='4' stitchTiles='stitch'/>
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
  
<svelte:head>
	<title>Anagramdr</title>
	<meta name="description" content="Anagrammeur d'expression depuis 2024" />
</svelte:head>

<div class="bg">
    <div bind:this={noise} class="bg noise"> </div>
    <div class="bg bg-1"> </div>
    <div class="bg bg-2"> </div>
</div>
<slot />
<footer>
    <span>Anagramdr @ 2024</span>
    <a href="/export?origin=un exemple&destination=exemple nu" > Outil d'export </a>
    <a style="margin-left:auto; margin-right: 10px;" href="https://github.com/qpincon/anagramdr" target="_blank"> Voir code source </a>
</footer>
  