<script>
	import AnagramAnimation from '$lib/AnagramAnimation.svelte';
	import { onMount, tick } from 'svelte';
	import Logo from '$lib/Logo.svelte';

	export let data;

	let origin = '';
	let destination = '';
	let duration;
	let color;
	let component;
	let blobUrl;

	onMount(async () => {
		origin = data.origin;
		destination = data.destination;
		duration = data.duration ?? 5;
		color = data.color ?? '#000000';
		document.body.style.backgroundColor = '#202020';
		await tick();
		blobUrl = await component.startAnimation(true);
	});
</script>

{#if blobUrl}
	<div class="container">
		<div class="logo"><Logo fontSize="1em"></Logo></div>
		<img src={blobUrl} />
	</div>
{:else}
	<AnagramAnimation
		bind:this={component}
		sourceText={origin}
		targetText={destination}
		animationDurationMs={duration * 1000}
		{color}
	></AnagramAnimation>
{/if}

<style lang="scss">
	.container {
		width: fit-content;
		height: fit-content;
		& .logo {
			margin-bottom: 5px;
			width: fit-content;
		}
		position: absolute;
		margin: auto;
		inset: 0;
		& img {
			background: hsl(0, 0%, 90%);
		}
	}
</style>
