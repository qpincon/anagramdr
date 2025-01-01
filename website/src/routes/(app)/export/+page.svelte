<script>
	import { onMount } from 'svelte';
	import { areStringsAnagrams } from '$lib';
	import GifExporter from '$lib/GifExporter.svelte';
	import Logo from '$lib/Logo.svelte';
	import { goto } from '$app/navigation';

	let origin = "";
	let destination = "";

    export let data;

    onMount(() => {
        origin = data.origin;
        destination = data.destination;
	});

    $: isValidAnagram = origin !== "" && destination !== "" && areStringsAnagrams(origin, destination);

	$: if (isValidAnagram) {
		const params = new URLSearchParams();
		params.set('origin', origin);
		params.set('destination', destination);
		const curUrl = window.location.search.replace('?', '');
		if (curUrl !== params.toString()) {
			goto(`/export?${params.toString()}`); 
		}
	}
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
		padding: 1rem 1rem 0;
        margin: 1rem auto;
        max-width: 30rem;
    }
</style>
