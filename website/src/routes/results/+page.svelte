<script>
	import { onMount } from 'svelte';
	import { loadAnagrams } from '$lib';
	import { goto } from '$app/navigation';
	import backImg from '$lib/img/back.svg';

	import Logo from '../../lib/Logo.svelte';
	import GifExporter from '../../lib/GifExporter.svelte';
	import { sortedStringNormalized } from '../../lib';
	import settingsIcon from '$lib/img/settings.svg';
	import tippy from 'tippy.js';

	const MAX_NB_LETTERS = 20;

	let results = [];
	let highlightedResult = null;

	export let data;
	let textSnapshot;
	let textField;
	let backError = null;
	let validationError = null;
	let settingsImgElement;
	let settingsContentElement;
	let toIncludeInput;
	let toIncludeError = null;
	let isSearchExact = false;
	let encoreTooltip;
	let displayEncore = false;

	$: searchType = isSearchExact ? 'EXACT' : 'ROOT';

	onMount(async () => {
		if (!data.input) goto(`/`);
		textSnapshot = textField = data.input;
		isSearchExact = data.search_type === 'ROOT' ? false : true;
		if (data.word_to_include) {
			toIncludeInput = data.word_to_include;
		}
		await refreshResults(data);
		settingsContentElement.style.display = 'block';
		tippy(settingsImgElement, {
			content: settingsContentElement,
			theme: 'light',
			trigger: 'click',
			interactive: true
		});

		tippy(encoreTooltip, {
			content: "<span> Il y a beaucoup d'anagrammes qui peuvent sortir d'une expression donnée, donc on en pioche juste quelques-uns, comme dans une loterie.</span> <br/>  <span>Résultat : chaque recherche, c'est la surprise du chef !</span>",
			theme: 'light',
			allowHTML: true
		});
	});

	function goToResults() {
		const params = new URLSearchParams();
		textSnapshot = textField;
		params.set('input', textSnapshot);
		params.set('search_type', searchType);
		if (toIncludeInput) params.set('word_to_include', toIncludeInput);
		goto(`/results?${params.toString()}`);
		refreshResults();
	}

	async function refreshResults() {
		highlightedResult = null;
		const res = await loadAnagrams({
			input: textSnapshot,
			searchType,
			wordToInclude: toIncludeInput
		});
		if (res.code) {
			backError = res.message;
			results = [];
		} else {
			results = res.anagrams;
			displayEncore = res.was_truncated;
			backError = null;
		}
	}

	function onSearchKeyUp(e) {
		if (e.key === 'Enter' && !validationError && !toIncludeError) {
			goToResults();
		}
	}

	function validateInput(e) {
		const normalized = sortedStringNormalized(e.target.value);
		if (normalized.length >= MAX_NB_LETTERS) {
			validationError = `Un maximum de ${MAX_NB_LETTERS} lettres est supporté`;
		} else {
			validationError = null;
		}
	}

	function changeSelectedResult(result) {
		highlightedResult = result[0];
	}

	function validateToInclude(e) {
		const txt = e.target.value;
		const totalInputSorted = sortedStringNormalized(textField).split('');
		const toIncludeSorted = sortedStringNormalized(txt).split('');
		for (const c of toIncludeSorted) {
			const index = totalInputSorted.findIndex((char) => char === c);
			if (index === -1) {
				toIncludeError = "Les lettres doivent être présentes dans l'expression de base";
				return;
			} else {
				totalInputSorted.splice(index, 1);
			}
		}
		toIncludeError = null;
	}

	function encore() {
		window.scrollTo(0,0);
		refreshResults();
	}
</script>

<svelte:window on:keyup={onSearchKeyUp} />
<main class:peek-opened={highlightedResult !== null} class:error={validationError}>
	<header>
		<div class="logo">
			<Logo></Logo>
		</div>
		<div class="input">
			<input
				class="search"
				on:keyup={onSearchKeyUp}
				on:input={validateInput}
				bind:value={textField}
				type="search"
			/>
			{#if validationError}
				<small class="error-message"> {validationError}</small>
			{/if}
		</div>
		<img class="settings-trigger" src={settingsIcon} bind:this={settingsImgElement} />
		<div class="settings" style="display: none;" bind:this={settingsContentElement}>
			<div class="title"><small> Paramètres de recherche </small></div>
			<div class="param">
				<input type="checkbox" id="exact" name="exact" bind:checked={isSearchExact} />
				<label for="exact"> Les mots cherchés doivent garder les accents, etc.</label>
			</div>
			<div class="param">
				<label for="include"> Les résultats doivent inclure ce mot:</label>
				<div class="include-input">
					<input
						type="text"
						id="include"
						name="include"
						bind:value={toIncludeInput}
						on:input={validateToInclude}
						on:keyup={onSearchKeyUp}
					/>
					{#if toIncludeError}
						<span class="error-message include"> {toIncludeError} </span>
					{/if}
				</div>
			</div>
		</div>
	</header>

	{#if backError}
		<div class="error-message" style="margin: auto;">{backError}</div>
	{:else}
		<div class="results">
			<div>{results.length} résultats</div>
			{#each results as result}
				<div on:click={changeSelectedResult(result)} class="result">
					{result[0]}
				</div>
			{/each}
		</div>
	{/if}

	<div class="side-peek">
		{#if highlightedResult}
			<div class="side-content">
				<img
					class="icon back"
					on:click={() => (highlightedResult = null)}
					src={backImg}
					alt=""
					title="Close panel"
				/>
				<GifExporter origin={textSnapshot} destination={highlightedResult}></GifExporter>
			</div>
		{/if}
	</div>

	<div class="encore" class:visible={displayEncore} on:click={encore}>
		Encore !
		<div bind:this={encoreTooltip}> <span> ? </span> </div>
	</div>
</main>

<style lang="scss">
	.logo {
		margin: auto 10px auto 0;
		order: 1;
	}
	.back {
		margin: 5px;
		width: 32px;
		height: 32px;
		display: none;
	}

	.icon {
		cursor: pointer;
	}
	.input {
		order: 2;
		display: flex;
		flex-direction: column;
		max-width: 25rem;
		margin: auto 1rem auto 5rem;
		align-items: center;
	}

	.error-message {
		color: #9b2318;
		margin: 0;
		width: max-content;
		display: block;
		&.include {
			max-width: 10rem;
		}
	}
	.search {
		margin: 0;
		height: 45px;
	}

	.error .search {
		border-color: #9b2318;
	}
	.results {
		margin: 0 0 0 17rem;
		padding-top: 1rem;
		& > div {
			padding: 5px;
		}
	}

	.result {
		font-size: 30px;
		cursor: pointer;
		width: max-content;
	}

	.side-peek {
		position: fixed;
		top: 0px;
		right: 0px;
		bottom: 0px;
		max-width: 100vw;
		width: 0px;

		& .side-content {
			margin: 90px 1rem 1rem 1rem;
		}
		z-index: 1;
		transition-property: width, transform;
		transition-duration: 200ms;
		transition-delay: 0ms;
		transition-timing-function: ease;
		flex-direction: column;
		display: flex;
		margin-left: auto;
		border-left: 1px solid #91795683;
	}
	.peek-opened {
		& .side-peek {
			width: min(500px, 100vw);
		}
		& .results {
			margin: 0 0 0 10rem;
		}
	}
	.settings-trigger {
		cursor: pointer;
		order: 3;
	}

	.settings {
		font-size: 0.7rem;
		background-color: white;
		& .param {
			display: flex;
		}

		& .title {
			margin: auto;
			width: max-content;
		}

		& .param {
			display: flex;
			align-items: center;
			margin-top: 10px;
		}

		& input[type='text'] {
			font-size: 0.8rem;
			height: 25px;
			margin: 0 0 0 5px;
		}

		& .include-input {
			max-width: 10rem;
			flex: 1 0 60px;
			display: flex;
			flex-direction: column;
			align-items: center;
		}
	}

	.encore {
		z-index: 2;
		position: sticky;
		cursor: pointer;
		bottom: 20px;
		margin-bottom: 20px;
		left: 20px;
		width: max-content;
		background-color: white;
		padding: 20px;
		border-radius: 25px;
		border: 1px solid #917956;
		visibility: hidden;
		&.visible {
			visibility: visible;
		}
		& > div {
			position: absolute;
			top: 0;
			left: -5px;
			background-color: #917956;
			width: 20px;
			height: 20px;
			display: flex;
			align-items: center;
			justify-content: center;
			border-radius: 100%;
			font-size: 0.6rem;
			border: 1px solid #584a35;
		}
	}

	@media screen and (max-width: 400px) {
		.logo {
			display: none;
		}
		.search {
			margin: auto;
		}
	}
	@media screen and (max-width: 1000px) {
		.results {
			margin: 0 0 0 5rem;
		}
		.back {
			display: block;
		}
		.peek-opened .side-peek {
			background-color: #f1dbbb;
			width: 100vw !important;
		}

		.settings-trigger {
			order: 2;
			margin: 0 10px 0 auto;
		}
		.input {
			order: 3;
			margin-left: 10px;
		}
	}
</style>
