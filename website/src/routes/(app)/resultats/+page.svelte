<script>
	import { onMount } from 'svelte';
	import { loadAnagrams, MAX_NB_LETTERS } from '$lib';
	import { goto } from '$app/navigation';
	import backImg from '$lib/img/back.svg';
	import { navigating } from '$app/stores';
	import Logo from '$lib/Logo.svelte';
	import GifExporter from '$lib/GifExporter.svelte';
	import { sortedStringNormalized } from '$lib';
	import settingsIcon from '$lib/img/settings.svg';
	import upArrowIcon from '$lib/img/up.svg';
	import spinner from '$lib/img/puff.svg';
	import tippy from 'tippy.js';
	import { debounce } from 'lodash-es';

	let results = [];
	let highlightedResult = null;

	let textSnapshot;
	let textField;
	let backError = null;
	let validationError = null;
	let settingsImgElement;
	let settingsContentElement;
	let toIncludeInput = '';
	let toIncludeError = null;
	let isSearchExact = false;
	let encoreTooltip;
	let displayEncore = false;
	let displayBackToTop = false;
	let loading = false;

	let settingsTooltipHandle;

	$: searchType = isSearchExact ? 'EXACT' : 'ROOT';

	$: if ($navigating) {
		const dest = $navigating.to;
		if (dest.route.id.includes('resultats')) {
			const params = Object.fromEntries(new URL(dest.url).searchParams);
			loadPageDeb(params);
		}
	}

	const loadPageDeb = debounce(loadPage, 200, { leading: true, trailing: false });

	onMount(() => {
		const params = Object.fromEntries(new URLSearchParams(window.location.search));
		loadPageDeb(params);
		settingsContentElement.style.display = 'block';
		settingsTooltipHandle = tippy(settingsImgElement, {
			content: settingsContentElement,
			theme: 'light',
			trigger: 'click',
			interactive: true
		});

		tippy(encoreTooltip, {
			content:
				"<span> Beaucoup d'anagrammes peuvent sortir d'une expression donnée, donc de l'aléatoire est utilisé pour les choisir.</span> <br/>  <span>Résultat : chaque recherche, c'est la surprise du chef !</span>",
			theme: 'light',
			allowHTML: true
		});
	});

	function loadPage(inputObject) {
		if (!inputObject.input) goto(`/`);
		const searchExact = inputObject.search_type === 'ROOT' ? false : true;
		isSearchExact = searchExact;
		textSnapshot = textField = inputObject.input;
		if (inputObject.word_to_include) {
			toIncludeInput = inputObject.word_to_include;
		}
		return refreshResults(inputObject);
	}

	function goToResults() {
		const params = new URLSearchParams();
		textSnapshot = textField;
		params.set('input', textSnapshot);
		params.set('search_type', searchType);
		if (toIncludeInput.length) params.set('word_to_include', toIncludeInput);
		goto(`/resultats?${params.toString()}`);
	}

	async function refreshResults() {
		highlightedResult = null;
		loading = true;
		results = [];
		if (settingsTooltipHandle) settingsTooltipHandle.hide();
		const res = await loadAnagrams({
			input: textSnapshot,
			searchType,
			wordToInclude: toIncludeInput
		});
		loading = false;
		if (res.code) {
			backError = res.message;
			displayEncore = false;
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
		toIncludeInput = '';
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
		window.scrollTo(0, 0);
		refreshResults();
	}

	function backToTop() {
		window.scrollTo({ top: 0, behavior: 'smooth' });
	}

	function onScroll(e) {
		displayBackToTop = window.scrollY > 1000;
	}
</script>

<svelte:window on:keyup={onSearchKeyUp} on:scroll={onScroll} />
<main class:peek-opened={highlightedResult !== null} class:error={validationError}>
	<header>
		<div class="logo">
			<Logo></Logo>
		</div>
		<div class="input">
			<input class="search" on:input={validateInput} bind:value={textField} type="search" />
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
	{:else if loading}
		<div class="spinner">
			<img src={spinner} />
		</div>
	{:else}
		<div class="results">
			{#if results.length}
				<div>{results.length} résultats</div>
				{#each results as result}
					<div on:click={changeSelectedResult(result)} class="result">
						{result[0]}
					</div>
				{/each}
			{:else}
				<div>Aucun résultat trouvé pour ces lettres... <br /> Essayez une autre expression !</div>
			{/if}
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

	<div class="bottom-left">
		{#if displayBackToTop}
			<div class="back-to-top" on:click={backToTop}>
				<img src={upArrowIcon} />
			</div>
		{/if}
		<div class="encore" class:visible={displayEncore} on:click={encore}>
			Encore !
			<div bind:this={encoreTooltip}><span> ? </span></div>
		</div>
	</div>
</main>

<style lang="scss">
	.logo {
		margin: auto 10px auto 0;
		order: 1;
	}
	.spinner {
		margin: 50px auto;
		width: fit-content;
	}
	.back {
		margin: 5px;
		width: 32px;
		height: 32px;
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

	.bottom-left {
		z-index: 2;
		position: sticky;
		bottom: 20px;
		left: 20px;
		margin-bottom: 20px;
		width: fit-content;
		display: flex;
		flex-direction: column;
		align-items: center;
	}

	.back-to-top {
		margin-bottom: 10px;
		cursor: pointer;
		left: 50px;
		background-color: white;
		border-radius: 25px;
		border: 1px solid #917956;
		width: fit-content;
		& > img {
			padding: 10px;
		}
	}

	.encore {
		position: relative;
		cursor: pointer;
		width: max-content;
		background-color: white;
		padding: 20px;
		border-radius: 25px;
		border: 1px solid #917956;
		display: none;
		&.visible {
			display: block;
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

		.peek-opened .encore {
			visibility: hidden !important;
		}
	}
	@media screen and (max-width: 1000px) {
		.results {
			margin: 0 0 0 5rem;
		}
		.peek-opened .side-peek {
			background-color: #f1dbbb;
			width: 100vw !important;
		}

		.settings {
			font-size: 1rem;
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
