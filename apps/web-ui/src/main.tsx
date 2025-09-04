import { StrictMode } from 'react'
import { createRoot } from 'react-dom/client'
import App from './App.tsx'
import './index.css'
import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import { RealtimeProvider } from './realtime/useRealtime'
import { ApiKeyProvider } from './state/ApiKeyContext'

const root = document.getElementById('root')

const queryClient = new QueryClient({
  defaultOptions: {
    queries: {
      staleTime: 0,
      refetchOnWindowFocus: true,
      retry: 2
    }
  }
})

// biome-ignore lint/style/noNonNullAssertion: We know there is a root element.
createRoot(root!).render(
  <StrictMode>
    <QueryClientProvider client={queryClient}>
      <ApiKeyProvider>
        <RealtimeProvider>
          <App />
        </RealtimeProvider>
      </ApiKeyProvider>
    </QueryClientProvider>
  </StrictMode>
)
