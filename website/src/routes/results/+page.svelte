<script>
	import { onMount, tick } from 'svelte';
	import { loadAnagrams } from '$lib';
	import { goto } from '$app/navigation';
	import backImg from '$lib/img/back.svg';

	import GifExporter from '../../lib/GifExporter.svelte';
	// import Test from '$lib/VecTest.svelte';
	// import InfiniteLoading from 'svelte-infinite-loading';

	let results = [];
	let highlightedResult = null;

	export let data;
	let frozenInput;

	onMount(async () => {
		frozenInput = { ...data };
		await refreshResults(data);
	});

	function goToResults() {
		const params = new URLSearchParams();
		params.set('input', data.input);
		params.set('search_type', 'ROOT');
		goto(`/results?${params.toString()}`);
		refreshResults();
	}

	async function refreshResults() {
		highlightedResult = null;
		console.log('data=', data);
		results = await loadAnagrams(data);
		console.log(results);
	}

	function onSearchKeyUp(e) {
		if (e.key === 'Enter') goToResults();
	}

	async function changeSelectedResult(result) {
		highlightedResult = result[0];
		await tick();
	}
</script>

<main>
	<header >
		<a href="/">
			<h1 class="logo">
				<span class="dynapuff">anagra</span><i class="fuzzy-bubbles-bold mdr"
					><span>m</span><span>d</span><span>r</span></i
				>
			</h1>
		</a>
		<input
			class="search"
			on:keydown={onSearchKeyUp}
			bind:value={data.input}
			type="search"
			placeholder="Insérez une expression!"
		/>
	</header>

	<div class="results">
		<div>{results.length} results</div>
		{#each results as result}
			<div
				on:click={async () => {
					await changeSelectedResult(result);
				}}
				class="result"
			>
				{result[0]}
			</div>
		{/each}
	</div>

	<div class="side-peek" class:opened={highlightedResult !== null}>
		{#if highlightedResult}
			<div class="side-content">
				<img
					class="icon back"
					on:click={() => (highlightedResult = null)}
					src={backImg}
					alt=""
					title="Close panel"
				/>
				<GifExporter origin={frozenInput.input} destination={highlightedResult}></GifExporter>
			</div>
		{/if}
	</div>
</main>

<style lang="scss">
	header {
		padding: 10px;
		background-color: #f1dbbb;
		border-bottom: 1px solid rgba(145, 121, 86, 0.514);
		position: relative;
		z-index: 10;
		display: flex;
		justify-content: flex-start;
		align-items: center;
		& a {
			text-decoration: none;
		}
	}
	.logo {
		margin: auto 0 auto 0;
	}
	.back {
		margin: 5px;
		display: none;
	}

	.icon {
		width: 16px;
		height: 16px;
		cursor: pointer;
	}

	.search {
		max-width: 25rem;
		height: 45px;
		margin: auto auto auto 5rem;
	}

	.results {
		margin: 0 0 0 17rem;
		padding-top: 1rem;
	}

	.result {
		font-size: 30px;
		padding: 5px;
		cursor: pointer;
	}

	.side-peek {
		position: fixed;
		top: 0px;
		right: 0px;
		bottom: 0px;
		max-width: 100vw;
		width: 0px;
		&.opened {
			width: min(500px, 100vw);
		}
		& .side-content {
			margin-top: 90px;
		}
		z-index: 1;
		transition-property: width, transform;
		transition-duration: 200ms;
		transition-delay: 0ms;
		transition-timing-function: ease;
		flex-direction: column;
		display: flex;
		margin-left: auto;
		border-left: 1px solid grey;
	}

	@media screen and (max-width: 400px) {
		.logo {
			display: none;
		}
	}
	@media screen and (max-width: 1000px) {
		.results {
			margin: 0 0 0 5rem;
		}
		.back {
			display: block;
		}
		.side-peek.opened {
			background-color: #f1dbbb;
			width: 100vw !important;
		}
	}
</style>
