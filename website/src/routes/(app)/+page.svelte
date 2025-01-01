<script>
	import { goto } from '$app/navigation';
	import Logo from '$lib/Logo.svelte';
	let inputText = '';

	function goToResults() {
		if (!inputText.length) return;
		const params = new URLSearchParams();
		params.set('input', inputText);
		params.set('search_type', 'ROOT');
		goto(`/resultats?${params.toString()}`); 
	}

	function onSearchKeyUp(e) {
		if (e.key === 'Enter') goToResults();
	}

</script>
<div class="container">
	<div class="logo">
		<Logo></Logo>
		<h2 class="subtitle"> <i> Anagrammise ce qu'il te plaît ! </i> </h2>
	</div>
	<input bind:value={inputText} type="search" on:keyup={onSearchKeyUp} placeholder="Ecrivez n'importe quoi" />
	<button on:click={() => goToResults()} >Chercher!</button>
</div>

<style lang="scss">
	.container {
		margin: auto;
		margin-top: 6rem;
		display: flex;
		flex-direction: column;
		align-items: center;
	}
	.subtitle {
		font-weight: normal;
	}

	.logo {
		margin: 0 auto;
		width: min-content;
		font-size: 2em;
		display: flex;
		flex-direction: column;
		align-items: center	;
		& .subtitle {
			font-size: 1rem;
		}
	}
	input {
		max-width: 30rem;
		height: 50px;
		margin-top: 2rem;
	}
	button {
		width: 7rem;
		padding: 5px;
		border-color: #006b5f;
		background-color: #63baab;
	}

	@media screen and (max-width: 400px) {
		.logo {
			font-size: 1.5em;
		}
	}
</style>