<script>
	import { onMount, tick } from 'svelte';
	import { encodeToGif } from './index';
	// import * as flubber from 'flubber';
	import opentype from 'opentype.js';
	// import { flattenSVG } from 'flatten-svg';

	export let sourceText;
	export let targetText;
	export let textColor = 'black';
	export let animationDurationMs = 5000;
	
	let animationId = null;
	let scale = 1;
	onMount(() => {
		startAnimation();
	});

	let canvasElement;
	let canvasElementForExport;
	const urlRegex = /url\(.*?\)/g;
	export function startAnimation(forExport = false) {

		if (!forExport && animationId) {
			cancelAnimationFrame(animationId);
			animationId = null;
		}
		animateAnagram(sourceText, targetText, !forExport, forExport);
		// getLoadedFontsUrls();
	}

	async function getMaxAvailableSpace() {
		if (!canvasElement) return 0;
		const element = canvasElement.parentElement;
		const computedStyle = getComputedStyle(element);

		let elementWidth = element.clientWidth;   // width with padding

		if (element.clientWidth < 100) {
			await new Promise((res, _) => {
				setTimeout(() => res(), 200)
			})
			return getMaxAvailableSpace();
		}
		elementWidth -= parseFloat(computedStyle.paddingLeft) + parseFloat(computedStyle.paddingRight);

		return elementWidth;
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

	function toAsciiChars(input) {
		const diacriticsRemoved = input.normalize('NFKD').replace(/[\u0300-\u036f]/g, '');
		return diacriticsRemoved;
	}

	function computeXPositions(letters, letterWidths) {
		let acc = 10;
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

	async function animateAnagram(sourceStr, targetStr, loop = true, exportToGif = false) {
		// const font = await prepareFont();
		const sourceCharArray = Array.from(sourceStr);
		const destCharArray = Array.from(targetStr);
		const fromChars = Array.from(toAsciiChars(sourceStr).toLowerCase());
		const targetChars = Array.from(toAsciiChars(targetStr).toLowerCase());

		const lastSourceChar = sourceCharArray[sourceCharArray.length - 1];
		const lastDestChar = destCharArray[destCharArray.length - 1];
		let usedCanvas = exportToGif ? canvasElementForExport : canvasElement;
		if (!usedCanvas) return;
		const ctx = usedCanvas.getContext('2d',  { willReadFrequently: true });
		const fontSize = 50;
		ctx.textBaseline = 'middle';

		ctx.font = `${fontSize}px serif`;
		const allLetters = [...new Set([...Array.from(sourceStr), ...Array.from(targetStr)])];
		const letterWidths = measureWidths(ctx, allLetters);
		// console.log('letterWidths=', letterWidths);
		const sourceXPositions = computeXPositions(Array.from(sourceStr), letterWidths);
		// console.log('sourceXPositions=', sourceXPositions);
		const destXPositions = computeXPositions(Array.from(targetStr), letterWidths);
		const maxWidth = await getMaxAvailableSpace();
		usedCanvas.height = 100;
		usedCanvas.width = Math.max(
			sourceXPositions[sourceXPositions.length - 1] + letterWidths[lastSourceChar],
			destXPositions[destXPositions.length - 1] + letterWidths[lastDestChar]
		);



		if (usedCanvas.width > maxWidth) {
			scale = maxWidth/usedCanvas.width
		} else {
			scale = 1;
		}
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
			if (!charFound) {
				if (sourceChar.match(/[a-z]/)) {
					console.error(`'${sourceChar}' unmatched. That's a problem.`);
					return;
				} else {
					charState[i].destChar = sourceChar;
					charState[i].destX = charState[i].startX;
					charState[i].destY = -200;
				}
			}
		}

		// console.log('charState=', JSON.parse(JSON.stringify(charState)));
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
			ctx.clearRect(0, 0, usedCanvas.width, usedCanvas.height);
			ctx.fillStyle = textColor;
			for (const state of charState) {
				const p = timingTransform(progress);
				switchLetters(progress, 0.4, state, 'target');
				switchLetters(progress, 0.9, state, 'source');
				const x = state.startX + (state.destX - state.startX) * p;
				let y;
				const baseY = (usedCanvas.height / 2) + 15;
				if (state.destY) {
					y = baseY + (state.destY * p);
				} else {
					if (Math.abs(state.startX - state.destX) < 30) y = 0;
					else {
						y = ((-Math.pow(p * 2 - 1, 2) + 1) * fontSize) / 3;
						if (state.startX > state.destX) y = -y;
					}
					y += baseY;
				}
				ctx.fillText(state.char, x, y);
			}
		}
		
		
		if (exportToGif) {
			const url = await encodeToGif({
				ctx,
				duration: animationDurationMs / 1000,
				renderFunction: draw
			});
			window.open(url);
		} else {
			await animate({
				duration: animationDurationMs,
				ctx,
				draw
			});
			if (loop) {
				if (animationId) {
					cancelAnimationFrame(animationId);
					animationId = null;
				}
				animateAnagram(sourceStr, targetStr, loop);
			}
		}

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
		return new Promise((resolve, reject) => {
			let start = performance.now();

			animationId = requestAnimationFrame(function animate(time) {
				// timeFraction goes from 0 to 1
				let timeFraction = (time - start) / duration;
				if (timeFraction > 1) timeFraction = 1;

				draw(ctx, timeFraction); // draw it

				if (timeFraction < 1) {
					animationId = requestAnimationFrame(animate);
				} else resolve();
			});
		});
	}

	let testSvg;
	let testSvg2;
	let flubberSvg;
</script>

<canvas bind:this={canvasElement} 
style="
  transform: scale({scale}); 
  transform-origin: left;
  margin: auto;
  display: flex;
"></canvas>
<canvas bind:this={canvasElementForExport} 
style="
  position: absolute; 
  top: -200px;
"></canvas>
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
