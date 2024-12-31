import { sveltekit } from '@sveltejs/kit/vite';
import { defineConfig } from 'vite';
import { viteCommonjs } from '@originjs/vite-plugin-commonjs'
export default defineConfig({
    plugins: [viteCommonjs(), sveltekit()],
    server: {
        proxy: {
            '/engine': {
                target: 'http://localhost:3030',
                changeOrigin: true,
            },
        },
    },
    define: {
        global: {},
    },
});
