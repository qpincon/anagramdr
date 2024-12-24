<script>
	import { tick } from "svelte";
	import { areStringsAnagrams } from ".";
	import AnagramAnimation from "./AnagramAnimation.svelte";

    export let origin;
    export let destination;

    let isValid = false;
    let animationComponent;

    $: if (origin || destination) {
        console.log('origin, destination=', origin, destination)
        isValid = areStringsAnagrams(origin, destination);
        tick().then(() => {
            if(animationComponent) animationComponent.startAnimation();
        });
    }

</script>


<div>
    {#if !isValid} 
        <span> Problème: L'expression d'origine et de destination ont des letters pas en commun. </span>
    {:else} 
        <div>
            <AnagramAnimation bind:this={animationComponent} sourceText="{origin}" targetText="{destination}"></AnagramAnimation>
        </div>
    {/if}

</div>