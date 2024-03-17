<script>
    import * as d3 from 'd3';
    import { loadAnagrams, encode } from '$lib';
    let inputText = '';
    let canvasElement;


    const simpleCharRegex = /[^A-Za-z\- ']/g;
    function toAsciiChars(input) {
        const diacriticsRemoved = input.normalize('NFKD').replace(/[\u0300-\u036f]/g, "");
        const wrongChars = diacriticsRemoved.match(simpleCharRegex)
        if (wrongChars != null) {
            console.log(`Still wrong characters remaining (${wrongChars}). Please update.`);
            return null;
        }
        return diacriticsRemoved;
    }
    
    // const CHAR_WIDTH = 16;
    // function createResultSVG(text) {
    //     const charList = Array.from(text);
    //     const width = charList.length * CHAR_WIDTH;
    //     const svg = d3.create("svg")
    //         .attr("width", width)
    //         .attr("height", 33)
    //         .attr("viewBox", `0 -20 ${width} 33`)
    //         .attr("text", text);
    //     const positions = charList.map((_, i) => i * CHAR_WIDTH);
    //     svg.selectAll("text")
    //         .data(positions)
    //         .join("text")
    //         .attr("x", d => d)
    //         .text((_,i) => charList[i]);
    //     return svg;
    // }

    // /**
    //  * 1. Figure out diacritic / unused characters
    //  * 2. For each source character, get closest unused target position
    //  * 3. Left-to-right: top, right-to-left: bottom
    //  * 
    //  * @param svgNode
    //  * @param targetStr
    //  */
    // function transitionToAnagram(svgNode, targetStr) {
    //     const fromTxt = svgNode.attr('text');
    //     const fromChars = Array.from(toAsciiChars(fromTxt).toLowerCase());
    //     const targetChars = Array.from(toAsciiChars(targetStr).toLowerCase());
    //     const destIndexes = Array.from({ length: fromChars.length });
    //     console.log(`"${fromChars.join('')}" to "${targetChars.join('')}"`);
    //     for (let i = 0; i < fromChars.length; ++i) {
    //         const sourceChar = fromChars[i];
    //         let charFound = false;
    //         for (let j = 0; j < targetChars.length; ++j) {
    //             const targetChar = targetChars[j];
    //             if (sourceChar == targetChar && !destIndexes.includes(j)) {
    //                 destIndexes[i] = j;
    //                 charFound = true;
    //                 break;
    //             }
    //         }
    //         if (!charFound && sourceChar.match(/[a-z]/)) {
    //             console.error(`'${sourceChar}' unmatched. That's a problem.`);
    //             return;
    //         }
    //     }
    //     console.log(destIndexes);
    //     // const newPositions = destIndexes.map(i => i * CHAR_WIDTH);
    //     // console.log(newPositions);
    //     svgNode
    //     .selectAll("text")
    //     .each(function(p, j) {
    //         const dest = destIndexes[j];
    //         console.log(p, j, dest);
    //         console.log(this);
    //         if (dest === undefined) {
    //             return this.remove();
    //         }
    //         d3.select(this)
    //             .transition()
    //             .duration(2000)
    //             .attr("x", dest * CHAR_WIDTH);
    //     });
    // }

    function computeXPositions(letters, letterWidths) {
        let acc = 0;
        return letters.map(c => {
            const x = acc;
            acc += letterWidths[c];
            return x;
        });
    }

    function smoothstep (min, max, value) {
        const x = Math.max(0, Math.min(1, (value-min)/(max-min)));
        return x*x*(3 - 2*x);
    };

    function measureWidths(ctx, letters) {
        const measurements = {};
        letters.forEach(letter => {
            const textMetrics = ctx.measureText(letter);
            measurements[letter] = textMetrics.width;
        });
        return measurements;
    }

    function anagramAnimation(sourceStr, targetStr) {
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
        console.log('letterWidths=', letterWidths);
        const sourceXPositions = computeXPositions(Array.from(sourceStr), letterWidths);
        console.log('sourceXPositions=', sourceXPositions);
        const destXPositions = computeXPositions(Array.from(targetStr), letterWidths);
        canvasElement.width = Math.max(sourceXPositions[sourceXPositions.length - 1] + letterWidths[lastSourceChar], destXPositions[destXPositions.length - 1] + letterWidths[lastDestChar]);
        // destIndex, startX, destX, tweenFunc
        const charState = sourceCharArray.map((c,i) => {
           return {
                char: c,
                startX: sourceXPositions[i],
                letterState: "source"
            };
        });
        console.log(`"${fromChars.join('')}" to "${targetChars.join('')}"`);
        for (let i = 0; i < fromChars.length; ++i) {
            const sourceChar = fromChars[i];
            let charFound = false;
            for (let j = 0; j < targetChars.length; ++j) {
                const targetChar = targetChars[j];
                if (sourceChar == targetChar && charState.find(state => state?.destIndex === j) === undefined) {
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
        
        console.log('charState=', charState);
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
                switchLetters(progress, 0.4, state, "target");
                switchLetters(progress, 0.9, state, "source");
                const x = state.startX + (state.destX - state.startX) * p;
                let y = ((-(Math.pow((p * 2) - 1, 2)) + 1) * fontSize/3);
                if (state.startX > state.destX) y = -y;
                y += canvasElement.height / 2;
                ctx.fillText(state.char, x, y);
            }
        }
        animate({
            duration: 5000,
            ctx,
            draw
        });
        // encode({
        //     ctx,
        //     renderFunction: draw
        // });

    }


    function animate({draw, ctx, duration}) {
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
    // function animate({timing, draw, duration}) {

    //     let start = performance.now();

    //     requestAnimationFrame(function animate(time) {
    //         // timeFraction goes from 0 to 1
    //         let timeFraction = (time - start) / duration;
    //         if (timeFraction > 1) timeFraction = 1;

    //         // calculate the current animation state
    //         let progress = timing(timeFraction)

    //         draw(progress); // draw it

    //         if (timeFraction < 1) {
    //             requestAnimationFrame(animate);
    //         }

    //     });
    // }

    async function test() {
        const res = await loadAnagrams(inputText);
        console.log(res);
        const simplified = toAsciiChars(inputText);
        console.log("simplified=", simplified);
        const node = createResultSVG(simplified).node();
        document.body.append(node);
    }

    function testTransition() {
        const source = "Le Marquis de Sade";
        const target = "Démasqua le désir";
        const root = createResultSVG(source);
        document.body.append(root.node());
        transitionToAnagram(root, target);
    }
</script>

<header>
    <h1>AnagraMDR</h1>
    <input bind:value={inputText} type="search" placeholder="Insérez une expression!" />
</header>
<main>
    <button on:click={()=>test()}>TEST!</button>
    <button on:click={()=>testTransition()}>TEST transition!</button>
    <button on:click={()=>anagramAnimation("Le Marquis de Sade", "Démasqua le désir")}>TEST canvas!</button>
    
    <canvas bind:this={canvasElement} height=200></canvas>

</main>