/** @type {import('./$types').PageLoad} */
export function load({ params, url }) {
    console.log(url);
    return { ...Object.fromEntries(url.searchParams) };
}