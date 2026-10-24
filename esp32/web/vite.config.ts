import { defineConfig } from 'vite'
import preact from '@preact/preset-vite'
import { homeUi } from '@mqtt-home/ui/vite'

export default defineConfig({
  // homeUi(): the prepaint script (dark mode, Wall Display detection) first in <head>
  plugins: [preact(), homeUi()],
  test: {
    environment: 'jsdom',
    globals: true,
  },
  build: {
    outDir: '../data',
    emptyOutDir: false,
    minify: 'terser',
    terserOptions: {
      compress: {
        drop_console: true,
        drop_debugger: true
      }
    },
    rollupOptions: {
      output: {
        manualChunks: undefined,
        entryFileNames: 'app.js',
        assetFileNames: (assetInfo) => {
          if (assetInfo.name?.endsWith('.css')) {
            return 'style.css'
          }
          return '[name][extname]'
        }
      }
    },
    assetsInlineLimit: 10240
  }
})
