<script>
	import { onMount, tick } from 'svelte';
	import { areStringsAnagrams } from '.';
	import AnagramAnimation from './AnagramAnimation.svelte';
	import ColorPicker from 'svelte-awesome-color-picker';
	import { debounce } from 'lodash-es';
	import shareIcon from '$lib/img/share.svg';
	import tippy from 'tippy.js';

	const DEFAULT_ANIMATION_TIME = 5;
	const DEFAULT_COLOR = '#000000';

	export let origin;
	export let destination;

	let animationDurationSec = DEFAULT_ANIMATION_TIME;
	let textColor = DEFAULT_COLOR;

	let isValid = false;
	let animationComponent;
	let shareButton;

	$: vizUrl = `/viz?origin=${origin}&destination=${destination}&duration=${animationDurationSec}&color=${textColor.replace('#', '')}`;
	$: if (origin || destination) {
		isValid = areStringsAnagrams(origin, destination);
		tick().then(() => {
			if (animationComponent) animationComponent.startAnimation();
		});
	}

	$: if (animationDurationSec !== DEFAULT_ANIMATION_TIME) {
		animateDebounced();
	}

	onMount(() => {
		tippy(shareButton, {
			duration: [100, 200],
			trigger: 'click',
			theme: 'light',
			content: 'Lien copié dans le presse-papier !',
			arrow: false,
			onShow(instance) {
				setTimeout(() => {
					instance.hide();
				}, 1000);
			}
		});
	});

	const animateDebounced = debounce(() => {
		if (!animationComponent) return;
		animationComponent.startAnimation();
	}, 1000);


	function shareClicked() {
		const curUrl = window.location;
		navigator.clipboard.writeText(encodeURI(`${curUrl.origin}${vizUrl}`));
	}
</script>

<div class="exporter">
	{#if !isValid}
		<span> Problème: L'expression d'origine et de destination ont des lettres pas en commun. </span>
	{:else}
	<div class="display">
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
				<img src={shareIcon} title="Partager URL" on:click={shareClicked} bind:this={shareButton} />
			</div>
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
						min="1"
						max="10"
						bind:value={animationDurationSec}
					/>
					<span> {animationDurationSec}s</span>
				</div>
			</div>
	{/if}
</div>

<style>
	.exporter {
		margin-top: 1rem;
	}
	.display {
		display: flex;
		justify-content: center;
	}
	.canvas-container {
		width: 80%;
		border: 1px solid #91795683;
		padding: 0 1rem;
		border-radius: 5px;
	}

	.export {
		flex: 0 1 35px;
		margin: auto auto auto 10px;
		& img {
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

		& span {
			margin-left: 1rem;
		}
	}
	input[type='range'] {
		flex: 1 0 50px;
		margin: 1rem 0;
		max-width: 250px;
	}
</style>
