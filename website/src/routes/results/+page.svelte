<script>
	import { onMount, tick } from 'svelte';
	import { loadAnagrams } from '$lib';
	import { goto } from '$app/navigation';

	import AnagramAnimation from '$lib/AnagramAnimation.svelte';
	// import Test from '$lib/VecTest.svelte';
	// import InfiniteLoading from 'svelte-infinite-loading';


	let results = [];
	let highlightedResult;
	let animationComponent;
	// let displayedResults = [];


	/** @type {import('./$types').PageData} */
	export let data;
	let frozenInput;
	
	onMount(async () => {
		frozenInput = {...data};
		await refreshResults(data);
	});

	function goToResults() {
		const params = new URLSearchParams();
		params.set('input', data.input);
		params.set('search_type', 'ROOT');
		goto(`/results?${params.toString()}`); 
	}
	
	async function refreshResults() {
		console.log('data=', data);
		results =  await loadAnagrams(data);
		console.log(results);
	}
	function onSearchKeyDown(e) {
		if (e.key === 'Enter') goToResults();
	}

	async function changeSelectedResult(result) {
		highlightedResult = result;
		await tick();
		animationComponent.startAnimation();
	}
</script>

<main>
	<header class="results-header">
		<h1 class="logo">
			<span class="commissioner-bold">anagra</span><i class="fuzzy-bubbles-bold">mdr</i>
		</h1>
		<input class="search" on:keydown="{onSearchKeyDown}" bind:value={data.input} type="search" placeholder="Insérez une expression!" />
	</header>

	<!-- <Test></Test> -->
	
    
	<div>{results.length} results</div>
	{#if highlightedResult}
		<span>{highlightedResult}</span>
		<AnagramAnimation bind:this={animationComponent} sourceText="{frozenInput.input}" targetText="{highlightedResult}"></AnagramAnimation>
	{/if}
    <div class="results">
		{#each results as result}
			<div on:click={async () => {await changeSelectedResult(result)}} class="result"> { result } </div>
		{/each}
    </div>

</main>

<style lang="scss">
	header {
		padding: 10px;
		background-color: rgba(255, 255, 255, 0.199);
		border-bottom: 1px solid rgba(145, 121, 86, 0.514);
	}
	.results-header{
		display: flex;
		justify-content: center;
		align-items: center;
	}

	.search {
		margin-right: auto;
		margin-left: 3rem;
		max-width: 25rem;
		height: 45px;
	}
	.logo {
		width: 11rem;
	}
	.results {
		margin: 0 15rem ;
		padding-top: 1rem;
	}
	.result {
		font-size: 30px;
		padding: 5px;
	}
</style>