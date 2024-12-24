<script>
	import { goto } from '$app/navigation';

	let inputText = '';

	function goToResults() {
		const params = new URLSearchParams();
		params.set('input', inputText);
		params.set('search_type', 'ROOT');
		goto(`/results?${params.toString()}`); 
	}

	function onSearchKeyUp(e) {
		if (e.key === 'Enter') goToResults();
	}

</script>

<header>
	<h1 class="logo">
		<span class="dynapuff">anagra</span><i class="fuzzy-bubbles-bold mdr"
			><span>m</span><span>d</span><span>r</span></i
		>
	</h1>
	<input bind:value={inputText} type="search" on:keyup={onSearchKeyUp} placeholder="Insérez une expression!" />
	<button on:click={() => goToResults()} >Search</button>
</header>
