<script>
	import { goto } from '$app/navigation';
	import Logo from '$lib/Logo.svelte';
	import { onMount } from 'svelte';
	import { sortedStringNormalized, MAX_NB_LETTERS} from '$lib';
	let inputText = '';
	let validationError = null;


	onMount(() => {
		document.body.style.backgroundColor = '';
	});

	function goToResults() {
		if (!inputText.length) return;
		const params = new URLSearchParams();
		params.set('input', inputText);
		params.set('search_type', 'ROOT');
		goto(`/resultats?${params.toString()}`);
	}

	function onSearchKeyUp(e) {
		if (e.key === 'Enter' && validationError === null) goToResults();
	}

	function validateInput(e) {
		const normalized = sortedStringNormalized(e.target.value);
		if (normalized.length >= MAX_NB_LETTERS) {
			validationError = `Un maximum de ${MAX_NB_LETTERS} lettres est supporté`;
		} else {
			validationError = null;
		}
	}
</script>

<div class="container">
	<div class="logo">
		<Logo></Logo>
		<h2 class="subtitle"><i> Anagrammise ce qu'il te plaît ! </i></h2>
	</div>
	<input
		bind:value={inputText}
		type="search"
		on:keyup={onSearchKeyUp}
		on:input={validateInput}
		placeholder="Ecrivez n'importe quoi"
	/>
	{#if validationError}
		<div class="error-message">{validationError}</div>
	{/if}
	<button on:click={() => goToResults()}>Chercher!</button>
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
		align-items: center;
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

	.error-message {
		color: #9b2318;
		margin-bottom: 1rem;
		width: max-content;
	}

	@media screen and (max-width: 400px) {
		.logo {
			font-size: 1.5em;
		}
	}
</style>
