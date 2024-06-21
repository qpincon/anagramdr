import { sveltekit } from '@sveltejs/kit/vite';
import { defineConfig } from 'vite';
import { viteCommonjs } from '@originjs/vite-plugin-commonjs'
export default defineConfig({
    plugins: [viteCommonjs(), sveltekit()],
    server: {
        proxy: {
            '/query': 'http://localhost:3030',
        },
    },
    define: {
        // By default, Vite doesn't include shims for NodeJS/
        // necessary for segment analytics lib to work
        global: {},
      },
});
