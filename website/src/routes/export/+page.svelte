<script>
	import { onMount } from 'svelte';
	import { areStringsAnagrams } from '../../lib';
	import GifExporter from '../../lib/GifExporter.svelte';
	import Logo from '../../lib/Logo.svelte';
    
	let origin = "";
	let destination = "";

    // origin, destination
    export let data;

    onMount(async () => {
        console.log(data);
        origin = data.origin;
        destination = data.destination;
	});

    $: isValidAnagram = origin !== "" && destination !== "" && areStringsAnagrams(origin, destination);

</script>

<main>
	<header>
		<Logo></Logo>
	</header>

	<div class="content">
		<div class="origin">
			<label for="origin">Expression d'origine:</label>
			<input type="text" id="origin" name="origin" bind:value={origin} />
		</div>
		<div class="destination">
			<label for="destination">Expression de destination:</label>
			<input type="text" id="destination" name="destination" bind:value={destination} />
		</div>

        {#if isValidAnagram} 
            <GifExporter {origin} {destination} showExport={false}></GifExporter>
        {:else}
            <span style="color:#9B2318;">Les deux expressions ne sont pas des anagrammes </span>
        {/if}
	</div>
</main>

<style lang="scss">
	input[type='text'] {
		max-width: 30rem;
		height: 45px;
	}

    .content {
        padding-top: 1rem;
        margin: 1rem auto;
        max-width: 30rem;
    }
</style>
