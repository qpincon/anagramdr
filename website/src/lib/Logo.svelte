<script>
	export let fontSize = '2em';
</script>

<a href="/">
	<h1 class="logo" style="font-size: {fontSize}">
		<span class="dynapuff">anagra</span><i class="fuzzy-bubbles-bold mdr"
			><span>m</span><span>d</span><span>r</span></i
		>
	</h1>
</a>

<style>
	a {
		text-decoration: none !important;
		color: #7e7e7e;
	}
	h1 {
		margin: 0;
	}

	.mdr {
		font-size: 1em;
		& > span:first-child {
			color: #7e2e00;
		}
		& > span:nth-child(2) {
			color: #006b5f;
		}
		& > span:nth-child(3) {
			color: #63baab;
		}
	}

</style>
