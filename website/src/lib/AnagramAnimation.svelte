<script>
	import { onMount, tick } from 'svelte';
	import { encodeToGif } from './index';
	import * as flubber from 'flubber';
	import opentype from 'opentype.js';
	import { flattenSVG } from 'flatten-svg';

	export let sourceText;
	export let targetText;

	onMount(() => {
		startAnimation();
	});

	let canvasElement;

	const urlRegex = /url\(.*?\)/g;
	export function startAnimation() {
		console.log(sourceText, targetText);
		animateAnagram(sourceText, targetText);
		// getLoadedFontsUrls();
	}

	function getLoadedFontsUrls() {
		const fonts = [];
		for (let i = 0; i < document.styleSheets[0].cssRules.length; i++) {
			const urls = document.styleSheets[0].cssRules[i].cssText.match(urlRegex);
			if (urls) {
				for (let j = 0; j < urls.length; j++) {
					fonts.push(urls[j]);
				}
			}
		}
		return fonts;
	}

	const simpleCharRegex = /[^A-Za-z\- ']/g;
	function toAsciiChars(input) {
		const diacriticsRemoved = input.normalize('NFKD').replace(/[\u0300-\u036f]/g, '');
		const wrongChars = diacriticsRemoved.match(simpleCharRegex);
		if (wrongChars != null) {
			console.log(`Still wrong characters remaining (${wrongChars}). Please update.`);
			return null;
		}
		return diacriticsRemoved;
	}

	function computeXPositions(letters, letterWidths) {
		let acc = 0;
		return letters.map((c) => {
			const x = acc;
			acc += letterWidths[c];
			return x;
		});
	}

	function smoothstep(min, max, value) {
		const x = Math.max(0, Math.min(1, (value - min) / (max - min)));
		return x * x * (3 - 2 * x);
	}

	function measureWidths(ctx, letters) {
		const measurements = {};
		letters.forEach((letter) => {
			const textMetrics = ctx.measureText(letter);
			measurements[letter] = textMetrics.width;
		});
		return measurements;
	}

	async function prepareFont(
		fontUrl = 'https://fonts.gstatic.com/s/fuzzybubbles/v7/6qLbKZMbrgv9pwtjPEVNV0F2Ds_WcxQKZw.woff2'
		// fontUrl = 'https://fonts.gstatic.com/s/commissioner/v20/tDbw2o2WnlgI0FNDgduEk4jAhwgumbU1SVfU5BD8OuRL8OstC6KOhgvBYWSFJ-Mgdrgiju6fF8m0akXa.woff2'
	) {
		const req = await fetch(fontUrl);
		const buffer = await req.arrayBuffer();
		const decompressed = Module.decompress(buffer);
		return opentype.parse(Uint8Array.from(decompressed).buffer);
	}

	async function animateAnagram(sourceStr, targetStr) {
		const font = await prepareFont();
		const sourceCharArray = Array.from(sourceStr);
		const destCharArray = Array.from(targetStr);
		const fromChars = Array.from(toAsciiChars(sourceStr).toLowerCase());
		const targetChars = Array.from(toAsciiChars(targetStr).toLowerCase());
		const lastSourceChar = sourceCharArray[sourceCharArray.length - 1];
		const lastDestChar = destCharArray[destCharArray.length - 1];
		const ctx = canvasElement.getContext('2d');
		const fontSize = 50;
		ctx.font = `${fontSize}px serif`;
		const allLetters = [...new Set([...Array.from(sourceStr), ...Array.from(targetStr)])];
		const letterWidths = measureWidths(ctx, allLetters);
		// console.log('letterWidths=', letterWidths);
		const sourceXPositions = computeXPositions(Array.from(sourceStr), letterWidths);
		// console.log('sourceXPositions=', sourceXPositions);
		const destXPositions = computeXPositions(Array.from(targetStr), letterWidths);
		canvasElement.height = 200;
		canvasElement.width = Math.max(
			sourceXPositions[sourceXPositions.length - 1] + letterWidths[lastSourceChar],
			destXPositions[destXPositions.length - 1] + letterWidths[lastDestChar]
		);
		// destIndex, startX, destX, tweenFunc
		const charState = sourceCharArray.map((c, i) => {
			return {
				char: c,
				startX: sourceXPositions[i],
				letterState: 'source'
			};
		});
		// console.log(`"${fromChars.join('')}" to "${targetChars.join('')}"`);
		for (let i = 0; i < fromChars.length; ++i) {
			const sourceChar = fromChars[i];
			let charFound = false;
			for (let j = 0; j < targetChars.length; ++j) {
				const targetChar = targetChars[j];
				if (
					sourceChar == targetChar &&
					charState.find((state) => state?.destIndex === j) === undefined
				) {
					charState[i].destIndex = j;
					charState[i].destX = destXPositions[j];
					charState[i].destChar = destCharArray[j];
					charFound = true;
					break;
				}
			}
			if (!charFound && sourceChar.match(/[a-z]/)) {
				console.error(`'${sourceChar}' unmatched. That's a problem.`);
				return;
			}
		}

		// console.log('charState=', charState);
		ctx.fillStyle = 'blue';
		ctx.font = `${fontSize}px serif`;
		function timingTransform(progress) {
			const t1 = smoothstep(0.1, 0.4, progress);
			const t2 = 1 - smoothstep(0.6, 0.9, progress);
			return Math.min(t1, t2);
		}

		function switchLetters(progress, progressThresh, charState, targetState) {
			if (progress > progressThresh && charState.letterState !== targetState) {
				charState.letterState = targetState;
				const tmp = charState.char;
				charState.char = charState.destChar;
				charState.destChar = tmp;
			}
		}

		function draw(ctx, progress) {
			ctx.clearRect(0, 0, canvasElement.width, canvasElement.height);
			for (const state of charState) {
				const p = timingTransform(progress);
				switchLetters(progress, 0.4, state, 'target');
				switchLetters(progress, 0.9, state, 'source');
				const x = state.startX + (state.destX - state.startX) * p;
				let y;
				if (Math.abs(state.startX - state.destX) < 30) y = 0;
				else {
					y = ((-Math.pow(p * 2 - 1, 2) + 1) * fontSize) / 3;
					if (state.startX > state.destX) y = -y;
				}
				y += canvasElement.height / 2;
				ctx.fillText(state.char, x, y);
			}
		}
		animate({
			duration: 5000,
			ctx,
			draw
		});
		// encodeToGif({
		//     ctx,
		//     renderFunction: draw
		// });

		// const path1 = font.getPath('M', 0, 100, 72);
		// const pathData1 = path1.toPathData();
		// const path2 = font.getPath('m', 0, 100, 72);
		// const pathData2 = path2.toPathData();
		// // const pathData = paths.toSVG({optimize: false, decimalPlaces: 50});
		// // console.log(pathData1, pathData2);
		// testSvg.setAttribute('d', pathData1);
		// testSvg2.setAttribute('d', pathData2);
		// const flattened1 = flattenSVG(testSvg.parentElement);
		// const flattened2 = flattenSVG(testSvg2.parentElement);
		// console.log(flattened1[0].points, flattened2[0].points);

		// var interpolator = flubber.interpolate(flattened1[0].points, flattened2[0].points);

		// function draw(ctx, progress) {
		// 	flubberSvg.setAttribute('d', interpolator(progress));
		// 	// if (progress < 1) {
		// 	// 	requestAnimationFrame(draw);
		// 	// }
		// }

		// animate({
		// 	duration: 1000,
		// 	ctx,
		// 	draw
		// });

		// requestAnimationFrame(draw);
	}

	function animate({ draw, ctx, duration }) {
		let start = performance.now();

		requestAnimationFrame(function animate(time) {
			// timeFraction goes from 0 to 1
			let timeFraction = (time - start) / duration;
			if (timeFraction > 1) timeFraction = 1;

			draw(ctx, timeFraction); // draw it

			if (timeFraction < 1) {
				requestAnimationFrame(animate);
			}
		});
	}

	let testSvg;
	let testSvg2;
	let flubberSvg;
</script>

<canvas bind:this={canvasElement}></canvas>
<!-- <svg xmlns="http://www.w3.org/2000/svg" width="600" height="300">
	<path bind:this={testSvg}> </path>
</svg>
<svg xmlns="http://www.w3.org/2000/svg" width="600" height="300">
	<path bind:this={testSvg2}> </path>
</svg>
<div style="width:500px; height: 500px;">
	<svg xmlns="http://www.w3.org/2000/svg" fill="black" width="600" height="300">
		<path bind:this={flubberSvg}> </path>
	</svg>
</div> -->
<!-- <svg xmlns="http://www.w3.org/2000/svg" width="600" height="300" >
	<text y=30 class="commissioner-bold"> îö</text>
</svg> -->
<!-- <svg xmlns="http://www.w3.org/2000/svg" width="600" height="300" >
	<text font-size=70 y=60 class="fuzzy-bubbles-bold"> îö</text>
</svg> -->
