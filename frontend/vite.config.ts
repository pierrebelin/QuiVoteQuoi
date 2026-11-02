import { defineConfig, loadEnv } from 'vite'
import react from '@vitejs/plugin-react'
import tailwindcss from '@tailwindcss/vite'

// VITE_API_TARGET permet de pointer le SPA vers un backend lancé sur un autre
// port, sans toucher à cette configuration.
export default defineConfig(({ mode }) => {
  const env = loadEnv(mode, process.cwd(), '')
  return {
    plugins: [react(), tailwindcss()],
    server: {
      proxy: {
        // changeOrigin : un backend distant derrière un hôte virtuel attend
        // son propre nom dans l'en-tête Host, pas « localhost ».
        '/api': { target: env.VITE_API_TARGET || 'http://localhost:3000', changeOrigin: true },
      },
    },
  }
})
