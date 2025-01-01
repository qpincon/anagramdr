<script>
	import AnagramAnimation from '$lib/AnagramAnimation.svelte';
	import { onMount, tick } from 'svelte';
	import Logo from '$lib/Logo.svelte';
	import '../../fonts.scss';

	let origin = '';
	let destination = '';
	let duration;
	let color;
	let component;
	let blobUrl;

	onMount(async () => {
		const params = Object.fromEntries(new URLSearchParams(window.location.search));
		origin = params.origin;
		destination = params.destination;
		duration = params.duration ?? 5;
		color = params.color ? `#${params.color}` : '#000000';
		document.body.style.backgroundColor = '#202020';
		await tick();
		blobUrl = await component.startAnimation(true);
	});
</script>

<svelte:head>
	<meta property="og:title" content="Anagramdr" />
	<meta property="og:description" content="Anagrammeur d'expression depuis 2024." />
	<meta property="og:url" content="https://anagramdr.com" />
	<meta property="og:image" content="https://anagramdr.com/logo.png" />
</svelte:head>

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
		textColor={color}
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
