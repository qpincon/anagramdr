/** @type {import('./$types').PageLoad} */
export function load({ params, url }) {
    return { ...Object.fromEntries(url.searchParams) };
}