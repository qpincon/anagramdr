<script>
		import { goto } from '$app/navigation';

	let inputText = '';

	function goToResults() {
		const params = new URLSearchParams();
		params.set('input', inputText);
		params.set('search_type', 'ROOT');
		goto(`/results?${params.toString()}`); 
	}

	function onSearchKeyDown(e) {
		if (e.key === 'Enter') goToResults();
	}

</script>

<header>
	<h1>
		<span class="commissioner-bold">anagra</span><i class="fuzzy-bubbles-bold">mdr</i>
	</h1>
	<input bind:value={inputText} type="search" placeholder="Insérez une expression!" />
	<button on:click={() => goToResults()} on:keydown="{onSearchKeyDown}">Search</button>
</header>
