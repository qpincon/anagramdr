<script>
	import { tick } from 'svelte';
	import { areStringsAnagrams } from '.';
	import AnagramAnimation from './AnagramAnimation.svelte';
	import ColorPicker from 'svelte-awesome-color-picker';
	import { debounce } from 'lodash-es';
	import downloadIcon from '$lib/img/download.svg';
	// import shareIcon from '$lib/img/share.svg';
	import externalLinkIcon from '$lib/img/external-link.svg';

	const DEFAULT_ANIMATION_TIME = 5;
	const DEFAULT_COLOR = '#000000';

	export let origin;
	export let destination;

	let animationDurationSec = DEFAULT_ANIMATION_TIME;
	let textColor = DEFAULT_COLOR;

	let isValid = false;
	let animationComponent;

	$: if (origin || destination) {
		isValid = areStringsAnagrams(origin, destination);
		tick().then(() => {
			if (animationComponent) animationComponent.startAnimation();
		});
	}

	$: if (animationDurationSec !== DEFAULT_ANIMATION_TIME) {
		animateDebounced();
	}

	const animateDebounced = debounce(() => {
		if (!animationComponent) return;
		animationComponent.startAnimation();
	}, 1000);

	function download() {
		animationComponent.startAnimation(true);
	}


	function externalLink() {

	}
</script>

<div>
	{#if !isValid}
		<span> Problème: L'expression d'origine et de destination ont des letters pas en commun. </span>
	{:else}
		<div>
			<div class="canvas-container">
				<AnagramAnimation
					bind:this={animationComponent}
					sourceText={origin}
					targetText={destination}
					animationDurationMs={animationDurationSec * 1000}
					{textColor}
				></AnagramAnimation>
			</div>

			<div class="export">
				<img src={downloadIcon} title="Télécharger GIF" on:click={download}>
				<!-- <img src={shareIcon} title="Partager URL" on:click={share}> -->
				<img src={externalLinkIcon} title="Ouvrir outil d'export" on:click={externalLink}>
			</div>

			<div class="params">
				<ColorPicker
					bind:hex={textColor}
					label="Changer la couleur"
					--slider-width="25px"
					position="responsive"
				/>
				<div class="param">
                    <label for="duration">Durée de l'animation</label>
					<input
						type="range"
						id="duration"
						name="duration"
						min="0"
						max="10"
						bind:value={animationDurationSec}
					/>
				</div>
			</div>
		</div>
	{/if}
</div>

<style>
	.canvas-container {
		width: 80%;
		border: 1px solid #91795683;
		padding: 0 1rem;
		border-radius: 5px;
		margin: auto;
	}

	.export {
		display:flex;
		margin: auto;
		width: 100%;
		justify-content: center;
		& > img {
			width: 35px;
			cursor: pointer;
			margin: 10px;
		}
	}
    
    .params {
        margin-top: 2rem;
        display: flex;
        flex-direction: column;
        width: 100%;
    }
    .param {
        display: flex;
        align-items: center;

		& label {
			margin-right: 2rem;
		}
    }
    input[type="range"] {
        flex: 1 0 50px;
        margin: 1rem 0;
        max-width: 300px;
    }
</style>
